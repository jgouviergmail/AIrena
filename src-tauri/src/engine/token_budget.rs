//! Token Budget Calculator — dynamic allocation of prompt sections based on num_ctx.
//!
//! Uses a strict-priority waterfall algorithm to distribute available context tokens
//! across variable-size prompt sections (memory, messages, directives, etc.).

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::constants;

// ── Budget sections ────────────────────────────────────────────────────

/// Identifiers for each variable prompt section that can receive budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BudgetSection {
    CurrentTurnMessages,
    ImmediateMemory,
    ContextualSummary,
    CognitiveDirectives,
    ArbitreDirectives,
    FullDocument,
    RagContext,
    WebWikiSearch,
    PositionalMap,
}

impl BudgetSection {
    /// Sections whose priority rank can be configured by the user in Settings.
    pub const CONFIGURABLE: &[BudgetSection] = &[
        BudgetSection::CurrentTurnMessages,
        BudgetSection::ImmediateMemory,
        BudgetSection::ContextualSummary,
        BudgetSection::CognitiveDirectives,
        BudgetSection::ArbitreDirectives,
        BudgetSection::WebWikiSearch,
        BudgetSection::PositionalMap,
    ];

    /// Document-related sections — always last in the waterfall, not user-configurable.
    pub const DOCUMENT: &[BudgetSection] = &[
        BudgetSection::FullDocument,
        BudgetSection::RagContext,
    ];
}

// ── Priority definition ────────────────────────────────────────────────

/// A section with its priority rank (lower = higher priority), floor, and ceiling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionPriority {
    pub section: BudgetSection,
    /// Priority rank: 4 = highest variable priority, 12 = lowest. Fixed sections (1-3) not here.
    pub rank: u8,
    /// Minimum chars allocated to this section.
    pub floor: usize,
    /// Maximum chars allocated to this section.
    pub ceiling: usize,
}

// ── Input parameters ───────────────────────────────────────────────────

/// Flags indicating which optional features are active.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetFeatures {
    pub web_search_enabled: bool,
    pub wiki_search_enabled: bool,
    pub rag_enabled: bool,
    pub document_chars: usize,
}

/// All inputs needed to compute a token budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetParams {
    /// Context window size in tokens.
    pub num_ctx: u32,
    /// Tokens reserved for the generated response.
    pub num_predict: i32,
    /// Length of the speaker's system prompt in chars (varies per speaker).
    pub system_prompt_chars: usize,
    /// Number of GladIAteurs (not counting IArbitre or user).
    pub n_gladiateurs: usize,
    /// Discussion language code ("fr", "en", "zh").
    pub language: String,
    /// Active feature flags.
    #[serde(default)]
    pub features: BudgetFeatures,
}

// ── Output: computed budget ────────────────────────────────────────────

/// Per-section character caps computed by the waterfall algorithm.
/// Each field is a per-message or per-section total — see field docs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBudget {
    /// Max chars per message for current turn messages.
    pub current_turn_msg_chars: usize,
    /// Max chars per message for immediate memory messages.
    pub immediate_memory_msg_chars: usize,
    /// Max total chars for the contextual summary.
    pub contextual_summary_chars: usize,
    /// Max total chars for cognitive directives.
    pub cognitive_directives_chars: usize,
    /// Max total chars for IArbitre directives.
    pub arbitre_directives_chars: usize,
    /// Max total chars for full document injection (0 = not injected).
    pub full_document_chars: usize,
    /// Whether the full document fits in the budget (true = full injection, false = RAG).
    pub full_document_mode: bool,
    /// Max total chars for RAG context.
    pub rag_context_chars: usize,
    /// Max total chars for web+wiki search results.
    pub web_wiki_chars: usize,
    /// Max total chars for positional map.
    pub positional_map_chars: usize,
}

impl TokenBudget {
    /// Combined budget for all external knowledge sources (web/wiki + RAG).
    /// Since the orchestrator merges web/wiki search results and RAG context
    /// into a single string before passing to prompt builders, truncation must
    /// use the combined allocation.
    pub fn external_knowledge_chars(&self) -> usize {
        self.web_wiki_chars + self.rag_context_chars
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        // Fallback: use existing fixed constants for backward compatibility.
        Self {
            current_turn_msg_chars: constants::TRUNC_CURRENT_TURN,
            immediate_memory_msg_chars: constants::TRUNC_IMMEDIATE_MEMORY,
            contextual_summary_chars: constants::MEMORY_MAX_SUMMARY_CHARS,
            cognitive_directives_chars: constants::BUDGET_CEIL_COGNITIVE_DIRECTIVES,
            arbitre_directives_chars: constants::TRUNC_MODERATOR_DIRECTIVE,
            full_document_chars: 0,
            full_document_mode: false,
            rag_context_chars: constants::RAG_MAX_CONTEXT_LEN,
            web_wiki_chars: constants::SEARCH_MAX_CONTEXT_LEN,
            // ~5 participants × 200 chars/participant — reasonable default fallback.
            positional_map_chars: constants::BUDGET_CEIL_POSITIONAL_MAP_PER_PARTICIPANT * 5,
        }
    }
}

// ── Frontend preview ───────────────────────────────────────────────────

/// Per-section allocation detail for the frontend preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionAllocation {
    pub section: BudgetSection,
    pub allocated_chars: usize,
    pub floor_chars: usize,
    pub ceiling_chars: usize,
}

/// Overall quality assessment of the token budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BudgetQualityLevel {
    /// All sections have reached their ceiling — optimal quality.
    Optimal,
    /// Budget is tight, some sections are below their ceiling.
    Degraded,
    /// num_ctx below minimum viable OR non-negotiable sections exceed num_ctx.
    Insufficient,
}

/// Serializable budget summary for the setup page TokenBudgetPreview component.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBudgetPreview {
    /// Total tokens available (num_ctx).
    pub total_tokens: u32,
    /// Tokens reserved for non-negotiable sections (system prompt + deterministic + num_predict).
    pub reserved_tokens: usize,
    /// Tokens available for variable sections.
    pub available_tokens: usize,
    /// Per-section allocation details.
    pub sections: Vec<SectionAllocation>,
    /// Warnings emitted during budget computation.
    pub warnings: Vec<String>,
    /// Whether full document injection is active.
    pub full_document_mode: bool,
    /// Chars-per-token ratio used for this computation.
    pub chars_per_token: f64,
    /// Overall quality assessment.
    pub quality_level: BudgetQualityLevel,
    /// Tokens actually available for document sections (remaining after all non-document allocations).
    pub document_available_tokens: usize,
}

// ── Computation ────────────────────────────────────────────────────────

impl TokenBudget {
    /// Compute token budget allocation using a strict-priority waterfall algorithm.
    ///
    /// Returns the budget and any warnings (e.g. insufficient budget).
    pub fn compute(params: &BudgetParams, priorities: &[SectionPriority]) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();

        // ── Step 0: Validate minimum viable context ──
        if (params.num_ctx as usize) < constants::BUDGET_MIN_VIABLE_NUM_CTX {
            warnings.push(format!(
                "num_ctx ({}) is below minimum viable ({})",
                params.num_ctx,
                constants::BUDGET_MIN_VIABLE_NUM_CTX
            ));
            return (Self::default(), warnings);
        }

        // ── Step 1: Reserve non-negotiable sections (in tokens) ──
        let chars_per_token = chars_per_token_for_language(&params.language);
        let reserved_tokens = compute_reserved_tokens(params, chars_per_token);

        let available_tokens = if (params.num_ctx as usize) > reserved_tokens {
            params.num_ctx as usize - reserved_tokens
        } else {
            warnings.push(format!(
                "Non-negotiable sections ({reserved_tokens} tokens) exceed num_ctx ({}). \
                 No room for variable content.",
                params.num_ctx
            ));
            return (Self::default(), warnings);
        };

        // ── Step 2: Convert available tokens → chars ──
        let available_chars = (available_tokens as f64 * chars_per_token).floor() as usize;

        // ── Step 3: Build section entries with computed floors/ceilings ──
        let n_speakers = params.n_gladiateurs + 1; // +1 for IArbitre
        let immediate_memory_turns = constants::MEMORY_MAX_IMMEDIATE_TURNS;

        let mut entries: Vec<WaterfallEntry> = priorities
            .iter()
            .filter(|p| is_section_active(p.section, &params.features))
            .map(|p| {
                let (floor, ceiling) = scale_section_bounds(
                    p.section, p.floor, p.ceiling, n_speakers, immediate_memory_turns, params,
                );
                WaterfallEntry {
                    section: p.section,
                    rank: p.rank,
                    floor,
                    ceiling,
                    allocated: 0,
                }
            })
            .collect();

        // Sort by rank (ascending = highest priority first).
        entries.sort_by_key(|e| e.rank);

        // ── Step 4: Allocate floors ──
        let total_floors: usize = entries.iter().map(|e| e.floor).sum();

        if total_floors > available_chars {
            // Budget insufficient for all floors — cut from lowest priority first.
            warnings.push(format!(
                "Budget tight: floors need {total_floors} chars but only {available_chars} available. \
                 Low-priority sections will be reduced."
            ));
            let mut remaining = available_chars;
            for entry in &mut entries {
                let alloc = entry.floor.min(remaining);
                entry.allocated = alloc;
                remaining = remaining.saturating_sub(alloc);
            }
        } else {
            // All floors fit — allocate them.
            for entry in &mut entries {
                entry.allocated = entry.floor;
            }
        }

        // ── Step 5: Distribute surplus by priority ──
        let total_allocated: usize = entries.iter().map(|e| e.allocated).sum();
        let mut surplus = available_chars.saturating_sub(total_allocated);

        for entry in &mut entries {
            if surplus == 0 {
                break;
            }
            let headroom = entry.ceiling.saturating_sub(entry.allocated);
            let grant = headroom.min(surplus);
            entry.allocated += grant;
            surplus -= grant;
        }

        // ── Step 6: Build TokenBudget from allocated chars ──
        let mut budget = TokenBudget {
            current_turn_msg_chars: 0,
            immediate_memory_msg_chars: 0,
            contextual_summary_chars: 0,
            cognitive_directives_chars: 0,
            arbitre_directives_chars: 0,
            full_document_chars: 0,
            full_document_mode: false,
            rag_context_chars: 0,
            web_wiki_chars: 0,
            positional_map_chars: 0,
        };

        for entry in &entries {
            match entry.section {
                BudgetSection::CurrentTurnMessages => {
                    // Convert total allocation back to per-message.
                    budget.current_turn_msg_chars = if n_speakers > 0 {
                        entry.allocated / n_speakers
                    } else {
                        entry.allocated
                    };
                }
                BudgetSection::ImmediateMemory => {
                    let n_msgs = immediate_memory_turns * n_speakers;
                    budget.immediate_memory_msg_chars = if n_msgs > 0 {
                        entry.allocated / n_msgs
                    } else {
                        entry.allocated
                    };
                }
                BudgetSection::ContextualSummary => {
                    budget.contextual_summary_chars = entry.allocated;
                }
                BudgetSection::CognitiveDirectives => {
                    budget.cognitive_directives_chars = entry.allocated;
                }
                BudgetSection::ArbitreDirectives => {
                    budget.arbitre_directives_chars = entry.allocated;
                }
                BudgetSection::FullDocument => {
                    budget.full_document_chars = entry.allocated;
                    // Full injection only if the entire document fits.
                    budget.full_document_mode =
                        entry.allocated >= params.features.document_chars && entry.allocated > 0;
                }
                BudgetSection::RagContext => {
                    budget.rag_context_chars = entry.allocated;
                }
                BudgetSection::WebWikiSearch => {
                    budget.web_wiki_chars = entry.allocated;
                }
                BudgetSection::PositionalMap => {
                    budget.positional_map_chars = entry.allocated;
                }
            }
        }

        // If full document mode is active, RAG is unnecessary.
        if budget.full_document_mode {
            budget.rag_context_chars = 0;
        }

        (budget, warnings)
    }

    /// Build a frontend-ready preview from the compute result.
    pub fn to_preview(
        params: &BudgetParams,
        priorities: &[SectionPriority],
    ) -> TokenBudgetPreview {
        let chars_per_token = chars_per_token_for_language(&params.language);
        let reserved_tokens = compute_reserved_tokens(params, chars_per_token);
        let available_tokens = (params.num_ctx as usize).saturating_sub(reserved_tokens);

        let n_speakers = params.n_gladiateurs + 1;
        let immediate_memory_turns = constants::MEMORY_MAX_IMMEDIATE_TURNS;

        let (budget, warnings) = Self::compute(params, priorities);

        let sections: Vec<SectionAllocation> = priorities
            .iter()
            .filter(|p| is_section_active(p.section, &params.features))
            .map(|p| {
                let (floor, ceiling) = scale_section_bounds(
                    p.section, p.floor, p.ceiling, n_speakers, immediate_memory_turns, params,
                );
                let allocated = match p.section {
                    BudgetSection::CurrentTurnMessages => {
                        budget.current_turn_msg_chars * n_speakers
                    }
                    BudgetSection::ImmediateMemory => {
                        budget.immediate_memory_msg_chars * immediate_memory_turns * n_speakers
                    }
                    BudgetSection::ContextualSummary => budget.contextual_summary_chars,
                    BudgetSection::CognitiveDirectives => budget.cognitive_directives_chars,
                    BudgetSection::ArbitreDirectives => budget.arbitre_directives_chars,
                    BudgetSection::FullDocument => budget.full_document_chars,
                    BudgetSection::RagContext => budget.rag_context_chars,
                    BudgetSection::WebWikiSearch => budget.web_wiki_chars,
                    BudgetSection::PositionalMap => budget.positional_map_chars,
                };
                SectionAllocation {
                    section: p.section,
                    allocated_chars: allocated,
                    floor_chars: floor,
                    ceiling_chars: ceiling,
                }
            })
            .collect();

        // Compute quality level — "Optimal" only if ALL sections reached their ceiling.
        let all_at_ceiling = sections.iter().all(|s| s.allocated_chars >= s.ceiling_chars);
        let quality_level = if (params.num_ctx as usize) < constants::BUDGET_MIN_VIABLE_NUM_CTX
            || reserved_tokens >= params.num_ctx as usize
        {
            BudgetQualityLevel::Insufficient
        } else if !warnings.is_empty() || !all_at_ceiling {
            BudgetQualityLevel::Degraded
        } else {
            BudgetQualityLevel::Optimal
        };

        // Compute tokens available for document sections (remaining after non-document allocations)
        let non_doc_chars: usize = sections
            .iter()
            .filter(|s| !BudgetSection::DOCUMENT.contains(&s.section))
            .map(|s| s.allocated_chars)
            .sum();
        let non_doc_tokens = (non_doc_chars as f64 / chars_per_token).ceil() as usize;
        let document_available_tokens = available_tokens.saturating_sub(non_doc_tokens);

        TokenBudgetPreview {
            total_tokens: params.num_ctx,
            reserved_tokens,
            available_tokens,
            sections,
            warnings,
            full_document_mode: budget.full_document_mode,
            chars_per_token,
            quality_level,
            document_available_tokens,
        }
    }
}

// ── Default priorities ─────────────────────────────────────────────────

/// Build default section priorities. Rank 4 = highest variable priority, 12 = lowest.
pub fn default_priorities() -> Vec<SectionPriority> {
    use BudgetSection::*;
    vec![
        SectionPriority {
            section: CurrentTurnMessages,
            rank: 4,
            floor: constants::BUDGET_FLOOR_CURRENT_TURN,
            ceiling: constants::BUDGET_CEIL_CURRENT_TURN,
        },
        SectionPriority {
            section: ImmediateMemory,
            rank: 5,
            floor: constants::BUDGET_FLOOR_IMMEDIATE_MEMORY,
            ceiling: constants::BUDGET_CEIL_IMMEDIATE_MEMORY,
        },
        SectionPriority {
            section: ContextualSummary,
            rank: 6,
            floor: constants::BUDGET_FLOOR_CONTEXTUAL_SUMMARY,
            ceiling: constants::BUDGET_CEIL_CONTEXTUAL_SUMMARY,
        },
        SectionPriority {
            section: CognitiveDirectives,
            rank: 7,
            floor: constants::BUDGET_FLOOR_COGNITIVE_DIRECTIVES,
            ceiling: constants::BUDGET_CEIL_COGNITIVE_DIRECTIVES,
        },
        SectionPriority {
            section: ArbitreDirectives,
            rank: 8,
            floor: constants::BUDGET_FLOOR_ARBITRE_DIRECTIVES,
            ceiling: constants::BUDGET_CEIL_ARBITRE_DIRECTIVES,
        },
        SectionPriority {
            section: WebWikiSearch,
            rank: 9,
            floor: constants::BUDGET_FLOOR_WEB_WIKI,
            ceiling: constants::BUDGET_CEIL_WEB_WIKI,
        },
        SectionPriority {
            section: PositionalMap,
            rank: 10,
            floor: constants::BUDGET_FLOOR_POSITIONAL_MAP,
            ceiling: constants::BUDGET_CEIL_POSITIONAL_MAP_PER_PARTICIPANT,
        },
        // Document sections — always last, not user-configurable
        SectionPriority {
            section: FullDocument,
            rank: 11,
            floor: constants::BUDGET_FLOOR_FULL_DOCUMENT,
            ceiling: 0, // Ceiling is dynamic (= document_chars), set during compute.
        },
        SectionPriority {
            section: RagContext,
            rank: 12,
            floor: constants::BUDGET_FLOOR_RAG_CONTEXT,
            ceiling: constants::BUDGET_CEIL_RAG_CONTEXT,
        },
    ]
}

/// Parse user-customized priorities from a JSON string (stored in DB settings).
/// Returns `default_priorities()` if the string is empty, invalid, or incomplete.
///
/// Validation: every `BudgetSection` must appear exactly once.
/// User JSON only controls **rank order** — floor/ceiling always come from constants
/// via `apply_default_bounds()`.
pub fn parse_priorities_or_default(json: &str) -> Vec<SectionPriority> {
    if json.is_empty() {
        return default_priorities();
    }

    match serde_json::from_str::<Vec<SectionPriority>>(json) {
        Ok(priorities) => {
            // Validate completeness: every CONFIGURABLE section must be present.
            // Document sections are auto-appended by apply_default_bounds().
            let has_all = BudgetSection::CONFIGURABLE.iter().all(|s| {
                priorities.iter().any(|p| p.section == *s)
            });
            if has_all {
                apply_default_bounds(&priorities)
            } else {
                tracing::warn!(
                    "Saved token budget priorities are incomplete — using defaults"
                );
                default_priorities()
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to parse saved token budget priorities — using defaults"
            );
            default_priorities()
        }
    }
}

/// Take user-provided priorities (which may have floor/ceiling = 0 from the frontend),
/// preserve their rank order, but replace floor/ceiling with the correct default constants.
///
/// The frontend Settings page only controls rank order for CONFIGURABLE sections.
/// Document sections (FullDocument, RagContext) are always appended at fixed ranks (11-12)
/// to ensure they're allocated last in the waterfall.
///
/// Old saved data with 9 sections (including documents) is handled gracefully:
/// document sections are stripped from the input and re-appended at fixed ranks.
/// Configurable section ranks are re-numbered to contiguous 4..N to prevent collisions.
pub fn apply_default_bounds(user_priorities: &[SectionPriority]) -> Vec<SectionPriority> {
    let defaults = default_priorities();
    let default_map: HashMap<BudgetSection, (usize, usize)> = defaults
        .iter()
        .map(|d| (d.section, (d.floor, d.ceiling)))
        .collect();

    // 1. Process only configurable sections (filter out any document sections from user input)
    //    and deduplicate by section (keep first occurrence) to handle malformed input.
    let mut seen = HashSet::new();
    let mut result: Vec<SectionPriority> = user_priorities
        .iter()
        .filter(|p| BudgetSection::CONFIGURABLE.contains(&p.section) && seen.insert(p.section))
        .map(|p| {
            let (floor, ceiling) = default_map
                .get(&p.section)
                .copied()
                .unwrap_or((0, 0));
            SectionPriority {
                section: p.section,
                rank: p.rank,
                floor,
                ceiling,
            }
        })
        .collect();

    // Guard: if input doesn't contain all configurable sections, fall back to defaults.
    if result.len() != BudgetSection::CONFIGURABLE.len() {
        return default_priorities();
    }

    // 2. Sort by user-defined rank order, then re-assign contiguous ranks 4..N.
    //    Prevents rank collision with fixed document ranks (11-12) when migrating
    //    old saved data that had WebWikiSearch at rank 11, PositionalMap at rank 12.
    result.sort_by_key(|p| p.rank);
    for (i, p) in result.iter_mut().enumerate() {
        p.rank = (i + 4) as u8;
    }

    // 3. Always append document sections at fixed ranks (after all configurable sections)
    for doc_prio in defaults.iter().filter(|d| BudgetSection::DOCUMENT.contains(&d.section)) {
        result.push(doc_prio.clone());
    }

    result
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Compute the number of tokens reserved for non-negotiable sections
/// (system prompt + deterministic overhead + num_predict).
fn compute_reserved_tokens(params: &BudgetParams, chars_per_token: f64) -> usize {
    let system_prompt_tokens =
        (params.system_prompt_chars as f64 / chars_per_token).ceil() as usize;
    let deterministic_tokens =
        (constants::BUDGET_DETERMINISTIC_OVERHEAD_CHARS as f64 / chars_per_token).ceil() as usize;
    let num_predict_tokens = params.num_predict.max(0) as usize;
    system_prompt_tokens + deterministic_tokens + num_predict_tokens
}

/// Get the chars-per-token ratio for a given language code.
pub fn chars_per_token_for_language(lang: &str) -> f64 {
    match lang {
        "zh" | "ja" | "ko" => constants::CHARS_PER_TOKEN_CJK,
        _ => constants::CHARS_PER_TOKEN_LATIN,
    }
}

/// Whether a section should be included in the budget (based on active features).
fn is_section_active(section: BudgetSection, features: &BudgetFeatures) -> bool {
    match section {
        BudgetSection::WebWikiSearch => features.web_search_enabled || features.wiki_search_enabled,
        BudgetSection::RagContext => features.rag_enabled,
        BudgetSection::FullDocument => features.document_chars > 0,
        // Always active sections:
        _ => true,
    }
}

/// Scale a section's floor/ceiling from the `SectionPriority` to totals for the waterfall.
///
/// Per-message sections (CurrentTurnMessages, ImmediateMemory) store per-message values
/// in `SectionPriority`; this function multiplies by the expected message count.
/// PositionalMap ceiling is per-participant; FullDocument ceiling is the actual document size.
/// All other sections use the priority values as-is.
fn scale_section_bounds(
    section: BudgetSection,
    base_floor: usize,
    base_ceiling: usize,
    n_speakers: usize,
    immediate_memory_turns: usize,
    params: &BudgetParams,
) -> (usize, usize) {
    match section {
        BudgetSection::CurrentTurnMessages => {
            // per_msg × N speakers
            (base_floor * n_speakers, base_ceiling * n_speakers)
        }
        BudgetSection::ImmediateMemory => {
            // per_msg × turns × N speakers
            let n_msgs = immediate_memory_turns * n_speakers;
            (base_floor * n_msgs, base_ceiling * n_msgs)
        }
        BudgetSection::FullDocument => {
            // All-or-nothing: floor from priority (typically 0), ceiling = actual document size.
            (base_floor, params.features.document_chars)
        }
        BudgetSection::PositionalMap => {
            // Floor is flat minimum, ceiling is per-participant.
            (base_floor, base_ceiling * n_speakers)
        }
        // All other sections: priority values are totals, no scaling needed.
        _ => (base_floor, base_ceiling),
    }
}

/// Internal struct for the waterfall algorithm.
struct WaterfallEntry {
    section: BudgetSection,
    rank: u8,
    floor: usize,
    ceiling: usize,
    allocated: usize,
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// All sections in default priority order — test-only helper.
    const ALL_SECTIONS: &[BudgetSection] = &[
        BudgetSection::CurrentTurnMessages,
        BudgetSection::ImmediateMemory,
        BudgetSection::ContextualSummary,
        BudgetSection::CognitiveDirectives,
        BudgetSection::ArbitreDirectives,
        BudgetSection::WebWikiSearch,
        BudgetSection::PositionalMap,
        BudgetSection::FullDocument,
        BudgetSection::RagContext,
    ];

    fn make_params(num_ctx: u32, n_glad: usize, lang: &str) -> BudgetParams {
        BudgetParams {
            num_ctx,
            num_predict: 1024,
            system_prompt_chars: 4_000, // ~1050 tokens for a typical GladIAteur
            n_gladiateurs: n_glad,
            language: lang.to_string(),
            features: BudgetFeatures::default(),
        }
    }

    #[test]
    fn test_budget_generous_32k() {
        let params = make_params(32_768, 3, "fr");
        let priorities = default_priorities();
        let (budget, warnings) = TokenBudget::compute(&params, &priorities);

        assert!(warnings.is_empty(), "No warnings expected: {warnings:?}");
        // All sections should reach or approach their ceilings.
        assert_eq!(budget.current_turn_msg_chars, constants::BUDGET_CEIL_CURRENT_TURN);
        assert_eq!(budget.immediate_memory_msg_chars, constants::BUDGET_CEIL_IMMEDIATE_MEMORY);
        assert_eq!(budget.contextual_summary_chars, constants::BUDGET_CEIL_CONTEXTUAL_SUMMARY);
        assert_eq!(budget.cognitive_directives_chars, constants::BUDGET_CEIL_COGNITIVE_DIRECTIVES);
        assert_eq!(budget.arbitre_directives_chars, constants::BUDGET_CEIL_ARBITRE_DIRECTIVES);
    }

    #[test]
    fn test_budget_tight_4k() {
        let params = make_params(4_096, 3, "fr");
        let priorities = default_priorities();
        let (budget, warnings) = TokenBudget::compute(&params, &priorities);

        // Should have at least one warning about tight budget.
        assert!(
            !warnings.is_empty() || budget.positional_map_chars < 200,
            "Tight budget should produce warnings or reduced allocations"
        );
        // High-priority sections should still get some allocation.
        assert!(budget.current_turn_msg_chars > 0);
        assert!(budget.contextual_summary_chars > 0);
    }

    #[test]
    fn test_budget_medium_8k() {
        let params = make_params(8_192, 2, "fr");
        let priorities = default_priorities();
        let (budget, warnings) = TokenBudget::compute(&params, &priorities);

        // Medium budget — most sections should have meaningful allocations.
        assert!(budget.current_turn_msg_chars >= constants::BUDGET_FLOOR_CURRENT_TURN);
        assert!(budget.contextual_summary_chars >= constants::BUDGET_FLOOR_CONTEXTUAL_SUMMARY);
        // Warnings may or may not be present.
        let _ = warnings;
    }

    #[test]
    fn test_budget_with_document_full_injection() {
        let mut params = make_params(32_768, 2, "fr");
        params.features.document_chars = 5_000; // Small doc — should fit.
        params.features.rag_enabled = true;

        let priorities = default_priorities();
        let (budget, _warnings) = TokenBudget::compute(&params, &priorities);

        assert!(budget.full_document_mode, "Small doc should trigger full injection");
        assert_eq!(budget.rag_context_chars, 0, "RAG should be disabled in full injection mode");
        assert!(budget.full_document_chars >= 5_000);
    }

    #[test]
    fn test_budget_with_document_rag_fallback() {
        let mut params = make_params(4_096, 2, "fr");
        params.features.document_chars = 50_000; // Large doc — won't fit.
        params.features.rag_enabled = true;

        let priorities = default_priorities();
        let (budget, _warnings) = TokenBudget::compute(&params, &priorities);

        assert!(!budget.full_document_mode, "Large doc should NOT trigger full injection");
        // RAG should have some allocation.
        assert!(budget.rag_context_chars > 0);
    }

    #[test]
    fn test_budget_chinese_smaller_chars() {
        let params_fr = make_params(8_192, 2, "fr");
        let params_zh = make_params(8_192, 2, "zh");
        let priorities = default_priorities();

        let (budget_fr, _) = TokenBudget::compute(&params_fr, &priorities);
        let (budget_zh, _) = TokenBudget::compute(&params_zh, &priorities);

        // Chinese should get fewer chars (1.5 chars/token vs 3.8 chars/token).
        assert!(
            budget_zh.contextual_summary_chars < budget_fr.contextual_summary_chars
                || budget_zh.current_turn_msg_chars < budget_fr.current_turn_msg_chars,
            "Chinese budget should have smaller char allocations than French"
        );
    }

    #[test]
    fn test_budget_below_minimum_viable() {
        let params = make_params(1_024, 2, "fr");
        let priorities = default_priorities();
        let (budget, warnings) = TokenBudget::compute(&params, &priorities);

        assert!(!warnings.is_empty(), "Below-minimum should produce warnings");
        // Should return default budget as fallback.
        assert_eq!(budget.current_turn_msg_chars, constants::TRUNC_CURRENT_TURN);
    }

    #[test]
    fn test_budget_with_web_wiki() {
        let mut params = make_params(16_384, 3, "fr");
        params.features.web_search_enabled = true;
        params.features.wiki_search_enabled = true;

        let priorities = default_priorities();
        let (budget, _) = TokenBudget::compute(&params, &priorities);

        assert!(budget.web_wiki_chars > 0, "Web+Wiki should get allocation when enabled");
    }

    #[test]
    fn test_budget_many_speakers_tight() {
        // 5 gladiateurs = 6 speakers total, 4K context — very tight.
        let params = make_params(4_096, 5, "fr");
        let priorities = default_priorities();
        let (budget, warnings) = TokenBudget::compute(&params, &priorities);

        assert!(!warnings.is_empty(), "Many speakers + small ctx should warn");
        // Even under pressure, current turn should get something.
        assert!(budget.current_turn_msg_chars > 0);
    }

    #[test]
    fn test_budget_rag_enabled_without_document_chars() {
        // RAG enabled but document_chars = 0 (Phase 2 scenario: document imported
        // for RAG chunking, but full document injection not yet available).
        let mut params = make_params(16_384, 3, "fr");
        params.features.rag_enabled = true;
        // document_chars stays 0 (default)

        let priorities = default_priorities();
        let (budget, _) = TokenBudget::compute(&params, &priorities);

        assert!(
            budget.rag_context_chars > 0,
            "RAG should get allocation when enabled, even without document_chars"
        );
    }

    #[test]
    fn test_default_priorities_cover_all_sections() {
        let priorities = default_priorities();
        for section in ALL_SECTIONS {
            assert!(
                priorities.iter().any(|p| p.section == *section),
                "Default priorities must cover {section:?}"
            );
        }
    }

    #[test]
    fn test_parse_priorities_valid_json() {
        let defaults = default_priorities();
        let json = serde_json::to_string(&defaults).unwrap();
        let parsed = parse_priorities_or_default(&json);
        assert_eq!(parsed.len(), defaults.len());
        assert_eq!(parsed[0].section, defaults[0].section);
    }

    #[test]
    fn test_parse_priorities_empty_string() {
        let parsed = parse_priorities_or_default("");
        assert_eq!(parsed.len(), default_priorities().len());
    }

    #[test]
    fn test_parse_priorities_invalid_json() {
        let parsed = parse_priorities_or_default("{bad json}");
        assert_eq!(parsed.len(), default_priorities().len());
    }

    #[test]
    fn test_parse_priorities_incomplete() {
        // Only 2 sections instead of all 7 configurable
        let partial = vec![
            SectionPriority {
                section: BudgetSection::CurrentTurnMessages,
                rank: 4,
                floor: 400,
                ceiling: 2000,
            },
            SectionPriority {
                section: BudgetSection::ImmediateMemory,
                rank: 5,
                floor: 300,
                ceiling: 1500,
            },
        ];
        let json = serde_json::to_string(&partial).unwrap();
        let parsed = parse_priorities_or_default(&json);
        // Should fall back to defaults since not all sections are covered
        assert_eq!(parsed.len(), default_priorities().len());
    }

    #[test]
    fn test_preview_matches_budget() {
        let params = make_params(16_384, 3, "en");
        let priorities = default_priorities();
        let preview = TokenBudget::to_preview(&params, &priorities);

        assert_eq!(preview.total_tokens, 16_384);
        assert!(preview.reserved_tokens > 0);
        assert!(preview.available_tokens > 0);
        assert!(!preview.sections.is_empty());
    }

    #[test]
    fn test_apply_default_bounds_fixes_zero_floors_ceilings() {
        // Simulate what the frontend SettingsPage sends: rank order only, floor/ceiling = 0
        let user_priorities: Vec<SectionPriority> = ALL_SECTIONS
            .iter()
            .enumerate()
            .map(|(i, &section)| SectionPriority {
                section,
                rank: (i + 4) as u8,
                floor: 0,
                ceiling: 0,
            })
            .collect();

        let fixed = apply_default_bounds(&user_priorities);

        // All sections should have non-zero floor or ceiling (except FullDocument which has floor=0)
        for p in &fixed {
            match p.section {
                BudgetSection::FullDocument => {
                    assert_eq!(p.floor, constants::BUDGET_FLOOR_FULL_DOCUMENT);
                    // FullDocument ceiling is 0 in defaults (dynamic, set during compute)
                }
                BudgetSection::CurrentTurnMessages => {
                    assert_eq!(p.floor, constants::BUDGET_FLOOR_CURRENT_TURN);
                    assert_eq!(p.ceiling, constants::BUDGET_CEIL_CURRENT_TURN);
                }
                _ => {
                    // At least one of floor/ceiling should be non-zero for active sections
                    assert!(
                        p.floor > 0 || p.ceiling > 0,
                        "Section {:?} should have non-zero bounds after apply_default_bounds",
                        p.section
                    );
                }
            }
        }
    }

    #[test]
    fn test_budget_with_zero_bounds_priorities_still_works() {
        // Ensure that even with frontend-style priorities (floor/ceiling=0),
        // after apply_default_bounds, the budget produces meaningful allocations.
        let params = make_params(16_384, 3, "fr");

        // Simulate frontend priorities with zeros
        let zero_priorities: Vec<SectionPriority> = ALL_SECTIONS
            .iter()
            .enumerate()
            .map(|(i, &section)| SectionPriority {
                section,
                rank: (i + 4) as u8,
                floor: 0,
                ceiling: 0,
            })
            .collect();

        let fixed = apply_default_bounds(&zero_priorities);
        let (budget, warnings) = TokenBudget::compute(&params, &fixed);

        assert!(warnings.is_empty(), "16K context should have no warnings: {warnings:?}");
        assert!(budget.current_turn_msg_chars > 0, "Current turn should get allocation");
        assert!(budget.contextual_summary_chars > 0, "Summary should get allocation");
    }

    #[test]
    fn test_parse_priorities_applies_default_bounds() {
        // Simulate JSON saved by SettingsPage with floor/ceiling=0
        let zero_priorities: Vec<SectionPriority> = ALL_SECTIONS
            .iter()
            .enumerate()
            .map(|(i, &section)| SectionPriority {
                section,
                rank: (i + 4) as u8,
                floor: 0,
                ceiling: 0,
            })
            .collect();
        let json = serde_json::to_string(&zero_priorities).unwrap();
        let parsed = parse_priorities_or_default(&json);

        // Should have non-zero floor for current turn messages
        let ct = parsed.iter().find(|p| p.section == BudgetSection::CurrentTurnMessages).unwrap();
        assert_eq!(ct.floor, constants::BUDGET_FLOOR_CURRENT_TURN);
        assert_eq!(ct.ceiling, constants::BUDGET_CEIL_CURRENT_TURN);
    }

    #[test]
    fn test_apply_default_bounds_appends_document_sections() {
        // Frontend now sends only 7 CONFIGURABLE sections (no documents)
        let user_priorities: Vec<SectionPriority> = BudgetSection::CONFIGURABLE
            .iter()
            .enumerate()
            .map(|(i, &section)| SectionPriority {
                section,
                rank: (i + 4) as u8,
                floor: 0,
                ceiling: 0,
            })
            .collect();

        let fixed = apply_default_bounds(&user_priorities);

        // Result should have all 9 sections (7 configurable + 2 document)
        assert_eq!(fixed.len(), ALL_SECTIONS.len());

        // Document sections should be at fixed ranks 11-12
        let full_doc = fixed.iter().find(|p| p.section == BudgetSection::FullDocument).unwrap();
        assert_eq!(full_doc.rank, 11);
        let rag = fixed.iter().find(|p| p.section == BudgetSection::RagContext).unwrap();
        assert_eq!(rag.rank, 12);

        // Configurable sections should have contiguous ranks 4-10
        for (i, section) in BudgetSection::CONFIGURABLE.iter().enumerate() {
            let p = fixed.iter().find(|p| p.section == *section).unwrap();
            assert_eq!(p.rank, (i + 4) as u8, "Section {section:?} should have rank {}", i + 4);
        }
    }

    #[test]
    fn test_apply_default_bounds_strips_old_document_sections() {
        // Old saved data had 9 sections including FullDocument at rank 9, RagContext at rank 10
        let old_priorities: Vec<SectionPriority> = ALL_SECTIONS
            .iter()
            .enumerate()
            .map(|(i, &section)| SectionPriority {
                section,
                rank: (i + 4) as u8,
                floor: 0,
                ceiling: 0,
            })
            .collect();

        let fixed = apply_default_bounds(&old_priorities);

        // Still 9 sections
        assert_eq!(fixed.len(), ALL_SECTIONS.len());

        // Document sections should be replaced at fixed ranks from defaults
        let full_doc = fixed.iter().find(|p| p.section == BudgetSection::FullDocument).unwrap();
        assert_eq!(full_doc.rank, 11);
        assert_eq!(full_doc.floor, constants::BUDGET_FLOOR_FULL_DOCUMENT);

        let rag = fixed.iter().find(|p| p.section == BudgetSection::RagContext).unwrap();
        assert_eq!(rag.rank, 12);
        assert_eq!(rag.floor, constants::BUDGET_FLOOR_RAG_CONTEXT);
    }

    #[test]
    fn test_apply_default_bounds_renumbers_old_ranks() {
        // Simulate old saved data where WebWikiSearch had rank 11, PositionalMap rank 12
        // These should be renumbered to contiguous 4-10 to avoid collision with doc ranks
        let old_priorities = vec![
            SectionPriority { section: BudgetSection::CurrentTurnMessages, rank: 4, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::ImmediateMemory, rank: 5, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::ContextualSummary, rank: 6, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::CognitiveDirectives, rank: 7, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::ArbitreDirectives, rank: 8, floor: 0, ceiling: 0 },
            // Old positions for these sections
            SectionPriority { section: BudgetSection::WebWikiSearch, rank: 11, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::PositionalMap, rank: 12, floor: 0, ceiling: 0 },
        ];

        let fixed = apply_default_bounds(&old_priorities);

        // Configurable ranks should be contiguous 4-10 (no gaps or collisions)
        let configurable: Vec<_> = fixed.iter()
            .filter(|p| BudgetSection::CONFIGURABLE.contains(&p.section))
            .collect();
        let mut ranks: Vec<u8> = configurable.iter().map(|p| p.rank).collect();
        ranks.sort();
        assert_eq!(ranks, vec![4, 5, 6, 7, 8, 9, 10], "Ranks should be contiguous 4-10");

        // WebWikiSearch should now be rank 9 (was 11), PositionalMap rank 10 (was 12)
        let wws = fixed.iter().find(|p| p.section == BudgetSection::WebWikiSearch).unwrap();
        assert_eq!(wws.rank, 9);
        let pm = fixed.iter().find(|p| p.section == BudgetSection::PositionalMap).unwrap();
        assert_eq!(pm.rank, 10);

        // Document sections at fixed ranks 11-12
        let full_doc = fixed.iter().find(|p| p.section == BudgetSection::FullDocument).unwrap();
        assert_eq!(full_doc.rank, 11);
    }

    #[test]
    fn test_apply_default_bounds_empty_input_returns_defaults() {
        // Empty input should yield full defaults (all 9 sections)
        let fixed = apply_default_bounds(&[]);

        assert_eq!(
            fixed.len(),
            ALL_SECTIONS.len(),
            "Empty input should produce all sections via defaults"
        );
        // Document sections should still be at fixed ranks
        let full_doc = fixed.iter().find(|p| p.section == BudgetSection::FullDocument).unwrap();
        assert_eq!(full_doc.rank, 11);
    }

    #[test]
    fn test_apply_default_bounds_deduplicates_sections() {
        // Duplicate CurrentTurnMessages should be deduplicated (keep first)
        let duped = vec![
            SectionPriority { section: BudgetSection::CurrentTurnMessages, rank: 4, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::CurrentTurnMessages, rank: 5, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::ImmediateMemory, rank: 6, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::ContextualSummary, rank: 7, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::CognitiveDirectives, rank: 8, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::ArbitreDirectives, rank: 9, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::WebWikiSearch, rank: 10, floor: 0, ceiling: 0 },
            SectionPriority { section: BudgetSection::PositionalMap, rank: 11, floor: 0, ceiling: 0 },
        ];

        let fixed = apply_default_bounds(&duped);

        // Should have 9 sections (7 unique configurable + 2 documents), not 10
        assert_eq!(fixed.len(), ALL_SECTIONS.len());
        // CurrentTurnMessages should appear exactly once
        let ct_count = fixed.iter().filter(|p| p.section == BudgetSection::CurrentTurnMessages).count();
        assert_eq!(ct_count, 1, "Duplicate should be deduplicated");
    }

    #[test]
    fn test_preview_has_document_available_tokens() {
        let params = make_params(16_384, 3, "fr");
        let priorities = default_priorities();
        let preview = TokenBudget::to_preview(&params, &priorities);

        // document_available_tokens should be positive (surplus after non-doc allocations)
        assert!(preview.document_available_tokens > 0, "Should have tokens available for documents");
        // And it should not exceed available_tokens
        assert!(
            preview.document_available_tokens <= preview.available_tokens,
            "Document tokens ({}) should not exceed available tokens ({})",
            preview.document_available_tokens,
            preview.available_tokens
        );
    }
}
