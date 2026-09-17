//! Scene events (v1.18): the moderator breaks the routine of a turn — a
//! surprise fact, a format constraint, a question from the room, a forced
//! steelman, a duel or a hot seat. The policy (probability, preconditions) is
//! pure and testable with a seeded RNG; the engine materialises the event
//! (search for the fact, choice of the participants) and applies its effects.

use std::collections::HashSet;

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::constants;
use crate::engine::truncate_at_word_boundary;
use crate::models::discussion::DiscussionMode;

/// Formal constraint imposed on every speaker of the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FormatConstraintKind {
    OneSentence,
    NoJargon,
    Metaphor,
    Numbers,
    EndWithQuestion,
}

impl FormatConstraintKind {
    pub const ALL: [FormatConstraintKind; 5] = [Self::OneSentence, Self::NoJargon, Self::Metaphor, Self::Numbers, Self::EndWithQuestion];
}

/// A materialised event (participant display names).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SceneEvent {
    #[serde(rename_all = "camelCase")]
    SurpriseFact { fact: String, source: Option<String> },
    #[serde(rename_all = "camelCase")]
    FormatConstraint { constraint: FormatConstraintKind },
    /// The room's question, written by the moderator from the exchanges (v1.20.3)
    #[serde(rename_all = "camelCase")]
    AudienceQuestion { target: String, question: String },
    ForcedSteelman,
    #[serde(rename_all = "camelCase")]
    Duel { a: String, b: String },
    #[serde(rename_all = "camelCase")]
    HotSeat { target: String },
    /// A crisis dispatch (crisis cell, v1.19): never drawn, one per turn from the generated list
    #[serde(rename_all = "camelCase")]
    Dispatch { text: String, index: u32, total: u32 },
}

/// Kind drawn by the policy, before the engine materialises it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneEventKind {
    SurpriseFact,
    FormatConstraint,
    AudienceQuestion,
    ForcedSteelman,
    Duel,
    HotSeat,
}

/// What the policy needs to know.
#[derive(Debug, Clone)]
pub struct SceneContext {
    pub mode: DiscussionMode,
    pub turn: u32,
    pub max_turns: Option<u32>,
    pub stop_requested: bool,
    pub last_event_turn: Option<u32>,
    pub stagnating: bool,
    /// A web / wiki / RAG search can still be run this turn
    pub search_available: bool,
    pub audience_enabled: bool,
    /// Active speakers this turn
    pub active_count: usize,
    /// Probability per eligible turn and its boost while stagnating (from `Tuning`, v1.20)
    pub base_probability: f64,
    pub stagnation_boost: f64,
    /// Kinds already played in this discussion: a kind comes back only once every
    /// other available kind has been played (v1.20.3)
    pub used_kinds: HashSet<SceneEventKind>,
}

/// Modes where a scene event makes sense (a relay story or a user-driven
/// session has its own rhythm).
pub fn eligible_mode(mode: &DiscussionMode) -> bool {
    matches!(
        mode,
        DiscussionMode::Debate
            | DiscussionMode::Ideation
            | DiscussionMode::CritiqueReview
            | DiscussionMode::Socratic
            | DiscussionMode::CoConstruction
            | DiscussionMode::Trial
            | DiscussionMode::OxfordDebate
            | DiscussionMode::Negotiation
    )
}

/// Probability of an event this turn (0 when the preconditions fail).
pub fn trigger_probability(ctx: &SceneContext) -> f64 {
    if !eligible_mode(&ctx.mode) || ctx.turn < 2 || ctx.stop_requested || ctx.active_count < 2 {
        return 0.0;
    }
    if ctx.max_turns.is_some_and(|m| ctx.turn >= m) {
        return 0.0;
    }
    if ctx.last_event_turn.is_some_and(|t| ctx.turn.saturating_sub(t) < constants::SCENE_EVENT_MIN_GAP_TURNS) {
        return 0.0;
    }
    let boost = if ctx.stagnating { ctx.stagnation_boost } else { 0.0 };
    (ctx.base_probability + boost).min(1.0)
}

/// Kinds whose preconditions hold, the ones not played yet first: a kind is
/// drawn again only once every other available kind has been played.
pub fn available_kinds(ctx: &SceneContext) -> Vec<SceneEventKind> {
    let mut kinds = vec![SceneEventKind::FormatConstraint, SceneEventKind::ForcedSteelman];
    if ctx.search_available {
        kinds.push(SceneEventKind::SurpriseFact);
    }
    if ctx.audience_enabled {
        kinds.push(SceneEventKind::AudienceQuestion);
    }
    if ctx.active_count >= constants::SCENE_EVENT_MIN_ACTIVE_FOR_DUEL {
        kinds.push(SceneEventKind::Duel);
        kinds.push(SceneEventKind::HotSeat);
    }
    let fresh: Vec<SceneEventKind> = kinds.iter().copied().filter(|k| !ctx.used_kinds.contains(k)).collect();
    if fresh.is_empty() { kinds } else { fresh }
}

/// The policy: gate by probability, then one of the available kinds.
pub fn pick_scene_event(rng: &mut impl Rng, ctx: &SceneContext) -> Option<SceneEventKind> {
    let p = trigger_probability(ctx);
    if p <= 0.0 || !rng.gen_bool(p) {
        return None;
    }
    available_kinds(ctx).choose(rng).copied()
}

pub fn draw_constraint(rng: &mut impl Rng) -> FormatConstraintKind {
    *FormatConstraintKind::ALL.choose(rng).unwrap_or(&FormatConstraintKind::OneSentence)
}

impl SceneEvent {
    /// Display names of the participants the event singles out.
    pub fn participants(&self) -> Vec<String> {
        match self {
            Self::Duel { a, b } => vec![a.clone(), b.clone()],
            Self::HotSeat { target } | Self::AudienceQuestion { target, .. } => vec![target.clone()],
            _ => Vec::new(),
        }
    }

    /// The moderator's announcement pushed into the feed and the history (templated).
    pub fn announcement(&self, lang: &str) -> String {
        match (self, lang) {
            (Self::SurpriseFact { fact, source }, "en") => format!("New element brought to the table: {fact}{} React to it.", source_suffix(source, lang)),
            (Self::SurpriseFact { fact, source }, "zh") => format!("新的事实摆上桌面：{fact}{} 请对此作出回应。", source_suffix(source, lang)),
            (Self::SurpriseFact { fact, source }, _) => format!("Fait nouveau versé au débat : {fact}{} Réagissez-y.", source_suffix(source, lang)),
            (Self::FormatConstraint { constraint }, _) => format!("{} {}", constraint_announcement(lang), constraint_rule(*constraint, lang)),
            (Self::AudienceQuestion { target, question }, "en") => format!("A question from the room for {target}: \"{question}\" Answer it first, then go on."),
            (Self::AudienceQuestion { target, question }, "zh") => format!("现场向{target}提问：\"{question}\" 请先回答，然后继续。"),
            (Self::AudienceQuestion { target, question }, _) => format!("Une question de la salle pour {target} : « {question} » Réponds-y d'abord, puis poursuis."),
            (Self::ForcedSteelman, "en") => "Steelman round: before answering, each of you restates the strongest version of the opposing argument.".to_string(),
            (Self::ForcedSteelman, "zh") => "钢人回合：在回应之前，每位都要先重述反方论点最强的版本。".to_string(),
            (Self::ForcedSteelman, _) => "Tour du steelman : avant de répondre, chacun reformule la version la plus forte de l'argument adverse.".to_string(),
            (Self::Duel { a, b }, "en") => format!("Duel: {a} and {b} have the floor to themselves this turn — settle your disagreement."),
            (Self::Duel { a, b }, "zh") => format!("对决：本轮只有{a}和{b}发言——把你们的分歧说清楚。"),
            (Self::Duel { a, b }, _) => format!("Duel : {a} et {b} ont la parole pour eux seuls ce tour — videz votre désaccord."),
            (Self::HotSeat { target }, "en") => format!("Hot seat: everyone addresses {target}, who answers last."),
            (Self::HotSeat { target }, "zh") => format!("热座：所有人都向{target}发言，由其最后作答。"),
            (Self::HotSeat { target }, _) => format!("Sellette : tout le monde s'adresse à {target}, qui répond en dernier."),
            (Self::Dispatch { text, index, total }, "en") => format!("Dispatch {index}/{total}: {text} The cell must respond."),
            (Self::Dispatch { text, index, total }, "zh") => format!("急电 {index}/{total}：{text} 小组必须作出应对。"),
            (Self::Dispatch { text, index, total }, _) => format!("Dépêche {index}/{total} : {text} La cellule doit réagir."),
        }
    }

    /// Instruction injected into a speaker's prompt (`None` when the speaker is not concerned).
    pub fn speaker_instruction(&self, speaker_name: &str, lang: &str) -> Option<String> {
        let text = match (self, lang) {
            (Self::SurpriseFact { fact, .. }, "en") => format!("Scene: a new element was brought to the table — \"{fact}\". Take a position on it in your intervention."),
            (Self::SurpriseFact { fact, .. }, "zh") => format!("场景：一个新的事实摆上了桌面——“{fact}”。在发言中对此表态。"),
            (Self::SurpriseFact { fact, .. }, _) => format!("Scène : un fait nouveau a été versé au débat — « {fact} ». Prends position dessus dans ton intervention."),
            (Self::FormatConstraint { constraint }, "en") => format!("Scene constraint: {}", constraint_rule(*constraint, lang)),
            (Self::FormatConstraint { constraint }, "zh") => format!("场景限制：{}", constraint_rule(*constraint, lang)),
            (Self::FormatConstraint { constraint }, _) => format!("Contrainte de scène : {}", constraint_rule(*constraint, lang)),
            (Self::AudienceQuestion { target, question }, lang) if target == speaker_name => match lang {
                "en" => format!("Scene: the room asks you — \"{question}\" Answer it first, plainly, then go on."),
                "zh" => format!("场景：现场向你提问——\"{question}\" 先直接回答，然后继续。"),
                _ => format!("Scène : la salle te demande — « {question} » Réponds-y d'abord, sans détour, puis poursuis."),
            },
            (Self::AudienceQuestion { .. }, _) => return None,
            (Self::ForcedSteelman, "en") => "Scene: before your own point, restate the strongest version of the argument you oppose — fairly enough that its author would sign it.".to_string(),
            (Self::ForcedSteelman, "zh") => "场景：在提出自己的观点之前，先重述你所反对论点的最强版本——公允到其作者也会认可。".to_string(),
            (Self::ForcedSteelman, _) => "Scène : avant ton propre point, reformule la version la plus forte de l'argument que tu combats — assez loyalement pour que son auteur la signe.".to_string(),
            (Self::Duel { a, b }, lang) if a == speaker_name || b == speaker_name => {
                let other = if a == speaker_name { b } else { a };
                match lang {
                    "en" => format!("Scene: duel with {other} — you two alone have the floor. Address {other} directly and settle the disagreement."),
                    "zh" => format!("场景：与{other}对决——只有你们两人发言。直接对{other}说话，把分歧说清楚。"),
                    _ => format!("Scène : duel avec {other} — vous seuls avez la parole. Adresse-toi directement à {other} et videz le désaccord."),
                }
            }
            (Self::Duel { .. }, _) => return None,
            (Self::HotSeat { target }, lang) if target == speaker_name => match lang {
                "en" => "Scene: you are in the hot seat — everyone addressed you; answer each of them, briefly, without dodging.".to_string(),
                "zh" => "场景：你坐在热座上——所有人都向你发了言；逐一简短回应，不要回避。".to_string(),
                _ => "Scène : tu es sur la sellette — tout le monde s'est adressé à toi ; réponds à chacun, brièvement, sans esquiver.".to_string(),
            },
            (Self::HotSeat { target }, "en") => format!("Scene: hot seat — address {target} directly, with your sharpest question or objection."),
            (Self::HotSeat { target }, "zh") => format!("场景：热座——直接对{target}说话，提出你最尖锐的问题或异议。"),
            (Self::HotSeat { target }, _) => format!("Scène : sellette — adresse-toi directement à {target}, avec ta question ou ton objection la plus tranchante."),
            (Self::Dispatch { text, .. }, "en") => format!("Scene: a new dispatch just came in — \"{text}\". Say what it changes, decide, and name what you need from the others."),
            (Self::Dispatch { text, .. }, "zh") => format!("场景：刚收到一份新急电——“{text}”。说明它改变了什么，作出决定，并指名你需要其他人做什么。"),
            (Self::Dispatch { text, .. }, _) => format!("Scène : une nouvelle dépêche vient de tomber — « {text} ». Dis ce qu'elle change, décide, et nomme ce que tu attends des autres."),
        };
        Some(truncate_at_word_boundary(&text, constants::SCENE_EVENT_INSTRUCTION_MAX_CHARS))
    }
}

fn source_suffix(source: &Option<String>, lang: &str) -> String {
    match (source, lang) {
        (Some(s), "en") => format!(" (source: {s})."),
        (Some(s), "zh") => format!("（来源：{s}）。"),
        (Some(s), _) => format!(" (source : {s})."),
        (None, _) => String::new(),
    }
}

fn constraint_announcement(lang: &str) -> &'static str {
    match lang {
        "en" => "Constraint for this turn:",
        "zh" => "本轮限制：",
        _ => "Contrainte pour ce tour :",
    }
}

fn constraint_rule(kind: FormatConstraintKind, lang: &str) -> &'static str {
    match (kind, lang) {
        (FormatConstraintKind::OneSentence, "en") => "make your point in ONE sentence.",
        (FormatConstraintKind::OneSentence, "zh") => "用一句话表达你的观点。",
        (FormatConstraintKind::OneSentence, _) => "fais passer ton point en UNE phrase.",
        (FormatConstraintKind::NoJargon, "en") => "no jargon — explain as if to a twelve-year-old.",
        (FormatConstraintKind::NoJargon, "zh") => "不用行话——像对十二岁的孩子那样解释。",
        (FormatConstraintKind::NoJargon, _) => "aucun jargon — explique comme à un enfant de douze ans.",
        (FormatConstraintKind::Metaphor, "en") => "carry your argument with a single metaphor or image.",
        (FormatConstraintKind::Metaphor, "zh") => "用一个比喻或意象承载你的论点。",
        (FormatConstraintKind::Metaphor, _) => "porte ton argument par une seule métaphore ou image.",
        (FormatConstraintKind::Numbers, "en") => "back your point with a figure, an order of magnitude or a date.",
        (FormatConstraintKind::Numbers, "zh") => "用一个数字、数量级或日期支撑你的观点。",
        (FormatConstraintKind::Numbers, _) => "appuie ton point sur un chiffre, un ordre de grandeur ou une date.",
        (FormatConstraintKind::EndWithQuestion, "en") => "end your intervention with a question to a named participant.",
        (FormatConstraintKind::EndWithQuestion, "zh") => "以向某位点名参与者提出的问题结束发言。",
        (FormatConstraintKind::EndWithQuestion, _) => "termine ton intervention par une question à un participant nommé.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn ctx() -> SceneContext {
        SceneContext {
            mode: DiscussionMode::Debate,
            turn: 3,
            max_turns: Some(6),
            stop_requested: false,
            last_event_turn: None,
            stagnating: false,
            search_available: true,
            audience_enabled: true,
            active_count: 3,
            base_probability: constants::SCENE_EVENT_BASE_PROBABILITY,
            stagnation_boost: constants::SCENE_EVENT_STAGNATION_BOOST,
            used_kinds: HashSet::new(),
        }
    }

    /// v1.20.3 — a kind already played is left out until every other available kind
    /// has been played; then the cycle starts again.
    #[test]
    fn played_kinds_wait_until_the_others_were_played() {
        let mut used = HashSet::from([SceneEventKind::AudienceQuestion, SceneEventKind::Duel]);
        let fresh = available_kinds(&SceneContext { used_kinds: used.clone(), ..ctx() });
        assert!(!fresh.contains(&SceneEventKind::AudienceQuestion) && !fresh.contains(&SceneEventKind::Duel) && fresh.len() == 4, "{fresh:?}");
        for k in [SceneEventKind::FormatConstraint, SceneEventKind::ForcedSteelman, SceneEventKind::SurpriseFact, SceneEventKind::HotSeat] {
            used.insert(k);
        }
        assert_eq!(available_kinds(&SceneContext { used_kinds: used, ..ctx() }).len(), 6, "all played: the cycle restarts");
    }

    #[test]
    fn preconditions_gate_the_probability() {
        assert_eq!(trigger_probability(&ctx()), constants::SCENE_EVENT_BASE_PROBABILITY);
        assert_eq!(trigger_probability(&SceneContext { stagnating: true, ..ctx() }), constants::SCENE_EVENT_BASE_PROBABILITY + constants::SCENE_EVENT_STAGNATION_BOOST);
        assert_eq!(trigger_probability(&SceneContext { turn: 1, ..ctx() }), 0.0, "never on turn 1");
        assert_eq!(trigger_probability(&SceneContext { turn: 6, ..ctx() }), 0.0, "never on the last turn");
        assert_eq!(trigger_probability(&SceneContext { stop_requested: true, ..ctx() }), 0.0);
        assert_eq!(trigger_probability(&SceneContext { last_event_turn: Some(2), ..ctx() }), 0.0, "gap of two turns");
        assert!(trigger_probability(&SceneContext { last_event_turn: Some(1), ..ctx() }) > 0.0);
        assert_eq!(trigger_probability(&SceneContext { mode: DiscussionMode::CollaborativeFiction, ..ctx() }), 0.0);
        assert_eq!(trigger_probability(&SceneContext { mode: DiscussionMode::UserDriven, ..ctx() }), 0.0);
        assert_eq!(trigger_probability(&SceneContext { active_count: 1, ..ctx() }), 0.0);
    }

    #[test]
    fn available_kinds_follow_the_preconditions_and_the_draw_is_seeded() {
        let all = available_kinds(&ctx());
        assert_eq!(all.len(), 6);
        let few = available_kinds(&SceneContext { search_available: false, audience_enabled: false, active_count: 2, ..ctx() });
        assert_eq!(few, vec![SceneEventKind::FormatConstraint, SceneEventKind::ForcedSteelman]);
        // Seeded: a stagnating turn (p = 0.5) triggers in a known share of draws
        let mut rng = StdRng::seed_from_u64(7);
        let stagnating = SceneContext { stagnating: true, ..ctx() };
        let hits = (0..400).filter(|_| pick_scene_event(&mut rng, &stagnating).is_some()).count();
        assert!((150..=250).contains(&hits), "{hits}");
        let mut rng = StdRng::seed_from_u64(7);
        let calm_hits = (0..400).filter(|_| pick_scene_event(&mut rng, &ctx()).is_some()).count();
        assert!(calm_hits < hits, "stagnation raises the probability: {calm_hits} < {hits}");
        assert!(pick_scene_event(&mut rng, &SceneContext { turn: 1, ..ctx() }).is_none());
    }

    #[test]
    fn texts_target_the_right_speakers_and_stay_bounded() {
        let duel = SceneEvent::Duel { a: "Ana".into(), b: "Bo".into() };
        assert_eq!(duel.participants(), vec!["Ana", "Bo"]);
        assert!(duel.speaker_instruction("Ana", "fr").unwrap().contains("duel avec Bo"));
        assert!(duel.speaker_instruction("Cy", "fr").is_none());
        let hot = SceneEvent::HotSeat { target: "Ana".into() };
        assert!(hot.speaker_instruction("Ana", "en").unwrap().contains("hot seat"));
        assert!(hot.speaker_instruction("Bo", "en").unwrap().contains("address Ana"));
        let q = SceneEvent::AudienceQuestion { target: "Ana".into(), question: "Et le coût social ?".into() };
        assert!(q.speaker_instruction("Bo", "zh").is_none());
        assert!(q.speaker_instruction("Ana", "fr").unwrap().contains("« Et le coût social ? »"));
        assert!(q.announcement("fr").contains("Une question de la salle pour Ana : « Et le coût social ? »"));
        let fact = SceneEvent::SurpriseFact { fact: "x".repeat(400), source: Some("wikipedia.org".into()) };
        assert!(fact.announcement("fr").contains("(source : wikipedia.org)"));
        assert!(fact.speaker_instruction("Ana", "fr").unwrap().len() <= constants::SCENE_EVENT_INSTRUCTION_MAX_CHARS + 3);
        for lang in ["fr", "en", "zh"] {
            for c in FormatConstraintKind::ALL {
                let e = SceneEvent::FormatConstraint { constraint: c };
                assert!(!e.announcement(lang).is_empty() && e.speaker_instruction("Ana", lang).is_some(), "{c:?} {lang}");
            }
            assert!(!SceneEvent::ForcedSteelman.announcement(lang).is_empty());
        }
        let json = serde_json::to_string(&duel).unwrap();
        assert!(json.contains("\"kind\":\"duel\"") && json.contains("\"a\":\"Ana\""), "{json}");
    }
}
