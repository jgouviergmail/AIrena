//! DeepSeek price list and cost estimation.
//!
//! Prices live in `constants.rs` (dated by `DEEPSEEK_PRICING_DATE`). Costs are
//! **estimates**: the provider's invoice is authoritative. Reasoning tokens are
//! billed as output tokens; cached prompt tokens are billed at the cache-hit rate.

use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};

use crate::constants;
use crate::models::llm::LlmUsage;

/// Peak-hour prices in USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    pub input_cache_hit: f64,
    pub input_cache_miss: f64,
    pub output: f64,
}

const FLASH: ModelPricing = ModelPricing {
    input_cache_hit: constants::DEEPSEEK_PRICE_FLASH_INPUT_HIT,
    input_cache_miss: constants::DEEPSEEK_PRICE_FLASH_INPUT_MISS,
    output: constants::DEEPSEEK_PRICE_FLASH_OUTPUT,
};

const V4_PRO: ModelPricing = ModelPricing {
    input_cache_hit: constants::DEEPSEEK_PRICE_V4PRO_INPUT_HIT,
    input_cache_miss: constants::DEEPSEEK_PRICE_V4PRO_INPUT_MISS,
    output: constants::DEEPSEEK_PRICE_V4PRO_OUTPUT,
};

/// Price list for a model id, including retired aliases that DeepSeek still routes.
/// Unknown models return `None` (tokens are counted, cost is shown as unavailable).
pub fn pricing_for(model: &str) -> Option<ModelPricing> {
    match model.trim().to_lowercase().as_str() {
        "deepseek-flash" | "deepseek-v4-flash" | "deepseek-v4-flash-vision-exp" | "deepseek-chat"
        | "deepseek-reasoner" => Some(FLASH),
        "deepseek-v4-pro" => Some(V4_PRO),
        _ => None,
    }
}

/// Peak pricing applies Monday–Friday inside the UTC windows of
/// `DEEPSEEK_PEAK_WINDOWS_UTC`; everything else is off-peak (half price).
pub fn is_peak_hour(now: DateTime<Utc>) -> bool {
    if matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }
    let hour = now.hour();
    constants::DEEPSEEK_PEAK_WINDOWS_UTC
        .iter()
        .any(|&(start, end)| hour >= start && hour < end)
}

/// Estimated cost in USD for one usage record at the given tariff period.
pub fn estimate_cost_usd(model: &str, usage: &LlmUsage, peak: bool) -> Option<f64> {
    let pricing = pricing_for(model)?;
    let factor = if peak { 1.0 } else { constants::DEEPSEEK_OFFPEAK_FACTOR };
    let cached = usage.cached_tokens.min(usage.prompt_tokens) as f64;
    let uncached = (usage.prompt_tokens as f64) - cached;
    let output = usage.completion_tokens as f64;
    let per_million = cached * pricing.input_cache_hit
        + uncached * pricing.input_cache_miss
        + output * pricing.output;
    Some(per_million * factor / 1_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, s).unwrap()
    }

    #[test]
    fn peak_windows_and_boundaries() {
        // 2026-09-16 is a Wednesday
        assert!(is_peak_hour(utc(2026, 9, 16, 1, 0, 0)));
        assert!(is_peak_hour(utc(2026, 9, 16, 3, 59, 59)));
        assert!(!is_peak_hour(utc(2026, 9, 16, 4, 0, 0)));
        assert!(!is_peak_hour(utc(2026, 9, 16, 5, 30, 0)));
        assert!(is_peak_hour(utc(2026, 9, 16, 6, 0, 0)));
        assert!(is_peak_hour(utc(2026, 9, 16, 9, 59, 59)));
        assert!(!is_peak_hour(utc(2026, 9, 16, 10, 0, 0)));
        assert!(!is_peak_hour(utc(2026, 9, 16, 0, 59, 59)));
        assert!(!is_peak_hour(utc(2026, 9, 16, 23, 0, 0)));
    }

    #[test]
    fn weekend_is_never_peak() {
        // 2026-09-19 Saturday, 2026-09-20 Sunday
        assert!(!is_peak_hour(utc(2026, 9, 19, 2, 0, 0)));
        assert!(!is_peak_hour(utc(2026, 9, 20, 7, 0, 0)));
        // Monday 2026-09-21 is peak again
        assert!(is_peak_hour(utc(2026, 9, 21, 7, 0, 0)));
    }

    #[test]
    fn pricing_lookup_with_aliases_and_unknown() {
        assert_eq!(pricing_for("deepseek-flash"), Some(FLASH));
        assert_eq!(pricing_for("DeepSeek-V4-Flash"), Some(FLASH));
        assert_eq!(pricing_for("deepseek-chat"), Some(FLASH));
        assert_eq!(pricing_for("deepseek-v4-pro"), Some(V4_PRO));
        assert_eq!(pricing_for("deepseek-v5-ultra"), None);
        assert_eq!(pricing_for(""), None);
    }

    #[test]
    fn cost_estimate_flash_peak_and_offpeak() {
        // 1M uncached input + 1M output at peak = 0.30 + 1.20
        let usage = LlmUsage { prompt_tokens: 1_000_000, cached_tokens: 0, completion_tokens: 1_000_000, reasoning_tokens: 0 };
        let peak = estimate_cost_usd("deepseek-flash", &usage, true).unwrap();
        assert!((peak - 1.50).abs() < 1e-9, "{peak}");
        let off = estimate_cost_usd("deepseek-flash", &usage, false).unwrap();
        assert!((off - 0.75).abs() < 1e-9, "{off}");
    }

    #[test]
    fn cost_estimate_uses_cache_hit_rate_for_cached_tokens() {
        // 1M input fully cached at peak = 0.006
        let usage = LlmUsage { prompt_tokens: 1_000_000, cached_tokens: 1_000_000, completion_tokens: 0, reasoning_tokens: 0 };
        let cost = estimate_cost_usd("deepseek-flash", &usage, true).unwrap();
        assert!((cost - 0.006).abs() < 1e-9, "{cost}");
        // cached > prompt (malformed) is clamped, never negative
        let weird = LlmUsage { prompt_tokens: 10, cached_tokens: 100, completion_tokens: 0, reasoning_tokens: 0 };
        assert!(estimate_cost_usd("deepseek-flash", &weird, true).unwrap() >= 0.0);
    }

    #[test]
    fn cost_estimate_unknown_model_is_none_and_small_usage_scales() {
        assert!(estimate_cost_usd("mystery", &LlmUsage::default(), true).is_none());
        let usage = LlmUsage { prompt_tokens: 62_000, cached_tokens: 13_000, completion_tokens: 6_000, reasoning_tokens: 2_000 };
        let cost = estimate_cost_usd("deepseek-v4-pro", &usage, true).unwrap();
        // 13k×0.044 + 49k×1.32 + 6k×3.96 = 0.572 + 64.68 + 23.76 = 89.012 per-million-units → 0.089012 USD
        assert!((cost - 0.089012).abs() < 1e-6, "{cost}");
    }
}
