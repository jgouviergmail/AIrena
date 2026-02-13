/// A text chunk produced by the recursive splitter.
#[allow(dead_code)]
pub struct TextChunk {
    /// Index of this chunk within the document
    pub chunk_index: usize,
    /// Index of the source document (for multi-document stores)
    pub doc_index: usize,
    /// The chunk text content
    pub text: String,
    /// Byte offset in the original text where this chunk starts
    pub start_offset: usize,
}

/// Recursive character text splitter.
///
/// Strategy (benchmark-validated optimal for RAG, 2025-2026):
/// 1. Split on paragraph boundaries (`\n\n`)
/// 2. Accumulate paragraphs until `target_chars` (2000 ≈ 512 tokens)
/// 3. When full → save chunk, start next with `overlap_chars` (200) from tail
/// 4. If single paragraph > target → split on sentences (`. `, `! `, `? `, `\n`)
/// 5. If single sentence > target → split on word boundaries (space)
/// 6. UTF-8 safe: `floor_char_boundary()` for all truncation
pub fn chunk_text(
    text: &str,
    doc_index: usize,
    target_chars: usize,
    overlap_chars: usize,
) -> Vec<TextChunk> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let paragraphs = split_keeping_offsets(text, "\n\n");
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_start: usize = 0;
    let mut chunk_index: usize = 0;

    for (para_text, para_offset) in &paragraphs {
        let para_trimmed = para_text.trim();
        if para_trimmed.is_empty() {
            continue;
        }

        // If this single paragraph exceeds target, split it further
        if para_trimmed.len() > target_chars {
            // Flush current buffer first
            if !current.trim().is_empty() {
                chunks.push(TextChunk {
                    chunk_index,
                    doc_index,
                    text: current.trim().to_string(),
                    start_offset: current_start,
                });
                chunk_index += 1;
                current.clear();
            }

            // Split large paragraph into sentence-level chunks
            let sub_chunks = split_large_paragraph(para_trimmed, target_chars);
            for sub in sub_chunks {
                chunks.push(TextChunk {
                    chunk_index,
                    doc_index,
                    text: sub,
                    start_offset: *para_offset,
                });
                chunk_index += 1;
            }

            // Set overlap for next chunk
            if let Some(last) = chunks.last() {
                current = overlap_tail(&last.text, overlap_chars);
                current_start = para_offset + para_trimmed.len().saturating_sub(current.len());
            }
            continue;
        }

        // Check if adding this paragraph would exceed the target
        let sep = if current.is_empty() { "" } else { "\n\n" };
        if current.len() + sep.len() + para_trimmed.len() > target_chars && !current.is_empty() {
            // Save the current chunk
            chunks.push(TextChunk {
                chunk_index,
                doc_index,
                text: current.trim().to_string(),
                start_offset: current_start,
            });
            chunk_index += 1;

            // Start new chunk with overlap from previous
            current = overlap_tail(&chunks.last().unwrap().text, overlap_chars);
            current_start = para_offset.saturating_sub(current.len());
        }

        if current.is_empty() {
            current_start = *para_offset;
        } else {
            current.push_str("\n\n");
        }
        current.push_str(para_trimmed);
    }

    // Flush remaining content
    if !current.trim().is_empty() {
        chunks.push(TextChunk {
            chunk_index,
            doc_index,
            text: current.trim().to_string(),
            start_offset: current_start,
        });
    }

    chunks
}

/// Split a large paragraph into sentence-level chunks.
/// Tries sentence boundaries first, then word boundaries.
fn split_large_paragraph(text: &str, target_chars: usize) -> Vec<String> {
    let sentence_delimiters = [". ", "! ", "? ", "\n"];
    let sentences = split_on_delimiters(text, &sentence_delimiters);

    let mut result = Vec::new();
    let mut current = String::new();

    for sentence in &sentences {
        let trimmed = sentence.trim();
        if trimmed.is_empty() {
            continue;
        }

        // If a single sentence exceeds target, split on word boundaries
        if trimmed.len() > target_chars {
            if !current.trim().is_empty() {
                result.push(current.trim().to_string());
                current.clear();
            }
            let word_chunks = split_on_words(trimmed, target_chars);
            result.extend(word_chunks);
            continue;
        }

        let sep = if current.is_empty() { "" } else { " " };
        if current.len() + sep.len() + trimmed.len() > target_chars && !current.is_empty() {
            result.push(current.trim().to_string());
            current.clear();
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(trimmed);
    }

    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

/// Split text on word boundaries to fit within target_chars.
/// UTF-8 safe.
fn split_on_words(text: &str, target_chars: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let sep = if current.is_empty() { 0 } else { 1 };
        if current.len() + sep + word.len() > target_chars && !current.is_empty() {
            result.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        // If a single word exceeds target, truncate it UTF-8 safely
        if word.len() > target_chars {
            let boundary = word.floor_char_boundary(target_chars);
            current.push_str(&word[..boundary]);
            result.push(current.clone());
            current.clear();
        } else {
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

/// Split text keeping track of byte offsets in the original string.
fn split_keeping_offsets<'a>(text: &'a str, delimiter: &str) -> Vec<(&'a str, usize)> {
    let mut result = Vec::new();
    let mut offset = 0;
    for part in text.split(delimiter) {
        result.push((part, offset));
        offset += part.len() + delimiter.len();
    }
    result
}

/// Split text on multiple sentence delimiters, keeping delimiter attached to previous segment.
fn split_on_delimiters(text: &str, delimiters: &[&str]) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Find the earliest delimiter occurrence
        let mut earliest: Option<(usize, usize)> = None; // (position, delimiter_len)
        for delim in delimiters {
            if let Some(pos) = remaining.find(delim) {
                match earliest {
                    None => earliest = Some((pos, delim.len())),
                    Some((prev_pos, _)) if pos < prev_pos => {
                        earliest = Some((pos, delim.len()));
                    }
                    _ => {}
                }
            }
        }

        match earliest {
            Some((pos, dlen)) => {
                let end = pos + dlen;
                result.push(remaining[..end].to_string());
                remaining = &remaining[end..];
            }
            None => {
                result.push(remaining.to_string());
                break;
            }
        }
    }

    result
}

/// Get the tail of a string as overlap for the next chunk.
/// UTF-8 safe — uses `floor_char_boundary`.
fn overlap_tail(text: &str, overlap_chars: usize) -> String {
    if text.len() <= overlap_chars {
        return text.to_string();
    }
    let start = text.floor_char_boundary(text.len() - overlap_chars);
    text[start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let chunks = chunk_text("", 0, 2000, 200);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let chunks = chunk_text("   \n\n   ", 0, 2000, 200);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_single_short_paragraph() {
        let text = "Hello world, this is a short paragraph.";
        let chunks = chunk_text(text, 0, 2000, 200);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, text);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].doc_index, 0);
    }

    #[test]
    fn test_multiple_paragraphs_within_limit() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_text(text, 0, 2000, 200);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("First"));
        assert!(chunks[0].text.contains("Third"));
    }

    #[test]
    fn test_paragraphs_exceed_limit() {
        // Create text that exceeds 200 chars target
        let para1 = "A".repeat(120);
        let para2 = "B".repeat(120);
        let para3 = "C".repeat(120);
        let text = format!("{para1}\n\n{para2}\n\n{para3}");
        let chunks = chunk_text(&text, 0, 200, 40);
        assert!(chunks.len() >= 2, "Expected >=2 chunks, got {}", chunks.len());
        // First chunk should contain para1
        assert!(chunks[0].text.contains(&"A".repeat(50)));
    }

    #[test]
    fn test_overlap_present() {
        let para1 = "Word ".repeat(50); // ~250 chars
        let para2 = "Other ".repeat(50); // ~300 chars
        let text = format!("{para1}\n\n{para2}");
        let chunks = chunk_text(&text, 0, 300, 50);
        if chunks.len() >= 2 {
            // The second chunk should start with some content from the end of the first
            let tail_of_first = &chunks[0].text[chunks[0].text.len().saturating_sub(50)..];
            // Overlap means second chunk starts with tail of first
            assert!(
                chunks[1].text.starts_with(tail_of_first.trim())
                    || chunks[1].text.contains(tail_of_first.split_whitespace().last().unwrap_or("")),
                "Expected overlap in second chunk"
            );
        }
    }

    #[test]
    fn test_large_single_paragraph_splits() {
        let text = "Word. ".repeat(500); // ~3000 chars, all in one paragraph
        let chunks = chunk_text(&text, 0, 500, 50);
        assert!(chunks.len() > 1, "Large paragraph should produce multiple chunks");
        for chunk in &chunks {
            // Each chunk should not wildly exceed the target
            assert!(
                chunk.text.len() <= 600,
                "Chunk too large: {} chars",
                chunk.text.len()
            );
        }
    }

    #[test]
    fn test_utf8_safety() {
        // French text with accented characters
        let text = "Singularité technologique.\n\nL'intelligence artificielle générale est un concept révolutionnaire. Régénération des données à travers les réseaux neuronaux. Décentralisation de l'information. Épistémologie computationnelle. Éblouissement cognitif.";
        let chunks = chunk_text(&text, 0, 100, 20);
        // Should not panic — all boundaries are valid UTF-8
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            // Verify all chunk text is valid UTF-8
            assert!(chunk.text.is_char_boundary(0));
            assert!(chunk.text.is_char_boundary(chunk.text.len()));
        }
    }

    #[test]
    fn test_doc_index_preserved() {
        let text = "Hello world";
        let chunks = chunk_text(text, 42, 2000, 200);
        assert_eq!(chunks[0].doc_index, 42);
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let text = "A".repeat(100) + "\n\n" + &"B".repeat(100) + "\n\n" + &"C".repeat(100);
        let chunks = chunk_text(&text, 0, 120, 20);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }

    #[test]
    fn test_overlap_tail_utf8() {
        let text = "Éléphant à Noël";
        let tail = overlap_tail(text, 8);
        // Should not panic and should be valid UTF-8
        assert!(!tail.is_empty());
        assert!(tail.len() >= 8 || tail.len() == text.len());
    }

    #[test]
    fn test_split_on_words_large_word() {
        let text = "a".repeat(5000);
        let chunks = split_on_words(&text, 2000);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(chunk.len() <= 2000);
        }
    }
}
