use std::path::Path;

use crate::error::CommandError;

/// Supported file formats for RAG import
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RagFileFormat {
    Txt,
    Pdf,
    Code,
    Docx,
    Pptx,
}

impl RagFileFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Pdf => "pdf",
            Self::Code => "code",
            Self::Docx => "docx",
            Self::Pptx => "pptx",
        }
    }
}

/// Result of parsing a file into raw text
#[allow(dead_code)]
pub struct ParsedDocument {
    pub file_name: String,
    pub format: RagFileFormat,
    pub text: String,
    pub page_count: u32,
}

/// Detect file format from extension. Returns None for unsupported formats.
pub fn detect_format(path: &Path) -> Option<RagFileFormat> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "txt" => Some(RagFileFormat::Txt),
        "pdf" => Some(RagFileFormat::Pdf),
        "docx" => Some(RagFileFormat::Docx),
        "pptx" => Some(RagFileFormat::Pptx),
        // Code files
        "py" | "rs" | "ts" | "tsx" | "js" | "jsx" | "java" | "c" | "cpp" | "h" | "hpp"
        | "go" | "rb" | "php" | "swift" | "kt" | "cs" | "yaml" | "yml" | "json" | "xml"
        | "html" | "css" | "scss" | "sql" | "md" | "csv" | "toml" | "sh" | "bash" | "ps1"
        | "bat" | "log" | "ini" | "cfg" | "conf" | "env" | "r" | "scala" | "lua" | "dart"
        | "vue" | "svelte" | "zig" | "nim" | "ex" | "exs" | "erl" | "hs" | "ml" | "clj"
        | "proto" | "graphql" | "tf" | "dockerfile" | "makefile" => Some(RagFileFormat::Code),
        _ => None,
    }
}

/// Parse a file into raw text. Must be called from a blocking context (spawn_blocking).
pub fn parse_file(path: &Path) -> Result<ParsedDocument, CommandError> {
    let format = detect_format(path).ok_or_else(|| {
        CommandError::Rag(format!(
            "Unsupported file format: {}",
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown")
        ))
    })?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let text = match format {
        RagFileFormat::Txt | RagFileFormat::Code => parse_text(path)?,
        RagFileFormat::Pdf => parse_pdf(path)?,
        RagFileFormat::Docx => parse_docx(path)?,
        RagFileFormat::Pptx => parse_pptx(path)?,
    };

    if text.trim().is_empty() {
        return Err(CommandError::Rag(format!(
            "Document '{}' ({}) is empty after parsing — no extractable text found",
            file_name,
            format.as_str()
        )));
    }

    // Rough page count estimate: ~3000 chars per page
    let page_count = (text.len() / 3000).max(1) as u32;

    Ok(ParsedDocument {
        file_name,
        format,
        text,
        page_count,
    })
}

/// Parse plain text / code files (UTF-8)
fn parse_text(path: &Path) -> Result<String, CommandError> {
    std::fs::read_to_string(path)
        .map_err(|e| CommandError::Rag(format!("Failed to read file: {e}")))
}

/// Parse PDF files using pdf-extract with lopdf fallback.
///
/// Strategy:
/// 1. Try `pdf_extract::extract_text()` (handles most PDFs well)
/// 2. If empty/failed, use `lopdf` to diagnose why and attempt raw content-stream extraction
/// 3. Both wrapped in `catch_unwind` for panic safety
fn parse_pdf(path: &Path) -> Result<String, CommandError> {
    // Phase 1: Try pdf-extract (high-level, handles fonts + encodings)
    let path_buf = path.to_path_buf();
    let result = std::panic::catch_unwind(|| pdf_extract::extract_text(&path_buf));
    match result {
        Ok(Ok(ref text)) if !text.trim().is_empty() => return Ok(text.clone()),
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "pdf-extract failed, trying lopdf fallback");
        }
        Err(_) => {
            tracing::warn!("pdf-extract panicked, trying lopdf fallback");
        }
        Ok(Ok(_)) => {
            tracing::info!("pdf-extract returned empty text, trying lopdf fallback");
        }
    }

    // Phase 2: Fallback — use lopdf directly for raw text extraction
    let path_buf = path.to_path_buf();
    let fallback = std::panic::catch_unwind(move || pdf_fallback_extract(&path_buf));
    match fallback {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(CommandError::Rag(
            "PDF parsing failed (internal error)".to_string(),
        )),
    }
}

/// Fallback PDF text extraction using lopdf's content stream operators.
///
/// Extracts raw text from TJ/Tj PDF operators. Lower quality than pdf-extract
/// (no font decoding), but handles cases where pdf-extract returns empty.
/// Also produces specific diagnostic error messages for common failure modes.
fn pdf_fallback_extract(path: &Path) -> Result<String, CommandError> {
    let doc = lopdf::Document::load(path)
        .map_err(|e| CommandError::Rag(format!("Cannot read PDF structure: {e}")))?;

    let page_count = doc.get_pages().len();
    if page_count == 0 {
        return Err(CommandError::Rag("PDF contains no pages".to_string()));
    }

    if doc.is_encrypted() {
        return Err(CommandError::Rag(format!(
            "PDF is encrypted/password-protected ({page_count} pages). \
             Please provide an unprotected version."
        )));
    }

    let mut all_text = String::new();
    let mut has_images = false;

    for (page_num, page_id) in doc.get_pages() {
        // Check for image XObjects on this page
        if let Ok((Some(resources), _)) = doc.get_page_resources(page_id) {
            if has_image_xobjects(&doc, resources) {
                has_images = true;
            }
        }

        // Extract text from content stream operators
        if let Ok(content) = doc.get_page_content(page_id) {
            let page_text = extract_text_from_operations(&content);
            if !page_text.is_empty() {
                if !all_text.is_empty() {
                    all_text.push('\n');
                }
                all_text.push_str(&page_text);
            }
        } else {
            tracing::debug!(page = page_num, "Could not read content stream");
        }
    }

    if all_text.trim().is_empty() {
        let reason = if has_images {
            format!(
                "PDF appears to be scanned/image-based ({page_count} pages). \
                 No extractable text found. Please use a PDF with a text layer (OCR)."
            )
        } else {
            format!(
                "No extractable text found in PDF ({page_count} pages). \
                 The file may use unsupported font encodings or contain only graphics."
            )
        };
        return Err(CommandError::Rag(reason));
    }

    tracing::info!(
        chars = all_text.len(),
        pages = page_count,
        "lopdf fallback extracted text"
    );
    Ok(all_text)
}

/// Check if a page's resources contain image XObjects (indicators of scanned content).
fn has_image_xobjects(doc: &lopdf::Document, resources: &lopdf::Dictionary) -> bool {
    let xobject_ref = match resources.get(b"XObject") {
        Ok(v) => v,
        Err(_) => return false,
    };
    let xobjects = match doc.dereference(xobject_ref) {
        Ok((_, obj)) => obj,
        Err(_) => return false,
    };
    let dict = match xobjects.as_dict() {
        Ok(d) => d,
        Err(_) => return false,
    };
    for (_name, value) in dict.iter() {
        if let Ok((_, dereffed)) = doc.dereference(value) {
            if let Ok(stream) = dereffed.as_stream() {
                if let Ok(subtype) = stream.dict.get(b"Subtype") {
                    if let Ok(name) = subtype.as_name() {
                        if name == b"Image" {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

/// Extract text from raw PDF content stream bytes by parsing TJ/Tj operators.
///
/// Best-effort extraction without font decoding — catches text that pdf-extract
/// misses due to encoding issues.
fn extract_text_from_operations(content_bytes: &[u8]) -> String {
    use lopdf::content::Content;

    let content = match Content::decode(content_bytes) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut text = String::new();
    for op in &content.operations {
        match op.operator.as_str() {
            "Tj" => {
                // Single string text operator
                for operand in &op.operands {
                    if let Ok(bytes) = operand.as_str() {
                        push_pdf_bytes(&mut text, bytes);
                    }
                }
            }
            "TJ" => {
                // Array of strings + spacing adjustments
                for operand in &op.operands {
                    if let Ok(arr) = operand.as_array() {
                        for item in arr {
                            if let Ok(bytes) = item.as_str() {
                                push_pdf_bytes(&mut text, bytes);
                            }
                            if let Ok(n) = item.as_float() {
                                if n < -100.0 {
                                    text.push(' ');
                                }
                            }
                        }
                    }
                }
            }
            "Td" | "TD" | "T*" => {
                // Text positioning — likely a new line
                if !text.is_empty() && !text.ends_with('\n') && !text.ends_with(' ') {
                    text.push('\n');
                }
            }
            _ => {}
        }
    }

    // Clean up: collapse whitespace
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Push raw PDF bytes as text, attempting UTF-8 then Latin-1 decoding.
fn push_pdf_bytes(text: &mut String, bytes: &[u8]) {
    match std::str::from_utf8(bytes) {
        Ok(s) => text.push_str(s),
        Err(_) => {
            // Fallback: Latin-1 (ISO 8859-1) — maps bytes 0x00-0xFF to U+0000-U+00FF
            for &b in bytes {
                text.push(b as char);
            }
        }
    }
}

/// Parse DOCX files (ZIP → word/document.xml → extract <w:t> text)
fn parse_docx(path: &Path) -> Result<String, CommandError> {
    let file =
        std::fs::File::open(path).map_err(|e| CommandError::Rag(format!("Cannot open DOCX: {e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| CommandError::Rag(format!("Invalid DOCX archive: {e}")))?;

    let document_xml = archive
        .by_name("word/document.xml")
        .map_err(|e| CommandError::Rag(format!("Missing word/document.xml: {e}")))?;

    extract_xml_text(document_xml, "w:t", "w:p")
}

/// Parse PPTX files (ZIP → ppt/slides/slide{N}.xml → extract <a:t> text)
fn parse_pptx(path: &Path) -> Result<String, CommandError> {
    let file =
        std::fs::File::open(path).map_err(|e| CommandError::Rag(format!("Cannot open PPTX: {e}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| CommandError::Rag(format!("Invalid PPTX archive: {e}")))?;

    // Collect slide file names and sort numerically
    let mut slide_names: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            let name = archive.by_index(i).ok()?.name().to_string();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    slide_names.sort_by(|a, b| {
        let num_a = extract_slide_number(a);
        let num_b = extract_slide_number(b);
        num_a.cmp(&num_b)
    });

    let mut all_text = String::new();
    for slide_name in &slide_names {
        let slide = archive
            .by_name(slide_name)
            .map_err(|e| CommandError::Rag(format!("Cannot read {slide_name}: {e}")))?;
        let slide_text = extract_xml_text(slide, "a:t", "a:p")?;
        if !slide_text.is_empty() {
            if !all_text.is_empty() {
                all_text.push_str("\n\n");
            }
            all_text.push_str(&slide_text);
        }
    }

    Ok(all_text)
}

/// Extract text from XML elements using quick-xml.
/// `text_tag` is the tag containing text (e.g. "w:t" for DOCX, "a:t" for PPTX).
/// `para_tag` is the paragraph boundary tag (e.g. "w:p" for DOCX, "a:p" for PPTX).
fn extract_xml_text<R: std::io::Read>(
    reader: R,
    text_tag: &str,
    para_tag: &str,
) -> Result<String, CommandError> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut xml_reader = Reader::from_reader(std::io::BufReader::new(reader));
    let mut buf = Vec::new();
    let mut output = String::new();
    let mut in_text = false;
    let mut current_para = String::new();

    let text_tag_bytes = text_tag.as_bytes();
    let para_tag_bytes = para_tag.as_bytes();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == text_tag_bytes => {
                in_text = true;
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == text_tag_bytes => {
                in_text = false;
            }
            Ok(Event::Text(ref t)) if in_text => {
                let text = t
                    .unescape()
                    .map_err(|e| CommandError::Rag(format!("XML decode error: {e}")))?;
                current_para.push_str(&text);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == para_tag_bytes => {
                let trimmed = current_para.trim().to_string();
                if !trimmed.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&trimmed);
                }
                current_para.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(CommandError::Rag(format!("XML parse error: {e}")));
            }
            _ => {}
        }
        buf.clear();
    }

    // Flush any remaining text
    let trimmed = current_para.trim().to_string();
    if !trimmed.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&trimmed);
    }

    Ok(output)
}

/// Extract slide number from a path like "ppt/slides/slide12.xml" → 12
fn extract_slide_number(name: &str) -> u32 {
    name.trim_start_matches("ppt/slides/slide")
        .trim_end_matches(".xml")
        .parse()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_txt() {
        assert_eq!(detect_format(Path::new("file.txt")), Some(RagFileFormat::Txt));
    }

    #[test]
    fn test_detect_format_pdf() {
        assert_eq!(detect_format(Path::new("doc.pdf")), Some(RagFileFormat::Pdf));
    }

    #[test]
    fn test_detect_format_docx() {
        assert_eq!(detect_format(Path::new("doc.docx")), Some(RagFileFormat::Docx));
    }

    #[test]
    fn test_detect_format_pptx() {
        assert_eq!(detect_format(Path::new("slides.pptx")), Some(RagFileFormat::Pptx));
    }

    #[test]
    fn test_detect_format_code_various() {
        let code_files = &[
            "main.rs", "app.ts", "index.tsx", "script.py", "Main.java",
            "lib.go", "style.css", "query.sql", "config.yaml", "data.json",
            "page.html", "Makefile.makefile", "schema.proto", "main.zig",
        ];
        for f in code_files {
            assert_eq!(
                detect_format(Path::new(f)),
                Some(RagFileFormat::Code),
                "Expected Code for {f}"
            );
        }
    }

    #[test]
    fn test_detect_format_unknown() {
        assert_eq!(detect_format(Path::new("image.png")), None);
        assert_eq!(detect_format(Path::new("binary.exe")), None);
        assert_eq!(detect_format(Path::new("archive.zip")), None);
    }

    #[test]
    fn test_detect_format_no_extension() {
        assert_eq!(detect_format(Path::new("README")), None);
    }

    #[test]
    fn test_detect_format_case_insensitive() {
        assert_eq!(detect_format(Path::new("DOC.PDF")), Some(RagFileFormat::Pdf));
        assert_eq!(detect_format(Path::new("File.TXT")), Some(RagFileFormat::Txt));
        assert_eq!(detect_format(Path::new("Doc.DOCX")), Some(RagFileFormat::Docx));
    }

    #[test]
    fn test_slide_number_extraction() {
        assert_eq!(extract_slide_number("ppt/slides/slide1.xml"), 1);
        assert_eq!(extract_slide_number("ppt/slides/slide12.xml"), 12);
        assert_eq!(extract_slide_number("ppt/slides/slide100.xml"), 100);
    }

    #[test]
    fn test_format_as_str() {
        assert_eq!(RagFileFormat::Txt.as_str(), "txt");
        assert_eq!(RagFileFormat::Pdf.as_str(), "pdf");
        assert_eq!(RagFileFormat::Code.as_str(), "code");
        assert_eq!(RagFileFormat::Docx.as_str(), "docx");
        assert_eq!(RagFileFormat::Pptx.as_str(), "pptx");
    }
}
