//! Merges LLM argument extractions into the accumulated [`ArgumentMap`].
//!
//! The extraction model paraphrases: the same thesis comes back with different
//! wording turn after turn, and counter-arguments reference theses by loosely
//! quoted labels. This module resolves those references with a token-level
//! similarity (accent/case/punctuation-insensitive, stop words removed) so that
//! the map stays compact and nothing is silently lost.

use std::collections::{HashMap, HashSet};

use crate::constants;
use crate::engine::json_parser::{self, ParsedArgument, ParsedArgumentExtraction};
use crate::engine::truncate_at_word_boundary;
use crate::models::argument_map::{ArgumentMap, ArgumentNode, ArgumentType, ThesisNode};

/// What a merge did — surfaced to the frontend and the logs.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MergeReport {
    /// Ids of the theses and arguments created by this merge ("new this turn").
    pub new_node_ids: Vec<String>,
    /// Theses the model presented as new but that matched an existing one.
    pub deduplicated_theses: u32,
    /// Counter-arguments whose target thesis could not be resolved (parked in
    /// the speaker's "unattached" bucket instead of being lost).
    pub unattached: u32,
    /// Arguments/theses discarded because a cap was reached.
    pub dropped: u32,
}

/// Merge extractions into `map`. `speakers` maps display names to ids;
/// extractions for unknown speakers are skipped. `lang` selects the label of
/// the "unattached counter-arguments" bucket.
pub fn merge_extractions(
    map: &mut ArgumentMap,
    extractions: Vec<ParsedArgumentExtraction>,
    speakers: &HashMap<String, String>,
    lang: &str,
) -> MergeReport {
    let mut merger = Merger { map, lang, report: MergeReport::default() };
    for ext in extractions {
        let Some(speaker_id) = speakers.get(&ext.speaker_name).cloned() else {
            tracing::warn!(speaker_name = %ext.speaker_name, "Argument merge: unresolved speaker name, skipping extraction");
            continue;
        };
        for label in &ext.new_theses {
            merger.add_thesis(label, &speaker_id, &ext.speaker_name);
        }
        for arg in &ext.arguments {
            merger.add_argument(arg, &speaker_id, &ext.speaker_name);
        }
    }
    merger.report
}

// ── Label similarity ──────────────────────────────────────────────────

/// Fold the accents the discussion languages commonly use (FR/EN); CJK is kept as is.
fn fold_accent(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' => 'i',
        'ô' | 'ö' | 'ó' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ç' => 'c',
        'ÿ' => 'y',
        'ñ' => 'n',
        other => other,
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x3040..=0x30FF)
}

/// Normalised token set of a label: lower-case, accent-folded, punctuation split,
/// stop words removed. CJK text is tokenised per character (no spaces).
pub fn label_tokens(label: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let mut current = String::new();
    let flush = |current: &mut String, tokens: &mut HashSet<String>| {
        // Elisions ("l", "d") and stop words carry no meaning
        if current.chars().count() > 1 && !constants::ARGMAP_STOP_WORDS.contains(&current.as_str()) {
            tokens.insert(std::mem::take(current));
        }
        current.clear();
    };
    for c in label.chars().flat_map(char::to_lowercase).map(fold_accent) {
        if is_cjk(c) {
            flush(&mut current, &mut tokens);
            tokens.insert(c.to_string());
        } else if c.is_alphanumeric() {
            current.push(c);
        } else {
            flush(&mut current, &mut tokens);
        }
    }
    flush(&mut current, &mut tokens);
    tokens
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = (a.len() + b.len()) as f32 - inter;
    inter / union
}

/// Do two thesis labels designate the same thesis?
/// Jaccard above the threshold, or a long label almost fully contained in the
/// other (a reformulation with extra words). Short labels never match by
/// containment: "AI destroys jobs" is contained in "AI creates more jobs than
/// it destroys" yet says the opposite.
pub fn labels_match(a: &str, b: &str) -> bool {
    let (ta, tb) = (label_tokens(a), label_tokens(b));
    if jaccard(&ta, &tb) >= constants::ARGMAP_THESIS_SIMILARITY_THRESHOLD {
        return true;
    }
    let smaller = ta.len().min(tb.len());
    if smaller < constants::ARGMAP_CONTAINMENT_MIN_TOKENS {
        return false;
    }
    let inter = ta.intersection(&tb).count() as f32;
    inter / smaller as f32 >= constants::ARGMAP_CONTAINMENT_THRESHOLD
}

// ── Merger ────────────────────────────────────────────────────────────

struct Merger<'a> {
    map: &'a mut ArgumentMap,
    lang: &'a str,
    report: MergeReport,
}

impl Merger<'_> {
    fn next_thesis_id(&self) -> String {
        format!("t-{}", self.map.theses.len())
    }

    fn next_argument_id(&self) -> String {
        format!("a-{}", self.map.arguments_count())
    }

    /// Index of the thesis a label refers to: exact/fuzzy label match, then a
    /// 1-based numeric index (models copy list numbers), then the most similar
    /// thesis above the (looser) reference threshold.
    fn resolve_thesis(&self, label: &str) -> Option<usize> {
        if let Some(idx) = self.map.theses.iter().position(|t| labels_match(&t.label, label)) {
            return Some(idx);
        }
        if let Some(n) = label.trim().parse::<usize>().ok().and_then(|n| n.checked_sub(1)) {
            if n < self.map.theses.len() {
                tracing::info!(numeric_ref = n + 1, resolved_label = %self.map.theses[n].label, "Resolved numeric thesis reference");
                return Some(n);
            }
        }
        self.most_similar_thesis(label)
    }

    /// Most similar thesis by Jaccard, only if above the reference threshold.
    fn most_similar_thesis(&self, label: &str) -> Option<usize> {
        let tokens = label_tokens(label);
        self.map
            .theses
            .iter()
            .enumerate()
            .map(|(i, t)| (i, jaccard(&tokens, &label_tokens(&t.label))))
            .filter(|(_, s)| *s >= constants::ARGMAP_REFERENCE_SIMILARITY_THRESHOLD)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// Add a thesis unless an equivalent one exists. Returns its index.
    fn add_thesis(&mut self, label: &str, speaker_id: &str, speaker_name: &str) -> Option<usize> {
        if let Some(idx) = self.map.theses.iter().position(|t| labels_match(&t.label, label)) {
            self.report.deduplicated_theses += 1;
            tracing::info!(label = %label, existing = %self.map.theses[idx].label, "Thesis merged into an existing one");
            return Some(idx);
        }
        if self.map.theses.len() >= constants::ARGMAP_MAX_THESES {
            self.report.dropped += 1;
            tracing::warn!(label = %label, "Thesis cap reached — thesis dropped");
            return None;
        }
        let id = self.next_thesis_id();
        self.report.new_node_ids.push(id.clone());
        self.map.theses.push(ThesisNode {
            id,
            label: truncate_at_word_boundary(label, constants::ARGMAP_MAX_THESIS_LABEL),
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            arguments: Vec::new(),
        });
        Some(self.map.theses.len() - 1)
    }

    /// The speaker's bucket for counter-arguments whose target is unknown.
    fn unattached_bucket(&mut self, speaker_id: &str, speaker_name: &str) -> Option<usize> {
        let bucket_id = format!("t-unattached-{speaker_id}");
        if let Some(idx) = self.map.theses.iter().position(|t| t.id == bucket_id) {
            return Some(idx);
        }
        if self.map.theses.len() >= constants::ARGMAP_MAX_THESES {
            return None;
        }
        let label = match self.lang {
            "en" => "Unattached counter-arguments",
            "zh" => "未关联的反驳",
            _ => "Contre-arguments non rattachés",
        };
        self.report.new_node_ids.push(bucket_id.clone());
        self.map.theses.push(ThesisNode {
            id: bucket_id,
            label: label.to_string(),
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            arguments: Vec::new(),
        });
        Some(self.map.theses.len() - 1)
    }

    /// Where an argument goes: the referenced thesis, an auto-created one for
    /// supports/evidence, the most similar or the bucket for counters, and the
    /// speaker's latest thesis as a last resort.
    fn target_thesis(&mut self, arg: &ParsedArgument, speaker_id: &str, speaker_name: &str) -> Option<usize> {
        let is_counter = arg.arg_type == ArgumentType::Counter;
        let reference = if is_counter { arg.against_thesis.as_deref() } else { arg.for_thesis.as_deref() };
        if let Some(idx) = reference.and_then(|label| self.resolve_thesis(label)) {
            return Some(idx);
        }
        if !is_counter {
            // A support/evidence references the speaker's own (possibly new) thesis
            if let Some(label) = reference.filter(|l| json_parser::is_valid_thesis_label(l)) {
                if let Some(idx) = self.add_thesis(label, speaker_id, speaker_name) {
                    return Some(idx);
                }
            }
            return self.map.theses.iter().rposition(|t| t.speaker_id == speaker_id);
        }
        // Counter-argument against something we cannot identify: never attribute
        // the opposing thesis to the person refuting it — park it visibly.
        self.report.unattached += 1;
        tracing::warn!(reference = ?reference, speaker = %speaker_name, "Counter-argument target unresolved — parked as unattached");
        self.unattached_bucket(speaker_id, speaker_name)
    }

    fn add_argument(&mut self, arg: &ParsedArgument, speaker_id: &str, speaker_name: &str) {
        if self.map.arguments_count() >= constants::ARGMAP_MAX_ARGUMENTS {
            self.report.dropped += 1;
            tracing::warn!(label = %arg.text, "Argument cap reached — argument dropped");
            return;
        }
        let Some(thesis_idx) = self.target_thesis(arg, speaker_id, speaker_name) else {
            self.report.dropped += 1;
            tracing::warn!(label = %arg.text, "No thesis to attach the argument to — dropped");
            return;
        };
        let thesis_id = self.map.theses[thesis_idx].id.clone();
        let node = ArgumentNode {
            id: self.next_argument_id(),
            label: truncate_at_word_boundary(&arg.text, constants::ARGMAP_MAX_ARGUMENT_LABEL),
            arg_type: arg.arg_type.clone(),
            speaker_id: speaker_id.to_string(),
            speaker_name: speaker_name.to_string(),
            targets_thesis_id: (arg.arg_type == ArgumentType::Counter).then_some(thesis_id),
            children: vec![],
        };
        self.report.new_node_ids.push(node.id.clone());

        // Reply to a specific argument → nested under it (within the depth cap)
        if let Some(target) = &arg.targets_argument {
            // The prompt shows truncated labels ending with "…"; models copy them.
            let target_lower = target.trim_end_matches('…').to_lowercase();
            let thesis = &mut self.map.theses[thesis_idx];
            let depth = thesis.arguments.iter().find_map(|a| a.find_depth_by_label(&target_lower, 1));
            match depth {
                Some(d) if d < constants::ARGMAP_MAX_ARGUMENT_DEPTH => {
                    if let Some(parent) = thesis.arguments.iter_mut().find_map(|a| a.find_by_label_mut(&target_lower)) {
                        parent.children.push(node);
                        return;
                    }
                }
                Some(d) => tracing::info!(target = %target, depth = d, "Argument depth cap reached — attaching flat to thesis"),
                None => {}
            }
        }
        self.map.theses[thesis_idx].arguments.push(node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn speakers() -> HashMap<String, String> {
        HashMap::from([
            ("Alice".to_string(), "s1".to_string()),
            ("Bob".to_string(), "s2".to_string()),
        ])
    }

    fn arg(text: &str, arg_type: ArgumentType, for_t: Option<&str>, against: Option<&str>, targets: Option<&str>) -> ParsedArgument {
        ParsedArgument {
            text: text.to_string(),
            arg_type,
            for_thesis: for_t.map(str::to_string),
            against_thesis: against.map(str::to_string),
            targets_argument: targets.map(str::to_string),
        }
    }

    fn ext(speaker: &str, theses: &[&str], arguments: Vec<ParsedArgument>) -> ParsedArgumentExtraction {
        ParsedArgumentExtraction {
            speaker_name: speaker.to_string(),
            new_theses: theses.iter().map(|s| s.to_string()).collect(),
            arguments,
        }
    }

    #[test]
    fn tokens_are_normalised_and_stop_words_removed() {
        let t = label_tokens("L'IA crée plus d'emplois qu'elle n'en détruit !");
        assert!(t.contains("ia") && t.contains("cree") && t.contains("emplois") && t.contains("detruit"));
        assert!(!t.contains("l") && !t.contains("d") && !t.contains("plus") && !t.contains("elle"));
        let zh = label_tokens("人工智能创造就业");
        assert_eq!(zh.len(), 8, "CJK is tokenised per character");
    }

    #[test]
    fn similar_theses_are_merged_but_opposites_are_not() {
        // Reformulation (≥ 70 % token overlap) → same thesis; opposites sharing key words (60 %) are kept apart
        assert!(labels_match(
            "L'IA crée plus d'emplois qu'elle n'en détruit",
            "L'IA crée davantage d'emplois qu'elle n'en détruit"
        ));
        assert!(labels_match("AI creates more jobs than it destroys", "AI creates more jobs than it destroys."));
        // Short label contained in a longer one with opposite meaning → different theses
        assert!(!labels_match("AI destroys jobs", "AI creates more jobs than it destroys"));
        // Long label fully contained in a longer reformulation → containment match (Jaccard 0.5)
        assert!(labels_match(
            "Regulation of algorithms protects democratic institutions",
            "Strict regulation of algorithms clearly protects democratic institutions across Europe and beyond"
        ));
        assert!(!labels_match("", "anything"));
    }

    #[test]
    fn merge_dedups_theses_and_reports_new_nodes() {
        let mut map = ArgumentMap::default();
        let r1 = merge_extractions(
            &mut map,
            vec![ext("Alice", &["L'IA crée plus d'emplois qu'elle n'en détruit"], vec![
                arg("Le secteur tech a créé 3M d'emplois", ArgumentType::Evidence, Some("L'IA crée plus d'emplois qu'elle n'en détruit"), None, None),
            ])],
            &speakers(), "fr",
        );
        assert_eq!(map.theses_count(), 1);
        assert_eq!(map.arguments_count(), 1);
        assert_eq!(r1.new_node_ids, vec!["t-0".to_string(), "a-0".to_string()]);
        assert_eq!(r1.deduplicated_theses, 0);

        // Next turn: the model paraphrases the thesis and Bob counters it by a loose quote
        let r2 = merge_extractions(
            &mut map,
            vec![
                ext("Alice", &["L'IA crée davantage d'emplois qu'elle n'en détruit"], vec![]),
                ext("Bob", &[], vec![arg("Les emplois créés sont précaires", ArgumentType::Counter, None, Some("l'IA crée davantage d'emplois"), None)]),
            ],
            &speakers(), "fr",
        );
        assert_eq!(map.theses_count(), 1, "paraphrase merged: {:?}", map.theses.iter().map(|t| &t.label).collect::<Vec<_>>());
        assert_eq!(r2.deduplicated_theses, 1);
        assert_eq!(map.arguments_count(), 2);
        assert_eq!(r2.new_node_ids, vec!["a-1".to_string()]);
        assert_eq!(map.theses[0].arguments[1].targets_thesis_id.as_deref(), Some("t-0"));
        assert_eq!(r2.unattached, 0);
    }

    #[test]
    fn orphan_counter_arguments_are_parked_not_lost() {
        let mut map = ArgumentMap::default();
        let report = merge_extractions(
            &mut map,
            vec![
                ext("Alice", &["Regulation protects citizens"], vec![]),
                ext("Bob", &[], vec![
                    arg("Markets self-correct faster than laws", ArgumentType::Counter, None, Some("Something nobody said"), None),
                    arg("Counter without any reference", ArgumentType::Counter, None, None, None),
                ]),
            ],
            &speakers(), "en",
        );
        assert_eq!(report.unattached, 2);
        assert_eq!(report.dropped, 0);
        let bucket = map.theses.iter().find(|t| t.id == "t-unattached-s2").expect("bucket created");
        assert_eq!(bucket.label, "Unattached counter-arguments");
        assert_eq!(bucket.speaker_name, "Bob");
        assert_eq!(bucket.arguments.len(), 2);
        // The opposing thesis was NOT attributed to Bob
        assert!(map.theses.iter().all(|t| t.label != "Something nobody said"));
    }

    #[test]
    fn supports_auto_create_their_thesis_and_nest_under_target_argument() {
        let mut map = ArgumentMap::default();
        merge_extractions(
            &mut map,
            vec![ext("Alice", &[], vec![
                arg("Productivity rose 40 % in pilot teams", ArgumentType::Support, Some("AI assistants make developers faster"), None, None),
            ])],
            &speakers(), "en",
        );
        assert_eq!(map.theses_count(), 1);
        assert_eq!(map.theses[0].speaker_name, "Alice");
        let report = merge_extractions(
            &mut map,
            vec![ext("Bob", &[], vec![
                arg("Pilot teams were hand-picked", ArgumentType::Counter, None, Some("AI assistants make developers faster"), Some("Productivity rose 40 % in pilot…")),
            ])],
            &speakers(), "en",
        );
        assert_eq!(report.new_node_ids, vec!["a-1".to_string()]);
        assert_eq!(map.theses[0].arguments.len(), 1, "nested, not flat");
        assert_eq!(map.theses[0].arguments[0].children[0].label, "Pilot teams were hand-picked");
    }

    #[test]
    fn caps_are_enforced_and_counted() {
        let mut map = ArgumentMap::default();
        let theses: Vec<String> = (0..constants::ARGMAP_MAX_THESES + 2)
            .map(|i| format!("Distinct thesis zeta{i} kappa{i} lambda{i} omega{i}"))
            .collect();
        let refs: Vec<&str> = theses.iter().map(String::as_str).collect();
        let report = merge_extractions(&mut map, vec![ext("Alice", &refs, vec![])], &speakers(), "en");
        assert_eq!(map.theses_count(), constants::ARGMAP_MAX_THESES);
        assert_eq!(report.dropped, 2);
        // Unknown speaker → extraction ignored entirely
        let report = merge_extractions(&mut map, vec![ext("Nobody", &["Ghost thesis"], vec![])], &speakers(), "en");
        assert_eq!(report, MergeReport::default());
    }
}
