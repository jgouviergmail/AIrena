//! Model architecture parsing, VRAM detection, and `num_ctx` recommendation.
//!
//! Uses Ollama's `/api/show` and `/api/ps` responses plus GPU VRAM detection
//! (NVIDIA via `nvidia-smi`) to compute the optimal context window size.

use serde::{Deserialize, Serialize};

use super::types::{ModelInfo, PsResponse, ShowResponse};
use crate::constants;

// ── Public output types ──────────────────────────────────────────────────

/// Extracted architecture information from a model's `/api/show` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelArchInfo {
    /// Model family detected (e.g. "llama", "qwen2", "gemma2").
    pub family: String,
    /// Number of transformer layers.
    pub block_count: u32,
    /// Number of attention heads.
    pub head_count: u32,
    /// Number of KV attention heads (GQA — may be less than head_count).
    pub head_count_kv: u32,
    /// Embedding dimension (total).
    pub embedding_length: u32,
    /// Dimension per attention head (key/value).
    pub head_dim: u32,
    /// Model's native maximum context length.
    pub context_length: u32,
    /// Quantization level (e.g. "Q4_K_M", "Q8_0", "F16").
    pub quantization: String,
    /// Bytes per KV cache token (both K and V combined).
    pub kv_bytes_per_token: f64,
}

/// GPU VRAM status detected on the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VramStatus {
    /// GPU name (e.g. "NVIDIA GeForce RTX 4090").
    pub gpu_name: String,
    /// Total VRAM in MiB.
    pub total_mb: u64,
    /// Free VRAM in MiB.
    pub free_mb: u64,
    /// How VRAM was detected ("nvidia-smi", "manual").
    pub detection_method: String,
}

/// Combined model + VRAM information with `num_ctx` recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelBudgetInfo {
    /// Model architecture details (None if parsing failed).
    pub arch: Option<ModelArchInfo>,
    /// GPU VRAM status (None if detection failed).
    pub vram: Option<VramStatus>,
    /// Recommended maximum `num_ctx` for this model + GPU combo.
    pub recommended_num_ctx: Option<u32>,
    /// Currently loaded model's `num_ctx` (from `/api/ps`), if available.
    pub current_num_ctx: Option<u32>,
    /// VRAM (MiB) currently used by all Ollama models (from `/api/ps` `size_vram`).
    /// Helps the user understand the VRAM breakdown (Ollama vs system vs free).
    pub ollama_vram_mb: Option<u64>,
    /// Whether the model's template supports think/reasoning mode.
    pub supports_think: bool,
    /// Warnings emitted during analysis.
    pub warnings: Vec<String>,
}

// ── Architecture parsing ─────────────────────────────────────────────────

/// Parse model architecture from an `/api/show` response.
///
/// The `model_info` HashMap keys are prefixed by the model family
/// (e.g. `"llama.block_count"`, `"qwen2.embedding_length"`).
/// We detect the family from the `details.family` field first,
/// then fall back to scanning keys for known architecture fields.
pub fn parse_arch_info(show: &ShowResponse) -> Option<ModelArchInfo> {
    let family = detect_model_family(show)?;

    let get_u32 = |key: &str| -> Option<u32> {
        show.model_info
            .get(&format!("{family}.{key}"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
    };

    let block_count = get_u32("block_count")?;
    let head_count = get_u32("attention.head_count")?;
    let head_count_kv = get_u32("attention.head_count_kv").unwrap_or(head_count);
    let embedding_length = get_u32("embedding_length")?;
    let context_length = get_u32("context_length").unwrap_or(constants::LLM_DEFAULT_NUM_CTX);

    // head_dim fallback cascade: key_length → rope.dimension_count → embedding / head_count
    let head_dim = get_u32("attention.key_length")
        .or_else(|| get_u32("rope.dimension_count"))
        .unwrap_or_else(|| {
            if head_count > 0 {
                embedding_length / head_count
            } else {
                128 // sensible default for modern models
            }
        });

    let quantization = show.details.quantization_level.clone();
    let dtype_bytes = dtype_bytes_from_quantization(&quantization);

    // KV cache formula: 2 × block_count × head_count_kv × head_dim × dtype_bytes
    let kv_bytes_per_token =
        2.0 * block_count as f64 * head_count_kv as f64 * head_dim as f64 * dtype_bytes;

    Some(ModelArchInfo {
        family,
        block_count,
        head_count,
        head_count_kv,
        embedding_length,
        head_dim,
        context_length,
        quantization,
        kv_bytes_per_token,
    })
}

/// Detect the model family prefix used in `model_info` keys.
///
/// Strategy: try `details.family` first, then scan keys for `*.block_count`.
fn detect_model_family(show: &ShowResponse) -> Option<String> {
    // 1. Use the family from details if non-empty and valid
    let family_from_details = show.details.family.to_lowercase();
    if !family_from_details.is_empty() {
        let key = format!("{family_from_details}.block_count");
        if show.model_info.contains_key(&key) {
            return Some(family_from_details);
        }
    }

    // 2. Scan keys for the first `*.block_count` entry
    for key in show.model_info.keys() {
        if let Some(prefix) = key.strip_suffix(".block_count") {
            return Some(prefix.to_string());
        }
    }

    // 3. If we have at least a non-empty family, try it anyway (some models
    //    may not have block_count but still have other fields)
    if !family_from_details.is_empty() {
        return Some(family_from_details);
    }

    None
}

/// Estimate dtype bytes from the KV cache type or quantization string.
///
/// Ollama uses f16 by default for KV cache. If `OLLAMA_KV_CACHE_TYPE` env var
/// is set, we could read it, but since this is for recommendation only, f16 is safe.
fn dtype_bytes_from_quantization(_quantization: &str) -> f64 {
    // KV cache in Ollama is always f16 by default regardless of model quantization.
    // Only OLLAMA_KV_CACHE_TYPE=q8_0 or q4_0 would change this, which is rare.
    constants::KV_CACHE_DTYPE_BYTES as f64
}

// ── Think mode detection ──────────────────────────────────────────────────

/// Detect whether a model supports think/reasoning mode by inspecting its Ollama template.
/// Models that support thinking include `{{- if .Think }}` or similar Go template directives.
pub fn detect_think_support(template: &str) -> bool {
    template.contains(".Think")
}

// ── VRAM detection ───────────────────────────────────────────────────────

/// Detect GPU VRAM using `nvidia-smi` (NVIDIA GPUs only).
///
/// Returns `None` if detection fails (no NVIDIA GPU, nvidia-smi not in PATH, etc.).
/// AMD GPUs on Windows have no reliable CLI equivalent — manual config required.
pub async fn detect_gpu_vram() -> Option<VramStatus> {
    let output = tokio::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // nvidia-smi outputs one line per GPU. Use the first GPU.
    let line = stdout.lines().next()?;
    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }

    let gpu_name = parts[0].to_string();
    let total_mb = parts[1].parse::<u64>().ok()?;
    let free_mb = parts[2].parse::<u64>().ok()?;

    Some(VramStatus {
        gpu_name,
        total_mb,
        free_mb,
        detection_method: "nvidia-smi".to_string(),
    })
}

// ── num_ctx recommendation ───────────────────────────────────────────────

/// Recommend an optimal `num_ctx` based on model architecture, available VRAM,
/// and estimated model weight.
///
/// The formula accounts for:
/// - Reconstructed "clean" free VRAM (raw free + loaded models' VRAM)
/// - Model weight VRAM (LLM + embedding file sizes ≈ VRAM for weights)
/// - Safety margin for OS/driver overhead
/// - KV cache cost per token (from model architecture)
/// - Model's native context length as upper bound
///
/// # Arguments
/// - `arch` — Model architecture info (KV cache cost, context length)
/// - `vram` — Raw GPU VRAM status from nvidia-smi
/// - `model_weight_bytes` — Combined file sizes of LLM + embedding models
/// - `loaded_vram_bytes` — Sum of `size_vram` from `/api/ps` for loaded models
pub fn recommend_num_ctx(
    arch: &ModelArchInfo,
    vram: &VramStatus,
    model_weight_bytes: u64,
    loaded_vram_bytes: u64,
) -> (u32, Vec<String>) {
    let mut warnings = Vec::new();

    if arch.kv_bytes_per_token <= 0.0 {
        warnings.push("Cannot compute KV cache cost — kv_bytes_per_token is zero".to_string());
        return (constants::LLM_DEFAULT_NUM_CTX, warnings);
    }

    // 1. Reconstruct "clean" free VRAM (as if no models loaded).
    //    raw free_mb + VRAM used by loaded models = what free would be if everything unloaded.
    let clean_free_mb =
        (vram.free_mb + loaded_vram_bytes / (1024 * 1024)).min(vram.total_mb);

    // 2. Subtract safety margin + model weights (LLM + embedding).
    //    VRAM budget: total = model_weights (fixed) + KV_cache (variable) + safety_margin
    //    Therefore: KV_budget = clean_free - safety - model_weights
    let model_weight_mb = model_weight_bytes / (1024 * 1024);
    let available_for_kv_mb = clean_free_mb
        .saturating_sub(constants::VRAM_SAFETY_MARGIN_MB as u64)
        .saturating_sub(model_weight_mb);

    tracing::info!(
        "VRAM budget: clean_free={clean_free_mb}MB - safety={}MB - model_weight={model_weight_mb}MB = {available_for_kv_mb}MB for KV cache",
        constants::VRAM_SAFETY_MARGIN_MB,
    );

    // 3. Convert to bytes and divide by KV cache cost per token.
    let available_for_kv_bytes = available_for_kv_mb * 1024 * 1024;
    let max_num_ctx_from_vram =
        (available_for_kv_bytes as f64 / arch.kv_bytes_per_token).floor() as u32;

    // Clamp to model's native context length and our configured maximum.
    let recommended = max_num_ctx_from_vram
        .min(arch.context_length)
        .min(constants::BUDGET_MAX_RECOMMENDED_NUM_CTX as u32);

    if recommended < constants::BUDGET_MIN_VIABLE_NUM_CTX as u32 {
        warnings.push(format!(
            "Available VRAM ({clean_free_mb} MB clean free, {model_weight_mb} MB model weight) \
             too low for meaningful context. \
             Recommended num_ctx ({recommended}) below minimum viable ({}).",
            constants::BUDGET_MIN_VIABLE_NUM_CTX
        ));
    }

    if max_num_ctx_from_vram < arch.context_length {
        warnings.push(format!(
            "VRAM limits num_ctx to {max_num_ctx_from_vram} \
             (model supports up to {}).",
            arch.context_length
        ));
    }

    (recommended, warnings)
}

// ── Model weight helpers ────────────────────────────────────────────────

/// Find a model's file size (bytes) from the model list.
/// File size ≈ VRAM for model weights (validated for quantized models).
/// Returns 0 if model not found.
pub fn find_model_file_size(models: &[ModelInfo], model_name: &str) -> u64 {
    if model_name.is_empty() {
        return 0;
    }
    models
        .iter()
        .find(|m| m.name == model_name || m.name.starts_with(model_name))
        .map(|m| m.size)
        .unwrap_or(0)
}

/// Sum of VRAM used by all currently loaded models (from `/api/ps` `size_vram`).
pub fn total_loaded_vram(ps: &PsResponse) -> u64 {
    ps.models.iter().map(|m| m.size_vram).sum()
}

/// Build a complete `ModelBudgetInfo` from a `/api/show` response + VRAM detection.
///
/// # Arguments
/// - `show` — Model architecture metadata from `/api/show`
/// - `model_weight_bytes` — Combined file sizes of LLM + embedding models (from `/api/tags`)
/// - `loaded_vram_bytes` — Sum of `size_vram` from `/api/ps` for currently loaded models
pub async fn build_model_budget_info(
    show: &ShowResponse,
    model_weight_bytes: u64,
    loaded_vram_bytes: u64,
) -> ModelBudgetInfo {
    let mut all_warnings = Vec::new();

    let arch = parse_arch_info(show);
    let supports_think = detect_think_support(&show.template);
    if arch.is_none() {
        all_warnings.push("Could not parse model architecture — num_ctx recommendation unavailable.".to_string());
    }

    let vram = detect_gpu_vram().await;
    if vram.is_none() {
        all_warnings.push(
            "GPU VRAM detection failed (NVIDIA only). Configure num_ctx manually.".to_string(),
        );
    }

    let (recommended_num_ctx, rec_warnings) = match (&arch, &vram) {
        (Some(a), Some(v)) => {
            let (rec, w) = recommend_num_ctx(a, v, model_weight_bytes, loaded_vram_bytes);
            (Some(rec), w)
        }
        _ => (None, Vec::new()),
    };
    all_warnings.extend(rec_warnings);

    // Expose Ollama VRAM usage for frontend display (convert bytes → MiB).
    // When loaded_vram_bytes is 0 (e.g. after unload), show None to avoid misleading "0 MB".
    let ollama_vram_mb = if loaded_vram_bytes > 0 {
        Some(loaded_vram_bytes / (1024 * 1024))
    } else {
        None
    };

    ModelBudgetInfo {
        arch,
        vram,
        recommended_num_ctx,
        current_num_ctx: None,
        ollama_vram_mb,
        supports_think,
        warnings: all_warnings,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::ollama::types::ShowDetails;

    fn make_show_response(family: &str, overrides: &[(&str, u64)]) -> ShowResponse {
        let mut model_info = HashMap::new();

        // Default Llama 3.1 8B-like architecture
        let defaults = [
            ("block_count", 32),
            ("attention.head_count", 32),
            ("attention.head_count_kv", 8),
            ("embedding_length", 4096),
            ("attention.key_length", 128),
            ("context_length", 131072),
        ];

        for (key, val) in &defaults {
            model_info.insert(
                format!("{family}.{key}"),
                serde_json::Value::Number(serde_json::Number::from(*val)),
            );
        }

        for (key, val) in overrides {
            model_info.insert(
                format!("{family}.{key}"),
                serde_json::Value::Number(serde_json::Number::from(*val)),
            );
        }

        ShowResponse {
            template: String::new(),
            details: ShowDetails {
                family: family.to_string(),
                quantization_level: "Q4_K_M".to_string(),
                ..Default::default()
            },
            model_info,
        }
    }

    #[test]
    fn test_parse_arch_info_llama() {
        let show = make_show_response("llama", &[]);
        let arch = parse_arch_info(&show).expect("Should parse llama arch");

        assert_eq!(arch.family, "llama");
        assert_eq!(arch.block_count, 32);
        assert_eq!(arch.head_count, 32);
        assert_eq!(arch.head_count_kv, 8);
        assert_eq!(arch.embedding_length, 4096);
        assert_eq!(arch.head_dim, 128);
        assert_eq!(arch.context_length, 131072);
        // KV bytes: 2 × 32 × 8 × 128 × 2 = 131_072 bytes/token
        assert!((arch.kv_bytes_per_token - 131_072.0).abs() < 1.0);
    }

    #[test]
    fn test_parse_arch_info_qwen2() {
        let show = make_show_response(
            "qwen2",
            &[
                ("block_count", 28),
                ("attention.head_count", 16),
                ("attention.head_count_kv", 2),
                ("embedding_length", 2048),
                ("context_length", 32768),
            ],
        );
        let arch = parse_arch_info(&show).expect("Should parse qwen2 arch");

        assert_eq!(arch.family, "qwen2");
        assert_eq!(arch.head_count_kv, 2);
        assert_eq!(arch.head_dim, 128);
    }

    #[test]
    fn test_parse_arch_info_missing_fields() {
        let show = ShowResponse {
            template: String::new(),
            details: ShowDetails::default(),
            model_info: HashMap::new(),
        };
        assert!(parse_arch_info(&show).is_none());
    }

    #[test]
    fn test_detect_family_from_keys() {
        // Family field is empty but keys have prefix
        let mut model_info = HashMap::new();
        model_info.insert(
            "gemma2.block_count".to_string(),
            serde_json::Value::Number(28.into()),
        );
        model_info.insert(
            "gemma2.attention.head_count".to_string(),
            serde_json::Value::Number(16.into()),
        );
        model_info.insert(
            "gemma2.embedding_length".to_string(),
            serde_json::Value::Number(3072.into()),
        );

        let show = ShowResponse {
            template: String::new(),
            details: ShowDetails::default(),
            model_info,
        };

        let family = detect_model_family(&show);
        assert_eq!(family, Some("gemma2".to_string()));
    }

    fn make_llama_arch() -> ModelArchInfo {
        ModelArchInfo {
            family: "llama".to_string(),
            block_count: 32,
            head_count: 32,
            head_count_kv: 8,
            embedding_length: 4096,
            head_dim: 128,
            context_length: 131072,
            quantization: "Q4_K_M".to_string(),
            kv_bytes_per_token: 131_072.0,
        }
    }

    #[test]
    fn test_recommend_num_ctx_basic() {
        let arch = make_llama_arch();
        // Simulate clean VRAM (no models loaded) on a 24 GB GPU
        let vram = VramStatus {
            gpu_name: "Test GPU".to_string(),
            total_mb: 24576,
            free_mb: 22000, // ~22 GB free (clean)
            detection_method: "test".to_string(),
        };
        // ~5 GB model weight (Q4_K_M 8B)
        let model_weight = 5_000_000_000_u64;

        let (recommended, warnings) = recommend_num_ctx(&arch, &vram, model_weight, 0);

        // clean_free=22000, model_weight=4768MB, safety=512MB
        // available_kv = 22000 - 512 - 4768 = 16720 MB = 17,532,108,800 bytes
        // 17,532,108,800 / 131_072 ≈ 133,791 → capped at 131,072
        assert!(recommended > 0);
        assert!(recommended <= 131_072);
        // With 24 GB GPU and 5 GB model, should have plenty of VRAM
        assert!(recommended > 10_000, "Should recommend a decent context size");
    }

    #[test]
    fn test_recommend_num_ctx_small_vram() {
        let arch = make_llama_arch();
        let vram = VramStatus {
            gpu_name: "Small GPU".to_string(),
            total_mb: 4096,
            free_mb: 3800, // ~3.8 GB free (clean)
            detection_method: "test".to_string(),
        };
        // 5 GB model > 3.8 GB free → insufficient
        let model_weight = 5_000_000_000_u64;

        let (recommended, warnings) = recommend_num_ctx(&arch, &vram, model_weight, 0);

        // clean_free=3800, model_weight=4768 → 3800-512-4768 = saturates to 0
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("too low")));
        assert!(recommended < constants::BUDGET_MIN_VIABLE_NUM_CTX as u32);
    }

    #[test]
    fn test_recommend_num_ctx_with_loaded_models() {
        let arch = make_llama_arch();
        // Simulate: model is currently loaded, nvidia-smi shows reduced free
        let vram = VramStatus {
            gpu_name: "Test GPU".to_string(),
            total_mb: 24576,
            free_mb: 14000, // 14 GB free (model is loaded, occupying ~8 GB)
            detection_method: "test".to_string(),
        };
        let model_weight = 5_000_000_000_u64;
        // Model occupies ~8 GB in VRAM (weights + default KV cache)
        let loaded_vram = 8_000_000_000_u64;

        let (rec_loaded, _) = recommend_num_ctx(&arch, &vram, model_weight, loaded_vram);

        // Reconstruction: clean_free = 14000 + 8000/1024/1024 = 14000 + 7629 = 21629
        // (capped at 24576)
        // Then: 21629 - 512 - 4768 = 16349 MB for KV
        // Compare to clean scenario: free=22000, loaded=0
        let clean_vram = VramStatus {
            gpu_name: "Test GPU".to_string(),
            total_mb: 24576,
            free_mb: 22000,
            detection_method: "test".to_string(),
        };
        let (rec_clean, _) = recommend_num_ctx(&arch, &clean_vram, model_weight, 0);

        // Both should give similar results (within 5% of each other)
        let diff = (rec_loaded as f64 - rec_clean as f64).abs();
        let max_val = rec_loaded.max(rec_clean) as f64;
        assert!(
            diff / max_val < 0.05,
            "Loaded ({rec_loaded}) and clean ({rec_clean}) recommendations should be similar"
        );
    }

    #[test]
    fn test_find_model_file_size() {
        let models = vec![
            ModelInfo { name: "llama3.1:8b".to_string(), size: 5_000_000_000, digest: String::new() },
            ModelInfo { name: "nomic-embed-text:latest".to_string(), size: 274_000_000, digest: String::new() },
        ];

        // Exact match
        assert_eq!(find_model_file_size(&models, "llama3.1:8b"), 5_000_000_000);
        // starts_with match
        assert_eq!(find_model_file_size(&models, "llama3.1"), 5_000_000_000);
        // Not found
        assert_eq!(find_model_file_size(&models, "nonexistent"), 0);
        // Empty name
        assert_eq!(find_model_file_size(&models, ""), 0);
    }

    #[test]
    fn test_total_loaded_vram() {
        use crate::ollama::types::{PsModel, PsResponse};

        let ps = PsResponse {
            models: vec![
                PsModel {
                    name: "llama3.1:8b".to_string(),
                    model: "llama3.1:8b".to_string(),
                    size: 5_000_000_000,
                    size_vram: 6_000_000_000,
                    expires_at: String::new(),
                },
                PsModel {
                    name: "nomic-embed-text:latest".to_string(),
                    model: "nomic-embed-text:latest".to_string(),
                    size: 274_000_000,
                    size_vram: 300_000_000,
                    expires_at: String::new(),
                },
            ],
        };

        assert_eq!(total_loaded_vram(&ps), 6_300_000_000);

        // Empty
        let empty_ps = PsResponse { models: vec![] };
        assert_eq!(total_loaded_vram(&empty_ps), 0);
    }

    #[test]
    fn test_head_dim_fallback_to_rope() {
        let mut model_info = HashMap::new();
        model_info.insert("test.block_count".into(), serde_json::json!(32));
        model_info.insert("test.attention.head_count".into(), serde_json::json!(32));
        model_info.insert("test.attention.head_count_kv".into(), serde_json::json!(8));
        model_info.insert("test.embedding_length".into(), serde_json::json!(4096));
        model_info.insert("test.rope.dimension_count".into(), serde_json::json!(64));
        model_info.insert("test.context_length".into(), serde_json::json!(8192));
        // No attention.key_length — should fallback to rope.dimension_count

        let show = ShowResponse {
            template: String::new(),
            details: ShowDetails {
                family: "test".to_string(),
                quantization_level: "Q4_0".to_string(),
                ..Default::default()
            },
            model_info,
        };

        let arch = parse_arch_info(&show).expect("Should parse");
        assert_eq!(arch.head_dim, 64, "Should use rope.dimension_count as fallback");
    }

    #[test]
    fn test_head_dim_fallback_to_embedding_div() {
        let mut model_info = HashMap::new();
        model_info.insert("test.block_count".into(), serde_json::json!(24));
        model_info.insert("test.attention.head_count".into(), serde_json::json!(16));
        model_info.insert("test.embedding_length".into(), serde_json::json!(2048));
        model_info.insert("test.context_length".into(), serde_json::json!(4096));
        // No key_length, no rope — should compute 2048/16 = 128

        let show = ShowResponse {
            template: String::new(),
            details: ShowDetails {
                family: "test".to_string(),
                quantization_level: "F16".to_string(),
                ..Default::default()
            },
            model_info,
        };

        let arch = parse_arch_info(&show).expect("Should parse");
        assert_eq!(arch.head_dim, 128, "Should compute embedding_length / head_count");
        // head_count_kv defaults to head_count when missing
        assert_eq!(arch.head_count_kv, 16);
    }

    #[test]
    fn test_detect_think_support() {
        // DeepSeek R1 style template
        assert!(detect_think_support("{{- if .Think }}<think>{{ .Think }}</think>{{- end }}{{ .Content }}"));
        // Standard model without think
        assert!(!detect_think_support("{{ .System }}\n{{ .Prompt }}"));
        // Empty template
        assert!(!detect_think_support(""));
    }
}
