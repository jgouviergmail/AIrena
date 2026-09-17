//! Reaction rounds (v1.17): who reacts to what, with which vocabulary, and how
//! a persona's OCEAN profile shapes their propensity to react.
//!
//! The engine owns the LLM calls; this module holds the pure parts so that the
//! prompt wording and the propensity rules are testable without the engine.

use crate::constants;
use crate::models::discussion::DiscussionMode;
use crate::models::message::{Message, ReactionType};

/// What the reactor is looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionScope {
    /// Every intervention of the previous turn (deferred timing, v1.16)
    PreviousTurn,
    /// The intervention that just ended (immediate timing)
    LastIntervention,
}

/// How often and how harshly a persona reacts (from OCEAN extraversion and agreeableness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionFrequency {
    Rarely,
    Sometimes,
    Often,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionSeverity {
    Lenient,
    Balanced,
    Demanding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactionPropensity {
    pub frequency: ReactionFrequency,
    pub severity: ReactionSeverity,
}

impl ReactionPropensity {
    /// Extraversion drives frequency, agreeableness drives severity; a missing
    /// or middling profile is neutral (no instruction).
    pub fn from_ocean(ocean: Option<[u8; 5]>) -> Option<Self> {
        let [_, _, e, a, _] = ocean?;
        let frequency = if e <= constants::REACTION_PROPENSITY_LOW_E {
            ReactionFrequency::Rarely
        } else if e >= constants::REACTION_PROPENSITY_HIGH_E {
            ReactionFrequency::Often
        } else {
            ReactionFrequency::Sometimes
        };
        let severity = if a >= constants::REACTION_PROPENSITY_HIGH_A {
            ReactionSeverity::Lenient
        } else if a <= constants::REACTION_PROPENSITY_LOW_A {
            ReactionSeverity::Demanding
        } else {
            ReactionSeverity::Balanced
        };
        if frequency == ReactionFrequency::Sometimes && severity == ReactionSeverity::Balanced {
            return None;
        }
        Some(Self { frequency, severity })
    }

    /// One sentence for the reaction prompt.
    pub fn instruction(&self, lang: &str) -> String {
        let freq = match (self.frequency, lang) {
            (ReactionFrequency::Rarely, "en") => "You react rarely: stay silent (\"none\") unless something really strikes you.",
            (ReactionFrequency::Rarely, "zh") => "你很少反应：除非真的触动你，否则保持沉默（\"none\"）。",
            (ReactionFrequency::Rarely, _) => "Tu réagis rarement : reste silencieux (\"none\") sauf si quelque chose te frappe vraiment.",
            (ReactionFrequency::Often, "en") => "You react readily and openly: when you do react, your stance is clear.",
            (ReactionFrequency::Often, "zh") => "你反应积极而直率：一旦反应，态度就要鲜明。",
            (ReactionFrequency::Often, _) => "Tu réagis volontiers et ouvertement : quand tu réagis, ta prise de position est nette.",
            (ReactionFrequency::Sometimes, _) => "",
        };
        let sev = match (self.severity, lang) {
            (ReactionSeverity::Lenient, "en") => "You are lenient: you approve easily (\"like\") and only disapprove a real fault.",
            (ReactionSeverity::Lenient, "zh") => "你很宽容：容易赞同（\"like\"），只对真正的错误表示反对。",
            (ReactionSeverity::Lenient, _) => "Tu es indulgent : tu approuves facilement (\"like\") et ne désapprouves qu'une vraie faute.",
            (ReactionSeverity::Demanding, "en") => "You are demanding: approval must be earned, and a rival only gets it for a genuine concession.",
            (ReactionSeverity::Demanding, "zh") => "你很苛刻：赞同必须靠实力赢得，对手只有真正让步时才会得到。",
            (ReactionSeverity::Demanding, _) => "Tu es exigeant : l'approbation se mérite, et un rival ne l'obtient que pour une vraie concession.",
            (ReactionSeverity::Balanced, _) => "",
        };
        [freq, sev].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ")
    }
}

/// Reaction colours a mode allows (fiction is a relay: no disagreement, no off-topic).
pub fn allowed_reaction_types(mode: &DiscussionMode) -> &'static [ReactionType] {
    match mode {
        DiscussionMode::CollaborativeFiction => &constants::REACTION_TYPES_FICTION,
        _ => &ReactionType::ALL,
    }
}

/// The colour shown in the prompt's example (v1.20.4): never `Insightful` (the
/// anchor that made 💡 the default), rotating over the other colours with the
/// content so the same example never comes back twice in a row.
pub fn example_reaction_kind(allowed: &[ReactionType], seed: usize) -> ReactionType {
    let candidates: Vec<ReactionType> = allowed.iter().copied().filter(|k| *k != ReactionType::Insightful).collect();
    if candidates.is_empty() {
        return ReactionType::Like;
    }
    candidates[seed % candidates.len()]
}

/// Sincerity rules of the reaction prompt (v1.20.4): a reaction is an opinion,
/// not a courtesy; "insightful" is earned; disagreement is said with the
/// colours the mode allows; the justification is the reader's own view.
pub fn sincerity_rules(allowed: &[ReactionType], lang: &str) -> String {
    let dislike = allowed.contains(&ReactionType::Dislike);
    let disagree = match (lang, dislike) {
        ("en", true) | ("zh", true) => "\"dislike\" or \"question\"",
        ("en", false) | ("zh", false) => "\"question\"",
        (_, true) => "\"dislike\" ou \"question\"",
        (_, false) => "\"question\"",
    };
    match lang {
        "en" => format!(
            "Sincerity rules — a reaction is an opinion, not a courtesy:\n\
            - React only when you have a real reason; \"none\" is the normal answer to an intervention that leaves you cold — expect it about one intervention out of two.\n\
            - \"insightful\" must be earned: keep it for a point that changes how you see the topic — at most one intervention out of five, never by default. A well-put but expected point does not deserve it.\n\
            - If you object to a point, or it bothers you, say so: {disagree}, never a hedged compliment.\n\
            - Your justification gives YOUR view in one sentence, first person, with what it changes for you — not a summary of the intervention."
        ),
        "zh" => format!(
            "真诚规则——反应是一种观点，不是客套：\n\
            - 只有在有真实理由时才反应；对让你无动于衷的发言，\"none\" 是正常的回答——大约每两次发言就有一次。\n\
            - \"insightful\" 必须靠实力赢得：只留给改变你看待议题方式的观点——最多五次发言中一次，绝不是默认选项。说得好但在意料之中的观点不配。\n\
            - 如果你不同意或有什么让你不舒服，直说：{disagree}，绝不用委婉的恭维。\n\
            - 你的 justification 用一句话、第一人称给出你自己的看法，说明它对你改变了什么——而不是复述发言。"
        ),
        _ => format!(
            "Règles de sincérité — une réaction est une opinion, pas une politesse :\n\
            - Ne réagis que si tu as une vraie raison ; \"none\" est la réponse normale à une intervention qui ne te fait ni chaud ni froid — environ une intervention sur deux.\n\
            - \"insightful\" se mérite : réserve-le à un point qui change ta façon de voir le sujet — au plus une intervention sur cinq, jamais par défaut. Un point bien dit mais attendu ne le vaut pas.\n\
            - Si tu n'es pas d'accord ou si quelque chose te dérange, dis-le : {disagree}, jamais un compliment nuancé.\n\
            - Ta justification donne TON avis en une phrase, à la première personne, avec ce que ça change pour toi — pas un résumé de l'intervention."
        ),
    }
}

/// Trilingual meaning of each colour for the reaction prompt.
pub fn describe_reaction_type(kind: ReactionType, lang: &str) -> &'static str {
    match (kind, lang) {
        (ReactionType::Insightful, "en") => "a strong point that changes how you see things — rare",
        (ReactionType::Insightful, "zh") => "改变你看法的有力观点——罕见",
        (ReactionType::Insightful, _) => "un point fort qui change ta façon de voir — rare",
        (ReactionType::Question, "en") => "it raises a genuine question you want answered",
        (ReactionType::Question, "zh") => "引发了一个你希望得到解答的真正问题",
        (ReactionType::Question, _) => "cela soulève une vraie question à laquelle tu veux une réponse",
        (ReactionType::OffTopic, "en") => "off topic or derailing the discussion",
        (ReactionType::OffTopic, "zh") => "偏题或使讨论脱轨",
        (ReactionType::OffTopic, _) => "hors sujet ou fait dérailler la discussion",
        (ReactionType::Laugh, "en") => "it made you laugh (wit, irony, a good line)",
        (ReactionType::Laugh, "zh") => "让你发笑（机智、讽刺、妙语）",
        (ReactionType::Laugh, _) => "cela t'a fait rire (esprit, ironie, bon mot)",
        // like / dislike keep the mode-specific meanings (`mode_reaction_meanings`)
        _ => "",
    }
}

/// Reactions of the turn worth showing to the argument extractor: the quotes
/// participants flagged as strong points or questions.
pub fn argument_hints(turn_messages: &[Message], lang: &str) -> Vec<String> {
    let mut hints = Vec::new();
    for m in turn_messages {
        for r in &m.reactions {
            let Some(quote) = r.quote.as_deref() else { continue };
            let line = match (r.reaction_type, lang) {
                (ReactionType::Insightful, "en") => format!("[{} found this strong: \"{}\" — {}]", r.from_speaker_name, quote, m.speaker_name),
                (ReactionType::Insightful, "zh") => format!("[{}认为这一点有力：\"{}\"——{}]", r.from_speaker_name, quote, m.speaker_name),
                (ReactionType::Insightful, _) => format!("[{} a trouvé fort : « {} » — {}]", r.from_speaker_name, quote, m.speaker_name),
                (ReactionType::Question, "en") => format!("[{} questions this: \"{}\" — {}]", r.from_speaker_name, quote, m.speaker_name),
                (ReactionType::Question, "zh") => format!("[{}对此提出疑问：\"{}\"——{}]", r.from_speaker_name, quote, m.speaker_name),
                (ReactionType::Question, _) => format!("[{} questionne : « {} » — {}]", r.from_speaker_name, quote, m.speaker_name),
                _ => continue,
            };
            hints.push(line);
            if hints.len() >= constants::ARGMAP_REACTION_HINTS_MAX {
                return hints;
            }
        }
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::{Reaction, SpeakerRole};

    #[test]
    fn propensity_follows_extraversion_and_agreeableness() {
        assert_eq!(ReactionPropensity::from_ocean(None), None);
        assert_eq!(ReactionPropensity::from_ocean(Some([5, 5, 5, 5, 5])), None, "middling profile is neutral");
        let shy = ReactionPropensity::from_ocean(Some([5, 5, 3, 9, 5])).unwrap();
        assert_eq!((shy.frequency, shy.severity), (ReactionFrequency::Rarely, ReactionSeverity::Lenient));
        assert!(shy.instruction("fr").contains("rarement") && shy.instruction("fr").contains("indulgent"));
        let harsh = ReactionPropensity::from_ocean(Some([5, 5, 8, 2, 5])).unwrap();
        assert_eq!((harsh.frequency, harsh.severity), (ReactionFrequency::Often, ReactionSeverity::Demanding));
        assert!(harsh.instruction("en").contains("demanding"));
        let only_freq = ReactionPropensity::from_ocean(Some([5, 5, 9, 5, 5])).unwrap();
        assert_eq!(only_freq.severity, ReactionSeverity::Balanced);
        assert!(!only_freq.instruction("zh").is_empty());
    }

    #[test]
    fn fiction_restricts_colours_and_hints_keep_flagged_quotes_only() {
        assert_eq!(allowed_reaction_types(&DiscussionMode::CollaborativeFiction), &constants::REACTION_TYPES_FICTION);
        assert_eq!(allowed_reaction_types(&DiscussionMode::Debate).len(), 6);
        assert!(!describe_reaction_type(ReactionType::Laugh, "fr").is_empty());
        assert!(describe_reaction_type(ReactionType::Like, "fr").is_empty(), "like keeps the mode meaning");

        let react = |kind: ReactionType, quote: Option<&str>| Reaction {
            from_speaker_id: "g2".into(), from_speaker_name: "Le Philosophe".into(), reaction_type: kind,
            target_message_id: "m".into(), justification: None, quote: quote.map(String::from),
        };
        let msg = Message {
            id: "m".into(), discussion_id: "d".into(), turn_number: 2, speaker_id: "g1".into(), speaker_name: "Le Scientifique".into(),
            role: SpeakerRole::Gladiateur, content: "c".into(), inner_thought: None, thought_kind: Default::default(),
            reactions: vec![react(ReactionType::Insightful, Some("les données")), react(ReactionType::Like, Some("ignored")), react(ReactionType::Question, None), react(ReactionType::Question, Some("et après ?"))],
            is_ban_notification: false, kind: Default::default(), timestamp: chrono::Utc::now(),
        };
        let hints = argument_hints(std::slice::from_ref(&msg), "fr");
        assert_eq!(hints.len(), 2);
        assert!(hints[0].contains("a trouvé fort : « les données » — Le Scientifique"));
        assert!(hints[1].contains("questionne : « et après ? »"));
        let many: Vec<Message> = (0..5).map(|_| msg.clone()).collect();
        assert_eq!(argument_hints(&many, "en").len(), constants::ARGMAP_REACTION_HINTS_MAX);
    }
}
