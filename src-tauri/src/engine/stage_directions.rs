//! Stage directions (v1.17): one theatre line about a participant's state,
//! built without any LLM call from the persona's own `<dynamics>` when it has
//! one, or from trilingual templates. Shown in the feed, never in the prompts.

use crate::constants;
use crate::engine::dynamics_parser::ParsedDynamics;
use crate::engine::truncate_at_sentence_boundary;

/// What just happened to the subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageCue<'a> {
    /// An emotional axis crossed a threshold ("high" | "low")
    Threshold { axis: &'a str, direction: &'a str },
    Banned,
    Returned,
    /// The subject extended a hand to a former rival
    Reconciled { with: &'a str },
}

/// Axes from the most to the least theatrical: when several thresholds are
/// crossed at once, the line is about the first of these.
const AXIS_PRIORITY: [&str; 6] = ["frustration", "engagement", "confiance", "enthousiasme", "curiosite", "accord"];

/// The crossing worth a line among several `(axis, direction, value)`.
pub fn most_dramatic(crossed: &[(String, String, u8)]) -> Option<&(String, String, u8)> {
    crossed.iter().min_by_key(|(axis, _, _)| AXIS_PRIORITY.iter().position(|a| a == axis).unwrap_or(AXIS_PRIORITY.len()))
}

/// The line, bounded by `STAGE_DIRECTION_MAX_CHARS`.
pub fn describe(name: &str, cue: StageCue<'_>, dynamics: Option<&ParsedDynamics>, lang: &str) -> String {
    let from_dynamics = match cue {
        StageCue::Threshold { axis: "frustration", direction: "high" } | StageCue::Banned => dynamics.map(|d| d.under_pressure.as_str()),
        StageCue::Threshold { axis: "engagement", direction: "low" } => dynamics.map(|d| d.disengaged.as_str()),
        StageCue::Threshold { axis: "confiance", direction: "high" } => dynamics.map(|d| d.confident.as_str()),
        StageCue::Threshold { axis: "enthousiasme", direction: "high" } => dynamics.and_then(|d| d.enthusiastic.as_deref()),
        _ => None,
    }
    .map(first_sentence)
    .filter(|s| !s.is_empty());

    let line = match from_dynamics {
        Some(sentence) => format!("{name} — {}", lowercase_first(sentence)),
        None => template(name, cue, lang),
    };
    truncate_at_sentence_boundary(&line, constants::STAGE_DIRECTION_MAX_CHARS)
}

fn first_sentence(text: &str) -> &str {
    let text = text.trim();
    let end = text
        .char_indices()
        .find(|(_, c)| matches!(c, '.' | '!' | '?' | '。' | '！' | '？' | ';'))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(text.len());
    text[..end].trim_end_matches(';').trim()
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn template(name: &str, cue: StageCue<'_>, lang: &str) -> String {
    match (cue, lang) {
        (StageCue::Threshold { axis: "frustration", direction: "high" }, "en") => format!("{name} tenses up, voice turning dry."),
        (StageCue::Threshold { axis: "frustration", direction: "high" }, "zh") => format!("{name}绷紧了，声音变得生硬。"),
        (StageCue::Threshold { axis: "frustration", direction: "high" }, _) => format!("{name} se crispe, la voix plus sèche."),
        (StageCue::Threshold { axis: "frustration", direction: "low" }, "en") => format!("{name} breathes out and relaxes."),
        (StageCue::Threshold { axis: "frustration", direction: "low" }, "zh") => format!("{name}舒了口气，放松下来。"),
        (StageCue::Threshold { axis: "frustration", direction: "low" }, _) => format!("{name} souffle et se détend."),
        (StageCue::Threshold { axis: "engagement", direction: "low" }, "en") => format!("{name} leans back, gaze drifting away."),
        (StageCue::Threshold { axis: "engagement", direction: "low" }, "zh") => format!("{name}向后靠去，目光游离。"),
        (StageCue::Threshold { axis: "engagement", direction: "low" }, _) => format!("{name} se cale dans son siège, le regard ailleurs."),
        (StageCue::Threshold { axis: "engagement", direction: "high" }, "en") => format!("{name} leans in, fully caught by the exchange."),
        (StageCue::Threshold { axis: "engagement", direction: "high" }, "zh") => format!("{name}身体前倾，完全投入交流。"),
        (StageCue::Threshold { axis: "engagement", direction: "high" }, _) => format!("{name} se penche en avant, happé par l'échange."),
        (StageCue::Threshold { axis: "confiance", direction: "high" }, "en") => format!("{name} straightens up, sure of their ground."),
        (StageCue::Threshold { axis: "confiance", direction: "high" }, "zh") => format!("{name}挺直了身子，胸有成竹。"),
        (StageCue::Threshold { axis: "confiance", direction: "high" }, _) => format!("{name} se redresse, sûr de son fait."),
        (StageCue::Threshold { axis: "confiance", direction: "low" }, "en") => format!("{name} hesitates, looking for their words."),
        (StageCue::Threshold { axis: "confiance", direction: "low" }, "zh") => format!("{name}犹豫着，斟酌措辞。"),
        (StageCue::Threshold { axis: "confiance", direction: "low" }, _) => format!("{name} hésite, cherche ses mots."),
        (StageCue::Threshold { axis: "enthousiasme", direction: "high" }, "en") => format!("{name} lights up, gesturing wide."),
        (StageCue::Threshold { axis: "enthousiasme", direction: "high" }, "zh") => format!("{name}眼睛一亮，手势飞扬。"),
        (StageCue::Threshold { axis: "enthousiasme", direction: "high" }, _) => format!("{name} s'illumine, les gestes s'élargissent."),
        (StageCue::Threshold { axis: "enthousiasme", direction: "low" }, "en") => format!("{name} goes flat, the spark gone."),
        (StageCue::Threshold { axis: "enthousiasme", direction: "low" }, "zh") => format!("{name}没了兴致，光彩不再。"),
        (StageCue::Threshold { axis: "enthousiasme", direction: "low" }, _) => format!("{name} s'éteint, la flamme retombée."),
        (StageCue::Threshold { axis: "curiosite", direction: "high" }, "en") => format!("{name} narrows their eyes, intrigued."),
        (StageCue::Threshold { axis: "curiosite", direction: "high" }, "zh") => format!("{name}眯起眼睛，来了兴趣。"),
        (StageCue::Threshold { axis: "curiosite", direction: "high" }, _) => format!("{name} plisse les yeux, intrigué."),
        (StageCue::Threshold { axis: "accord", direction: "low" }, "en") => format!("{name} shakes their head, arms crossed."),
        (StageCue::Threshold { axis: "accord", direction: "low" }, "zh") => format!("{name}摇着头，双臂交叉。"),
        (StageCue::Threshold { axis: "accord", direction: "low" }, _) => format!("{name} secoue la tête, bras croisés."),
        (StageCue::Threshold { axis: "accord", direction: "high" }, "en") => format!("{name} nods along, visibly won over."),
        (StageCue::Threshold { axis: "accord", direction: "high" }, "zh") => format!("{name}连连点头，显然被说服了。"),
        (StageCue::Threshold { axis: "accord", direction: "high" }, _) => format!("{name} acquiesce, visiblement convaincu."),
        (StageCue::Threshold { .. }, "en") => format!("{name} shifts in their seat."),
        (StageCue::Threshold { .. }, "zh") => format!("{name}在座位上动了动。"),
        (StageCue::Threshold { .. }, _) => format!("{name} s'agite sur son siège."),
        (StageCue::Banned, "en") => format!("{name} takes the sanction in silence, jaw set."),
        (StageCue::Banned, "zh") => format!("{name}默默承受处罚，紧咬牙关。"),
        (StageCue::Banned, _) => format!("{name} encaisse la sanction en silence, mâchoire serrée."),
        (StageCue::Returned, "en") => format!("{name} comes back to the table, a point to prove."),
        (StageCue::Returned, "zh") => format!("{name}回到桌前，憋着一股劲。"),
        (StageCue::Returned, _) => format!("{name} revient à la table, quelque chose à prouver."),
        (StageCue::Reconciled { with }, "en") => format!("{name} extends a hand to {with}; the room notices."),
        (StageCue::Reconciled { with }, "zh") => format!("{name}向{with}伸出了手，全场注意到了。"),
        (StageCue::Reconciled { with }, _) => format!("{name} tend la main à {with} ; la salle le remarque."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dynamics() -> ParsedDynamics {
        ParsedDynamics {
            values: String::new(),
            triggers: String::new(),
            under_pressure: "Devient cassant et ironique. Multiplie les questions rhétoriques.".to_string(),
            confident: String::new(),
            disengaged: "  ".to_string(),
            enthusiastic: None,
        }
    }

    #[test]
    fn dynamics_first_sentence_wins_and_templates_cover_every_cue() {
        let d = dynamics();
        let line = describe("Le Juriste", StageCue::Threshold { axis: "frustration", direction: "high" }, Some(&d), "fr");
        assert_eq!(line, "Le Juriste — devient cassant et ironique.");
        // Same field for a ban
        assert_eq!(describe("A", StageCue::Banned, Some(&d), "en"), "A — devient cassant et ironique.");
        // Blank field → template
        assert_eq!(describe("A", StageCue::Threshold { axis: "engagement", direction: "low" }, Some(&d), "en"), "A leans back, gaze drifting away.");
        // No dynamics → templates, every language, every cue
        for lang in ["fr", "en", "zh"] {
            for axis in ["frustration", "engagement", "confiance", "enthousiasme", "curiosite", "accord"] {
                for dir in ["high", "low"] {
                    let l = describe("Ana", StageCue::Threshold { axis, direction: dir }, None, lang);
                    assert!(l.contains("Ana") && l.len() <= constants::STAGE_DIRECTION_MAX_CHARS, "{lang} {axis} {dir}: {l}");
                }
            }
            for cue in [StageCue::Banned, StageCue::Returned, StageCue::Reconciled { with: "Bo" }] {
                let l = describe("Ana", cue, None, lang);
                assert!(l.contains("Ana"), "{lang} {cue:?}: {l}");
            }
        }
        assert!(describe("Ana", StageCue::Reconciled { with: "Bo" }, None, "fr").contains("Bo"));
    }

    #[test]
    fn the_most_dramatic_crossing_wins() {
        let crossed = vec![
            ("accord".to_string(), "low".to_string(), 10),
            ("engagement".to_string(), "low".to_string(), 12),
            ("frustration".to_string(), "high".to_string(), 90),
        ];
        assert_eq!(most_dramatic(&crossed).unwrap().0, "frustration");
        assert_eq!(most_dramatic(&crossed[..2]).unwrap().0, "engagement");
        assert!(most_dramatic(&[]).is_none());
    }

    #[test]
    fn lines_are_bounded_at_a_word() {
        let long = ParsedDynamics { under_pressure: format!("{} fin.", "mot ".repeat(80)), ..dynamics() };
        let l = describe("Ana", StageCue::Banned, Some(&long), "fr");
        assert!(l.len() <= constants::STAGE_DIRECTION_MAX_CHARS + 3, "{}", l.len());
        assert!(l.ends_with('…'));
        // v1.20.4 — a whole persona sentence is kept in full (no more 120-char cuts)
        let sentence = "Il devient cassant, multiplie les questions rhétoriques, cite des chiffres comme des coups de poing et refuse toute concession jusqu'à ce que l'adversaire cède.";
        let full = ParsedDynamics { under_pressure: format!("{sentence} Ensuite il se tait."), ..dynamics() };
        let l = describe("Ana", StageCue::Banned, Some(&full), "fr");
        assert_eq!(l, format!("Ana — {}", lowercase_first(sentence)));
    }
}
