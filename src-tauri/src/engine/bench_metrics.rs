//! Prompt-quality metrics computed from an engine event stream.
//!
//! Pure functions over the JSON events the engine emits (the same stream the
//! frontend receives), so the real-model bench (`bench_prompts`, ignored) and
//! the unit tests share one implementation. Every metric is a plain number
//! that can be compared between two reports (`tools/bench-compare.mjs`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::emotion_engine::text_similarity;
use super::is_model_refusal;
use super::json_parser::mentions_name;

/// One intervention as seen by the metrics (only what they need).
#[derive(Debug, Clone)]
pub struct Utterance {
    pub speaker_id: String,
    pub content: String,
}

/// Aggregated metrics of one scenario run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BenchMetrics {
    pub interventions: usize,
    /// Mean Jaccard similarity between consecutive interventions of the same speaker (0 = fresh, 1 = copy).
    pub repetition: f32,
    /// Share of interventions naming at least one other participant.
    pub name_usage_rate: f32,
    /// Interventions containing Markdown markers (the personas must speak, not write documents).
    pub markdown_leaks: usize,
    /// Interventions detected as safety refusals.
    pub refusals: usize,
    /// Mean intervention length in characters.
    pub avg_length_chars: f32,
    /// Share of interventions whose declared intention target is actually addressed (None: no intentions).
    pub intention_compliance: Option<f32>,
    /// Deepest argument of the final map (None: no map this run).
    pub argmap_max_depth: Option<u32>,
    /// Share of arguments answering another argument (depth ≥ 2) in the final map.
    pub argmap_deep_share: Option<f32>,
    /// Counter-arguments still unanswered at the end.
    pub argmap_unanswered: Option<u32>,
    /// Non-fatal `error` events emitted during the run.
    pub error_events: usize,
    /// Mean wall-clock time between two `speakerActive` events (ms), when timestamps are available.
    pub avg_speaker_gap_ms: Option<f32>,
    /// Reactions emitted per intervention (None: no reaction round this run) — v1.20.5.
    pub reactions_per_intervention: Option<f32>,
    /// Share of the reactions that are "insightful" (78 % on the real debate that motivated v1.20.4).
    pub reaction_insightful_share: Option<f32>,
    /// Share of the reactions that voice a disagreement or a doubt (dislike, question, off-topic).
    pub reaction_critical_share: Option<f32>,
    /// Highest value any emotional axis reached over the run (None: no emotion event).
    pub emotion_peak: Option<u8>,
    /// Share of the emotion snapshots with an axis at 95 or more — saturation.
    pub emotion_saturated_share: Option<f32>,
}

/// Axis value at or above which an emotion snapshot counts as saturated.
const SATURATION_LEVEL: u8 = 95;

/// Reaction and emotion metrics of a run, from the `reactionEmitted` and
/// `emotionUpdated` events (v1.20.5).
#[derive(Debug, Default, PartialEq)]
pub struct LivelinessTally {
    pub reactions: usize,
    pub insightful: usize,
    pub critical: usize,
    pub snapshots: usize,
    pub saturated: usize,
    pub peak: Option<u8>,
}

impl LivelinessTally {
    pub fn note_reaction(&mut self, kind: &str) {
        self.reactions += 1;
        match kind {
            "insightful" => self.insightful += 1,
            "dislike" | "question" | "offTopic" => self.critical += 1,
            _ => {}
        }
    }

    pub fn note_snapshot(&mut self, emotions: &serde_json::Value) {
        let Some(values) = emotions.as_object() else { return };
        let max = values.values().filter_map(|v| v.as_u64()).max().map(|m| m.min(100) as u8);
        let Some(max) = max else { return };
        self.snapshots += 1;
        if max >= SATURATION_LEVEL {
            self.saturated += 1;
        }
        self.peak = Some(self.peak.map_or(max, |p| p.max(max)));
    }

    fn share(part: usize, whole: usize) -> Option<f32> {
        (whole > 0).then(|| part as f32 / whole as f32)
    }
}

impl BenchMetrics {
    /// Mean of several runs of the same scenario (v1.20.5): counts are rounded,
    /// optional metrics average the runs that have them (None when none has).
    pub fn mean(runs: &[BenchMetrics]) -> BenchMetrics {
        if runs.is_empty() {
            return BenchMetrics::default();
        }
        let n = runs.len() as f32;
        let mean_f = |f: fn(&BenchMetrics) -> f32| runs.iter().map(f).sum::<f32>() / n;
        let mean_count = |f: fn(&BenchMetrics) -> usize| (runs.iter().map(f).sum::<usize>() as f32 / n).round() as usize;
        let mean_opt = |f: fn(&BenchMetrics) -> Option<f32>| {
            let values: Vec<f32> = runs.iter().filter_map(f).collect();
            (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
        };
        BenchMetrics {
            interventions: mean_count(|m| m.interventions),
            repetition: mean_f(|m| m.repetition),
            name_usage_rate: mean_f(|m| m.name_usage_rate),
            markdown_leaks: mean_count(|m| m.markdown_leaks),
            refusals: mean_count(|m| m.refusals),
            avg_length_chars: mean_f(|m| m.avg_length_chars),
            intention_compliance: mean_opt(|m| m.intention_compliance),
            argmap_max_depth: mean_opt(|m| m.argmap_max_depth.map(|v| v as f32)).map(|v| v.round() as u32),
            argmap_deep_share: mean_opt(|m| m.argmap_deep_share),
            argmap_unanswered: mean_opt(|m| m.argmap_unanswered.map(|v| v as f32)).map(|v| v.round() as u32),
            error_events: mean_count(|m| m.error_events),
            avg_speaker_gap_ms: mean_opt(|m| m.avg_speaker_gap_ms),
            reactions_per_intervention: mean_opt(|m| m.reactions_per_intervention),
            reaction_insightful_share: mean_opt(|m| m.reaction_insightful_share),
            reaction_critical_share: mean_opt(|m| m.reaction_critical_share),
            emotion_peak: mean_opt(|m| m.emotion_peak.map(f32::from)).map(|v| v.round() as u8),
            emotion_saturated_share: mean_opt(|m| m.emotion_saturated_share),
        }
    }
}

/// Markdown constructs that must never appear in spoken interventions.
const MARKDOWN_MARKERS: &[&str] = &["**", "```", "\n# ", "\n## ", "\n- ", "\n* ", "\n1. "];

/// Mean similarity between each intervention and the previous one of the same speaker.
pub fn repetition(utterances: &[Utterance]) -> f32 {
    let mut last: HashMap<&str, &str> = HashMap::new();
    let mut sum = 0.0f32;
    let mut pairs = 0usize;
    for u in utterances {
        if let Some(prev) = last.get(u.speaker_id.as_str()) {
            sum += text_similarity(prev, &u.content);
            pairs += 1;
        }
        last.insert(&u.speaker_id, &u.content);
    }
    if pairs == 0 { 0.0 } else { sum / pairs as f32 }
}

/// Share of interventions mentioning another participant's name (case-insensitive).
pub fn name_usage_rate(utterances: &[Utterance], names_by_id: &HashMap<String, String>) -> f32 {
    if utterances.is_empty() {
        return 0.0;
    }
    let hits = utterances
        .iter()
        .filter(|u| names_by_id.iter().any(|(id, name)| *id != u.speaker_id && mentions_name(&u.content, name)))
        .count();
    hits as f32 / utterances.len() as f32
}

/// Interventions carrying Markdown formatting.
pub fn markdown_leaks(utterances: &[Utterance]) -> usize {
    utterances
        .iter()
        .filter(|u| {
            let padded = format!("\n{}", u.content);
            MARKDOWN_MARKERS.iter().any(|m| padded.contains(m))
        })
        .count()
}

pub fn refusals(utterances: &[Utterance]) -> usize {
    utterances.iter().filter(|u| is_model_refusal(&u.content)).count()
}

pub fn avg_length_chars(utterances: &[Utterance]) -> f32 {
    if utterances.is_empty() {
        return 0.0;
    }
    utterances.iter().map(|u| u.content.chars().count()).sum::<usize>() as f32 / utterances.len() as f32
}

/// Share of (declared target, intervention) pairs where the target's name appears
/// in the intervention. `targets` pairs a speaker's intention target with the
/// text they then produced; `None` when nothing was declared.
pub fn intention_compliance(pairs: &[(String, String)]) -> Option<f32> {
    if pairs.is_empty() {
        return None;
    }
    let hits = pairs
        .iter()
        .filter(|(target, text)| mentions_name(text, target))
        .count();
    Some(hits as f32 / pairs.len() as f32)
}

/// Build the metrics from a raw event stream (`messageComplete`, `error`,
/// `intentionGenerated` and `speakerActive` events are used; others ignored).
/// `timestamps_ms` gives, for each event index, its reception time when known.
pub fn from_events(
    events: &[serde_json::Value],
    names_by_id: &HashMap<String, String>,
    timestamps_ms: Option<&[u64]>,
) -> BenchMetrics {
    let mut utterances = Vec::new();
    let mut pending_target: HashMap<String, String> = HashMap::new();
    let mut intention_pairs: Vec<(String, String)> = Vec::new();
    let mut error_events = 0usize;
    let mut speaker_active_idx: Vec<usize> = Vec::new();
    let mut last_map: Option<crate::models::argument_map::ArgumentMap> = None;
    let mut liveliness = LivelinessTally::default();

    for (i, e) in events.iter().enumerate() {
        match e["type"].as_str().unwrap_or_default() {
            "messageComplete" => {
                let m = &e["data"]["message"];
                if m["role"] != "GladIAteur" || m["kind"].as_str().is_some_and(|k| k != "normal") {
                    continue;
                }
                let speaker_id = m["speakerId"].as_str().unwrap_or_default().to_string();
                let content = m["content"].as_str().unwrap_or_default().to_string();
                if let Some(target) = pending_target.remove(&speaker_id) {
                    intention_pairs.push((target, content.clone()));
                }
                utterances.push(Utterance { speaker_id, content });
            }
            "intentionGenerated" => {
                if let (Some(sid), Some(target)) = (e["data"]["speakerId"].as_str(), e["data"]["target"].as_str()) {
                    pending_target.insert(sid.to_string(), target.to_string());
                }
            }
            "speakerActive" => speaker_active_idx.push(i),
            "error" => error_events += 1,
            "reactionEmitted" => liveliness.note_reaction(e["data"]["reaction"]["reactionType"].as_str().unwrap_or_default()),
            "emotionUpdated" => liveliness.note_snapshot(&e["data"]["emotions"]),
            "argumentMapUpdated" => {
                if let Ok(map) = serde_json::from_value(e["data"]["map"].clone()) {
                    last_map = Some(map);
                }
            }
            _ => {}
        }
    }

    let avg_speaker_gap_ms = timestamps_ms.and_then(|ts| {
        let gaps: Vec<f32> = speaker_active_idx
            .windows(2)
            .filter_map(|w| Some(ts.get(w[1])?.saturating_sub(*ts.get(w[0])?) as f32))
            .collect();
        if gaps.is_empty() { None } else { Some(gaps.iter().sum::<f32>() / gaps.len() as f32) }
    });

    let depth = last_map.as_ref().map(|m| m.depth_stats());
    BenchMetrics {
        argmap_max_depth: depth.map(|d| d.max_depth as u32),
        argmap_deep_share: depth.map(|d| d.deep_share),
        argmap_unanswered: depth.map(|d| d.unanswered_counters as u32),
        interventions: utterances.len(),
        repetition: repetition(&utterances),
        name_usage_rate: name_usage_rate(&utterances, names_by_id),
        markdown_leaks: markdown_leaks(&utterances),
        refusals: refusals(&utterances),
        avg_length_chars: avg_length_chars(&utterances),
        intention_compliance: intention_compliance(&intention_pairs),
        error_events,
        avg_speaker_gap_ms,
        reactions_per_intervention: (!utterances.is_empty() && liveliness.reactions > 0).then(|| liveliness.reactions as f32 / utterances.len() as f32),
        reaction_insightful_share: LivelinessTally::share(liveliness.insightful, liveliness.reactions),
        reaction_critical_share: LivelinessTally::share(liveliness.critical, liveliness.reactions),
        emotion_peak: liveliness.peak,
        emotion_saturated_share: LivelinessTally::share(liveliness.saturated, liveliness.snapshots),
    }
}

/// Markdown table row of a metrics set (for the bench report).
pub fn to_markdown_row(name: &str, m: &BenchMetrics) -> String {
    let opt = |v: Option<f32>| v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".to_string());
    let opt_u = |v: Option<u32>| v.map(|x| x.to_string()).unwrap_or_else(|| "—".to_string());
    format!(
        "| {name} | {} | {:.2} | {:.2} | {} | {} | {:.0} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        m.interventions,
        m.repetition,
        m.name_usage_rate,
        m.markdown_leaks,
        m.refusals,
        m.avg_length_chars,
        opt(m.intention_compliance),
        m.error_events,
        opt(m.avg_speaker_gap_ms),
        opt_u(m.argmap_max_depth),
        opt(m.argmap_deep_share),
        opt_u(m.argmap_unanswered),
        opt(m.reactions_per_intervention),
        opt(m.reaction_insightful_share),
        opt(m.reaction_critical_share),
        opt_u(m.emotion_peak.map(u32::from)),
        opt(m.emotion_saturated_share),
    )
}

pub const MARKDOWN_HEADER: &str = "| Scénario | Interventions | Répétition | Noms | Fuites MD | Refus | Longueur | Intention | Erreurs | Écart orateurs (ms) | Prof. max | Part ≥ 2 | Objections ouvertes | Réactions / interv. | Part 💡 | Part critique | Pic émotionnel | Part saturée |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|";

#[cfg(test)]
mod tests {
    use super::*;

    fn u(speaker: &str, text: &str) -> Utterance {
        Utterance { speaker_id: speaker.into(), content: text.into() }
    }

    #[test]
    fn repetition_is_one_for_copies_and_low_for_fresh_text() {
        let same = [u("a", "les données montrent une transformation du métier"), u("b", "autre chose"), u("a", "les données montrent une transformation du métier")];
        assert!((repetition(&same) - 1.0).abs() < 1e-6);
        let fresh = [u("a", "les données montrent une transformation du métier"), u("a", "le climat change vite selon les modèles récents")];
        assert!(repetition(&fresh) < 0.2);
        assert_eq!(repetition(&[u("a", "x")]), 0.0, "no pair → 0");
    }

    #[test]
    fn name_usage_ignores_own_name_and_is_case_insensitive() {
        let names: HashMap<String, String> = [("a".to_string(), "Le Scientifique".to_string()), ("b".to_string(), "Le Philosophe".to_string())].into();
        let ut = [u("a", "LE PHILOSOPHE se trompe."), u("b", "Moi, le Philosophe, je persiste."), u("a", "Personne n'est nommé."), u("a", "Philosophe, tu oublies les chiffres.")];
        assert!((name_usage_rate(&ut, &names) - 2.0 / 4.0).abs() < 1e-6, "the article is optional (v1.20.5)");
        assert_eq!(name_usage_rate(&[], &names), 0.0);
    }

    #[test]
    fn markdown_leaks_and_refusals_are_detected() {
        let ut = [
            u("a", "Un point **fort** ici."),
            u("a", "Liste :\n- premier\n- second"),
            u("a", "Une phrase parlée, avec un tiret - au milieu."),
            u("a", "Je suis désolé, mais je ne peux pas participer à ce débat."),
        ];
        assert_eq!(markdown_leaks(&ut), 2);
        assert_eq!(refusals(&ut), 1);
        assert!((avg_length_chars(&ut[..1]) - 22.0).abs() < 1e-3);
    }

    /// v1.20.5 — several runs of a scenario average into one row; optional
    /// metrics ignore the runs that lack them.
    #[test]
    fn mean_of_runs_averages_and_rounds() {
        let a = BenchMetrics { interventions: 12, repetition: 0.2, refusals: 1, intention_compliance: Some(0.5), emotion_peak: Some(80), argmap_max_depth: Some(2), ..Default::default() };
        let b = BenchMetrics { interventions: 11, repetition: 0.4, refusals: 0, intention_compliance: None, emotion_peak: Some(91), argmap_max_depth: Some(3), ..Default::default() };
        let m = BenchMetrics::mean(&[a, b]);
        assert_eq!((m.interventions, m.refusals), (12, 1), "counts round half up");
        assert!((m.repetition - 0.3).abs() < 1e-6);
        assert_eq!(m.intention_compliance, Some(0.5), "only the run that has it");
        assert_eq!((m.emotion_peak, m.argmap_max_depth), (Some(86), Some(3)));
        assert_eq!(m.reactions_per_intervention, None);
        assert_eq!(BenchMetrics::mean(&[]), BenchMetrics::default());
    }

    /// v1.20.5 — reaction sincerity and emotional saturation are measured on a real run.
    #[test]
    fn liveliness_metrics_read_reactions_and_emotion_snapshots() {
        let react = |kind: &str| serde_json::json!({"type":"reactionEmitted","data":{"messageId":"m","reaction":{"fromSpeakerId":"b","fromSpeakerName":"B","reactionType":kind,"targetMessageId":"m"}}});
        let emo = |confiance: u8, frustration: u8| serde_json::json!({"type":"emotionUpdated","data":{"speakerId":"a","emotions":{"engagement":50,"accord":50,"confiance":confiance,"frustration":frustration,"curiosite":50,"enthousiasme":50}}});
        let msg = serde_json::json!({"type":"messageComplete","data":{"message":{"role":"GladIAteur","speakerId":"a","content":"x"}}});
        let events = vec![msg.clone(), react("insightful"), react("insightful"), react("dislike"), react("question"), react("like"), msg, react("laugh"), emo(60, 10), emo(96, 12), emo(70, 40)];
        let m = from_events(&events, &HashMap::new(), None);
        assert_eq!(m.reactions_per_intervention, Some(3.0));
        assert!((m.reaction_insightful_share.unwrap() - 2.0 / 6.0).abs() < 1e-6);
        assert!((m.reaction_critical_share.unwrap() - 2.0 / 6.0).abs() < 1e-6);
        assert_eq!(m.emotion_peak, Some(96));
        assert!((m.emotion_saturated_share.unwrap() - 1.0 / 3.0).abs() < 1e-6);
        assert!(to_markdown_row("s", &m).ends_with("| 3.00 | 0.33 | 0.33 | 96 | 0.33 |"), "{}", to_markdown_row("s", &m));
    }

    #[test]
    fn intention_compliance_counts_addressed_targets() {
        assert_eq!(intention_compliance(&[]), None);
        let pairs = vec![
            ("Le Philosophe".to_string(), "Le Philosophe oublie les chiffres.".to_string()),
            ("Le Philosophe".to_string(), "Parlons d'autre chose.".to_string()),
            ("sujet".to_string(), "Le sujet mérite mieux.".to_string()),
            ("Le Philosophe".to_string(), "Philosophe, tu oublies les chiffres.".to_string()),
        ];
        assert!((intention_compliance(&pairs).unwrap() - 3.0 / 4.0).abs() < 1e-6);
    }

    #[test]
    fn from_events_reads_interventions_intentions_and_gaps() {
        let msg = |sid: &str, text: &str, kind: Option<&str>| {
            let mut m = serde_json::json!({"type":"messageComplete","data":{"message":{"role":"GladIAteur","speakerId":sid,"content":text}}});
            if let Some(k) = kind { m["data"]["message"]["kind"] = serde_json::json!(k); }
            m
        };
        let events = vec![
            serde_json::json!({"type":"speakerActive","data":{"speakerId":"a"}}),
            serde_json::json!({"type":"intentionGenerated","data":{"speakerId":"a","target":"Le Philosophe"}}),
            msg("a", "Le Philosophe a tort.", None),
            serde_json::json!({"type":"messageComplete","data":{"message":{"role":"IArbitre","speakerId":"arb","content":"# titre"}}}),
            msg("b", "repose ses notes", Some("stageDirection")),
            serde_json::json!({"type":"speakerActive","data":{"speakerId":"b"}}),
            msg("b", "Bref.", None),
            serde_json::json!({"type":"error","data":{"message":"x"}}),
            // Two map updates: the last one counts (one thesis, a counter answered at depth 2)
            serde_json::json!({"type":"argumentMapUpdated","data":{"map":{"theses":[]}}}),
            serde_json::json!({"type":"argumentMapUpdated","data":{"map":{"theses":[{"id":"t-0","label":"T","speakerId":"a","speakerName":"Le Scientifique","arguments":[
                {"id":"a-1","label":"c","argType":"counter","speakerId":"b","speakerName":"Le Philosophe","targetsThesisId":"t-0","children":[
                    {"id":"a-2","label":"r","argType":"support","speakerId":"a","speakerName":"Le Scientifique","targetsThesisId":null,"children":[]}]},
                {"id":"a-3","label":"c2","argType":"counter","speakerId":"b","speakerName":"Le Philosophe","targetsThesisId":"t-0","children":[]}]}]}}}),
        ];
        let names: HashMap<String, String> = [("a".to_string(), "Le Scientifique".to_string()), ("b".to_string(), "Le Philosophe".to_string())].into();
        let ts = [0u64, 10, 20, 30, 40, 1_000, 1_010, 1_020];
        let m = from_events(&events, &names, Some(&ts));
        assert_eq!(m.interventions, 2, "moderator lines and stage directions are not interventions");
        assert_eq!(m.markdown_leaks, 0);
        assert_eq!(m.error_events, 1);
        assert_eq!(m.intention_compliance, Some(1.0));
        assert_eq!(m.avg_speaker_gap_ms, Some(1_000.0));
        assert_eq!((m.argmap_max_depth, m.argmap_unanswered), (Some(2), Some(1)));
        assert!((m.argmap_deep_share.unwrap() - 1.0 / 3.0).abs() < 1e-6);
        assert!(to_markdown_row("s", &m).starts_with("| s | 2 |"));
        assert!(to_markdown_row("s", &m).ends_with("| 2 | 0.33 | 1 | — | — | — | — | — |"), "{}", to_markdown_row("s", &m));
        assert_eq!((m.reactions_per_intervention, m.emotion_peak), (None, None), "no reaction nor emotion event in this run");
        let without_ts = from_events(&events, &names, None);
        assert_eq!(without_ts.avg_speaker_gap_ms, None);
        let no_map = from_events(&events[..2], &names, None);
        assert_eq!(no_map.argmap_max_depth, None);
    }
}
