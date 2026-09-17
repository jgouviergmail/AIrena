//! Dynamic Behavioral Directive builder.
//!
//! Produces a unique, contextual directive for each speaker at each turn.
//! Replaces the static final instruction in `prompt_builder.rs` when `emotion_driven` is enabled.
//!
//! 5 layers:
//!   1. Emotion → Behavior bridge (maps emotions to persona dynamics text)
//!   2. Relationship hints (ally/rival/tense from cumulative reactions)
//!   3. Speech act selection (weighted random from 10 acts, modulated by OCEAN + emotions)
//!   4. Self-memory anti-repetition (inject previous messages)
//!   5. Situational awareness (group mood, turn position, ban return)

use rand::distributions::WeightedIndex;
use rand::prelude::*;
use serde::Serialize;


use super::dynamics_parser::ParsedDynamics;
use super::emotion_engine;
use super::focus::Focus;
use super::mode_prompts;
use super::truncate_str;
use crate::models::relationship::RelationshipKind;
use crate::constants;
use crate::models::discussion::DiscussionMode;
use crate::models::emotion::EmotionalProfile;

// ── Public types ────────────────────────────────────────────────────

/// Full context needed to build a directive for one speaker on one turn.
pub struct SpeakerTurnContext {
    pub emotions: EmotionalProfile,
    /// The persona's initial profile — what the debate did to them is measured from it (v1.20.4)
    pub baseline: EmotionalProfile,
    pub relationships: Vec<RelationshipHint>,
    pub own_previous_messages: Vec<String>,
    pub dynamics: Option<ParsedDynamics>,
    pub ocean: Option<[u8; 5]>,
    pub turn_number: u32,
    pub speakers_this_turn: Vec<String>,
    pub is_first_speaker_this_turn: bool,
    pub was_recently_banned: bool,
    pub group_avg_frustration: u8,
    pub group_avg_engagement: u8,
    pub discussion_language: String,
    pub user_name: String,
    pub discussion_mode: DiscussionMode,
    /// Who to address in priority (turns ≥ 2); None when not applicable.
    pub focus: Option<Focus>,
    /// The speaker's most recent speech acts (newest last), penalised for variety.
    pub recent_speech_acts: Vec<SpeechAct>,
    /// CollaborativeFiction: who wrote the opening this turn (None on turn 1 = write it).
    pub opening_author: Option<String>,
    /// A former rival just approved the speaker (v1.17): invite them to acknowledge it once.
    pub reconciliation_with: Option<String>,
    /// Coalition of the turn (v1.18): the leader relays, the follower extends.
    pub coalition: Option<CoalitionRole>,
    /// The most recent counter the speaker still owes an answer to, as "text (by)" (argument map, v1.20.1).
    pub unanswered_objection: Option<String>,
    /// The audience's message this speaker owes an answer to (excerpt, v1.20.2).
    pub audience_message: Option<String>,
    /// The audience has spoken at least once in this discussion: no longer a mere observer (v1.20.2).
    pub user_has_spoken: bool,
}

/// The speaker's part in a coalition relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoalitionRole {
    /// Speaks first: forced `Relay` act toward `partner`
    Leader { partner: String },
    /// Speaks right after: extends without repeating
    Follower { partner: String },
}

pub struct RelationshipHint {
    pub other_name: String,
    pub kind: RelationshipKind,
    /// Which way a tension leans (v1.20.5): who is the cold one
    pub lean: RelationshipLean,
}

/// Who carries the coldness of a tense pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelationshipLean {
    /// Both directions matter (allies, rivals, one warm and one cold)
    #[default]
    Mutual,
    /// The speaker keeps disapproving the other, who does not return it
    IAmTheCritic,
    /// The other keeps disapproving the speaker, who does not answer
    TheyAreTheCritic,
}

/// Output of the directive builder — injected into the prompt + sent to frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectiveOutput {
    pub directive_text: String,
    pub speech_act: String,
    pub emotion_behavior: Option<String>,
    pub relationship_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechAct {
    Challenge,
    SteelMan,
    Anecdote,
    Question,
    Provocation,
    Concession,
    Redirect,
    Humor,
    Appeal,
    Synthesis,
    /// Depth (v1.20.1): answer the strongest objection on the merits — argumentative modes only
    Deepen,
    /// Coalition (v1.18): hand the argument over to an ally who speaks right after — never drawn, only forced
    Relay,
}

/// Number of speech acts (the weight table and `ALL` share it).
pub const SPEECH_ACT_COUNT: usize = 12;

// Compile-time guarantee: enum discriminants match ALL array indices.
const _: () = assert!(SpeechAct::Challenge as usize == 0);
const _: () = assert!(SpeechAct::Synthesis as usize == 9);
const _: () = assert!(SpeechAct::Deepen as usize == 10);
const _: () = assert!(SpeechAct::Relay as usize == SPEECH_ACT_COUNT - 1);

impl SpeechAct {
    const ALL: [SpeechAct; SPEECH_ACT_COUNT] = [
        SpeechAct::Challenge,
        SpeechAct::SteelMan,
        SpeechAct::Anecdote,
        SpeechAct::Question,
        SpeechAct::Provocation,
        SpeechAct::Concession,
        SpeechAct::Redirect,
        SpeechAct::Humor,
        SpeechAct::Appeal,
        SpeechAct::Synthesis,
        SpeechAct::Deepen,
        SpeechAct::Relay,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            SpeechAct::Challenge => "Challenge",
            SpeechAct::SteelMan => "SteelMan",
            SpeechAct::Anecdote => "Anecdote",
            SpeechAct::Question => "Question",
            SpeechAct::Provocation => "Provocation",
            SpeechAct::Concession => "Concession",
            SpeechAct::Redirect => "Redirect",
            SpeechAct::Humor => "Humor",
            SpeechAct::Appeal => "Appeal",
            SpeechAct::Synthesis => "Synthesis",
            SpeechAct::Deepen => "Deepen",
            SpeechAct::Relay => "Relay",
        }
    }

    /// Parse a speech act name string back into a SpeechAct enum variant.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().find(|a| a.name() == name).copied()
    }

    /// Index into the ALL array. Uses enum discriminant (fieldless enum starts at 0).
    const fn idx(&self) -> usize {
        *self as usize
    }

    fn describe(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (SpeechAct::Challenge, "en") => "Challenge a specific argument head-on — point out its weakness and offer a counter.",
            (SpeechAct::Challenge, "zh") => "正面挑战一个具体论点——指出其弱点并提出反驳。",
            (SpeechAct::Challenge, _) => "Conteste un argument précis de front — pointe sa faiblesse et propose un contre-argument.",

            (SpeechAct::SteelMan, "en") => "Reformulate another participant's point in its strongest form, then respond to THAT version.",
            (SpeechAct::SteelMan, "zh") => "将另一参与者的观点以最强形式重述，然后回应那个版本。",
            (SpeechAct::SteelMan, _) => "Reformule le point d'un autre participant dans sa version la plus forte, puis réponds à CETTE version.",

            (SpeechAct::Anecdote, "en") => "Illustrate your point with a personal story, a vivid example, or a striking analogy.",
            (SpeechAct::Anecdote, "zh") => "用个人故事、生动的例子或引人注目的类比来说明你的观点。",
            (SpeechAct::Anecdote, _) => "Illustre ton propos par une histoire personnelle, un exemple frappant ou une analogie saisissante.",

            (SpeechAct::Question, "en") => "Ask a probing, open question to a specific participant — genuinely explore their reasoning.",
            (SpeechAct::Question, "zh") => "向某个特定参与者提出一个深入的开放性问题——真正探索他们的推理。",
            (SpeechAct::Question, _) => "Pose une question ouverte et incisive à un participant précis — explore sincèrement son raisonnement.",

            (SpeechAct::Provocation, "en") => "Launch a deliberate provocation — a bold, spicy statement designed to shake up the discussion.",
            (SpeechAct::Provocation, "zh") => "发起一次故意的挑衅——一个大胆、辛辣的声明，旨在打破讨论的沉闷。",
            (SpeechAct::Provocation, _) => "Lance une provocation délibérée — une affirmation audacieuse et piquante pour secouer la discussion.",

            (SpeechAct::Concession, "en") => "Admit a point another participant made well — then pivot to show why your view still holds.",
            (SpeechAct::Concession, "zh") => "承认另一参与者提出的一个好观点——然后转向展示为什么你的观点仍然成立。",
            (SpeechAct::Concession, _) => "Admets un point bien formulé par un autre participant — puis pivote pour montrer pourquoi ton point de vue tient toujours.",

            (SpeechAct::Redirect, "en") => "Shift the angle — bring up an aspect of the topic nobody has explored yet.",
            (SpeechAct::Redirect, "zh") => "转换角度——提出一个还没有人探讨过的话题方面。",
            (SpeechAct::Redirect, _) => "Change d'angle — aborde un aspect du sujet que personne n'a encore exploré.",

            (SpeechAct::Humor, "en") => "Defuse tension with humor — a witty remark, a clever comparison, or gentle mockery.",
            (SpeechAct::Humor, "zh") => "用幽默化解紧张——一句机智的话、巧妙的对比或温和的嘲讽。",
            (SpeechAct::Humor, _) => "Désamorce la tension par l'humour — une remarque piquante, une comparaison maligne ou une moquerie bienveillante.",

            (SpeechAct::Appeal, "en") => "Appeal to shared values or emotions — connect your argument to something everyone cares about.",
            (SpeechAct::Appeal, "zh") => "诉诸共同价值观或情感——将你的论点与大家关心的事物联系起来。",
            (SpeechAct::Appeal, _) => "Fais appel aux valeurs ou émotions partagées — relie ton argument à quelque chose qui touche tout le monde.",

            (SpeechAct::Synthesis, "en") => "Synthesize the discussion so far — summarize key positions, then push forward with YOUR evolved stance.",
            (SpeechAct::Synthesis, "zh") => "综合迄今为止的讨论——总结关键立场，然后以你进化的立场推动讨论前进。",
            (SpeechAct::Synthesis, _) => "Synthétise la discussion — résume les positions clés, puis fais avancer avec TA position enrichie.",

            (SpeechAct::Deepen, "en") => "Go deeper — take the strongest objection made to your position and answer it on the merits: a mechanism, a piece of evidence or a precise example, not a restatement.",
            (SpeechAct::Deepen, "zh") => "深入——抓住针对你立场的最有力反驳，就实质作出回应：一个机制、一项证据或一个具体例子，而不是重述。",
            (SpeechAct::Deepen, _) => "Approfondis — reprends l'objection la plus forte faite à ta position et réponds-y sur le fond : un mécanisme, une preuve ou un exemple précis, pas une reformulation.",

            (SpeechAct::Relay, "en") => "Open the argument for your ally who speaks right after you — set the frame, leave them the decisive piece.",
            (SpeechAct::Relay, "zh") => "为紧接着发言的盟友铺开论点——搭好框架，把决定性的一击留给他。",
            (SpeechAct::Relay, _) => "Ouvre l'argument pour ton allié qui parle juste après toi — pose le cadre, laisse-lui la pièce décisive.",
        }
    }

    /// Fiction-specific descriptions for CollaborativeFiction mode.
    /// Remaps debate speech acts to narrative writing actions.
    fn describe_fiction(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (SpeechAct::Challenge, "en") => "Introduce a conflict, obstacle, or unexpected challenge for the characters.",
            (SpeechAct::Challenge, "zh") => "为角色引入冲突、障碍或意想不到的挑战。",
            (SpeechAct::Challenge, _) => "Introduis un conflit, un obstacle ou un défi inattendu pour les personnages.",

            (SpeechAct::SteelMan, "en") => "Develop a character's depth — reveal a new facet of their personality or motivations.",
            (SpeechAct::SteelMan, "zh") => "深化角色——揭示其性格或动机的新面向。",
            (SpeechAct::SteelMan, _) => "Développe la profondeur d'un personnage — révèle une nouvelle facette de sa personnalité ou de ses motivations.",

            (SpeechAct::Anecdote, "en") => "Add a vivid scene, sensory detail, or brief flashback that enriches the narrative.",
            (SpeechAct::Anecdote, "zh") => "添加生动的场景、感官细节或简短的回忆来丰富叙事。",
            (SpeechAct::Anecdote, _) => "Ajoute une scène vivante, un détail sensoriel ou un bref flashback qui enrichit le récit.",

            (SpeechAct::Question, "en") => "Create mystery or suspense — introduce an unanswered question or unknown element in the story.",
            (SpeechAct::Question, "zh") => "制造悬念——在故事中引入未解之谜或未知元素。",
            (SpeechAct::Question, _) => "Crée du mystère ou du suspense — introduis une question sans réponse ou un élément inconnu dans l'histoire.",

            (SpeechAct::Provocation, "en") => "Add a dark twist or shocking revelation that changes the direction of the story.",
            (SpeechAct::Provocation, "zh") => "添加黑暗转折或令人震惊的揭示，改变故事方向。",
            (SpeechAct::Provocation, _) => "Ajoute un rebondissement sombre ou une révélation choquante qui change la direction de l'histoire.",

            (SpeechAct::Concession, "en") => "Slow the pace — add a moment of reflection, calm, or emotional depth for a character.",
            (SpeechAct::Concession, "zh") => "放慢节奏——为角色添加反思、平静或情感深度的时刻。",
            (SpeechAct::Concession, _) => "Ralentis le rythme — ajoute un moment de réflexion, de calme ou de profondeur émotionnelle pour un personnage.",

            (SpeechAct::Redirect, "en") => "Shift the scene — change location, time, or introduce a new character or subplot.",
            (SpeechAct::Redirect, "zh") => "转换场景——改变地点、时间，或引入新角色或支线。",
            (SpeechAct::Redirect, _) => "Change de scène — change de lieu, de temps, ou introduis un nouveau personnage ou une sous-intrigue.",

            (SpeechAct::Humor, "en") => "Add a moment of levity, irony, or dark humor to the narrative.",
            (SpeechAct::Humor, "zh") => "在叙事中加入轻松、讽刺或黑色幽默的时刻。",
            (SpeechAct::Humor, _) => "Ajoute un moment de légèreté, d'ironie ou d'humour noir au récit.",

            (SpeechAct::Appeal, "en") => "Write an emotionally charged passage — build empathy for a character or heighten the drama.",
            (SpeechAct::Appeal, "zh") => "写一段情感充沛的段落——建立对角色的共情或加剧戏剧性。",
            (SpeechAct::Appeal, _) => "Écris un passage chargé d'émotion — crée de l'empathie pour un personnage ou accentue le drame.",

            (SpeechAct::Synthesis, "en") => "Write a transitional passage that ties together narrative threads and propels the story forward.",
            (SpeechAct::Synthesis, "zh") => "写一段过渡段落，将叙事线索联系起来并推动故事向前发展。",
            (SpeechAct::Synthesis, _) => "Écris un passage de transition qui relie les fils narratifs et propulse l'histoire en avant.",

            (SpeechAct::Deepen, "en") => "Dig into a scene already opened instead of opening a new one — give it its consequences.",
            (SpeechAct::Deepen, "zh") => "深挖一个已经展开的场景，而不是开启新场景——写出它的后果。",
            (SpeechAct::Deepen, _) => "Creuse une scène déjà ouverte au lieu d'en ouvrir une nouvelle — donne-lui ses conséquences.",

            (SpeechAct::Relay, "en") => "Set up a scene your co-author will complete right after you — leave the door open.",
            (SpeechAct::Relay, "zh") => "铺设一个由紧接着的合著者完成的场景——留一扇门。",
            (SpeechAct::Relay, _) => "Prépare une scène que ton co-auteur achèvera juste après toi — laisse la porte ouverte.",
        }
    }
}

// ── Main builder ────────────────────────────────────────────────────

/// Build a dynamic behavioral directive for the given speaker context.
/// For turn 1, returns a basic directive with no emotion/relationship context.
/// For turns 2+, applies all 5 layers (emotion, relationships, speech acts, memory, situation).
pub fn build_dynamic_directive(ctx: &SpeakerTurnContext) -> DirectiveOutput {
    let lang = ctx.discussion_language.as_str();
    let mut parts: Vec<String> = Vec::new();

    // Layer 5: Situational awareness (always first)
    parts.push(build_layer5_situation(ctx));

    // Layers 1-4 only for turns 2+
    if ctx.turn_number > 1 {
        // Layer 1: Emotion → Behavior bridge
        let emotion_behavior = build_layer1_emotion_behavior(ctx);

        if let Some(ref behavior) = emotion_behavior {
            parts.push(behavior.clone());
        }

        // Layer 2: Relationship hints
        let relationship_summary = build_layer2_relationships(ctx);
        if !relationship_summary.is_empty() {
            parts.push(relationship_summary.clone());
        }

        // Layer 3: Speech act selection
        let (selected_act, act_text) = build_layer3_speech_act(ctx, &ctx.recent_speech_acts);

        parts.push(act_text);

        // Layer 4: Self-memory anti-repetition
        let self_memory = build_layer4_self_memory(ctx);
        if !self_memory.is_empty() {
            parts.push(self_memory);
        }

        // User reminder (observer until they speak, addressee when they just did, none in fiction)
        parts.push(build_user_reminder(lang, ctx));

        // Mode key constraint — recency bias: last line has the most influence on local LLMs
        let constraint = mode_prompts::mode_key_constraint(&ctx.discussion_mode, lang);
        let remember = match lang {
            "en" => format!("REMEMBER: {}", constraint),
            "zh" => format!("记住：{}", constraint),
            _ => format!("RAPPEL : {}", constraint),
        };
        parts.push(remember);

        let rel_summary_for_ui = build_relationship_summary_for_ui(ctx);

        return DirectiveOutput {
            directive_text: parts.join("\n"),
            speech_act: selected_act.name().to_string(),
            emotion_behavior,
            relationship_summary: rel_summary_for_ui,
        };
    }

    // Turn 1: only Layer 5 + user reminder
    parts.push(build_user_reminder(lang, ctx));

    DirectiveOutput {
        directive_text: parts.join("\n"),
        speech_act: "Opening".to_string(),
        emotion_behavior: None,
        relationship_summary: String::new(),
    }
}

// ── User reminder helper ─────────────────────────────────────────────

/// In UserDriven mode, the user is an active participant → remind the speaker to respond.
/// In CollaborativeFiction, the user is a co-author → no special clause (contributions in history).
/// In all other modes, the user is an observer → remind the speaker NOT to address them.
/// How the speaker treats the audience member: an observer while silent, the
/// first addressee right after they spoke, a participant afterwards (v1.20.2).
fn build_user_reminder(lang: &str, ctx: &SpeakerTurnContext) -> String {
    let user_name = ctx.user_name.as_str();
    match ctx.discussion_mode {
        DiscussionMode::UserDriven => match lang {
            "en" => format!("Respond taking into account {}'s message.", user_name),
            "zh" => format!("在回应中考虑{}的消息。", user_name),
            _ => format!("Réponds en tenant compte du message de {}.", user_name),
        },
        DiscussionMode::CollaborativeFiction => String::new(),
        _ => match (&ctx.audience_message, ctx.user_has_spoken) {
            (Some(message), _) => match lang {
                "en" => format!("{user_name}, from the audience, just spoke: \"{message}\". They are part of the debate now: answer them FIRST, by name — however short or clumsy their words — then carry on."),
                "zh" => format!("现场观众{user_name}刚刚发言：\"{message}\"。他现在是辩论的一部分：先点名回应他——无论其言辞多么简短或笨拙——然后再继续。"),
                _ => format!("{user_name}, dans le public, vient d'intervenir : « {message} ». Il fait partie du débat désormais : réponds-lui D'ABORD, en le nommant — même si son propos est court ou maladroit — puis poursuis."),
            },
            (None, true) => match lang {
                "en" => format!("{user_name}, from the audience, has taken part in the debate: you may answer them like any participant."),
                "zh" => format!("现场观众{user_name}已参与辩论：你可以像回应任何参与者一样回应他。"),
                _ => format!("{user_name}, dans le public, a pris part au débat : tu peux lui répondre comme à tout participant."),
            },
            (None, false) => mode_prompts::user_observer_clause(lang, user_name),
        },
    }
}

/// The opening of an intervention (its first sentence, bounded) — what a speaker
/// must not reproduce twice in a row.
pub fn opening_of(text: &str) -> String {
    let first = text.split_inclusive(['.', '!', '?', '…', '。', '！', '？']).next().unwrap_or(text).trim();
    truncate_str(first, constants::OPENING_EXCERPT_CHARS).to_string()
}

// ── Layer 1: Emotion → Behavior Bridge ──────────────────────────────

/// Up to two emotional states shape the behaviour (priority: frustration,
/// disagreement, disengagement, confidence, curiosity, enthusiasm). When the
/// two are in tension (e.g. frustrated yet enthusiastic) a nuance line asks the
/// persona to let both show.
fn build_layer1_emotion_behavior(ctx: &SpeakerTurnContext) -> Option<String> {
    let emo = &ctx.emotions;
    let lang = ctx.discussion_language.as_str();

    // CollaborativeFiction: emotions influence narrative writing style, not debate behavior
    if ctx.discussion_mode == DiscussionMode::CollaborativeFiction {
        return build_fiction_emotion_behavior(emo, lang);
    }

    let dyn_field = |pick: fn(&ParsedDynamics) -> &str| -> Option<String> {
        ctx.dynamics.as_ref().map(pick).filter(|s| !s.is_empty()).map(str::to_string)
    };

    let mut triggered: Vec<(&'static str, String)> = Vec::new();
    if emo.frustration > constants::PERSONALITY_HIGH {
        triggered.push(("frustrated", match dyn_field(|d| &d.under_pressure) {
            Some(t) => format_behavior(lang, "under_pressure", &t),
            None => generic_behavior(lang, frustration_style(ctx.ocean)),
        }));
    }
    if emo.accord < constants::PERSONALITY_LOW {
        triggered.push(("disagreeing", generic_behavior(lang, "disagreeing")));
    }
    if emo.engagement < constants::PERSONALITY_LOW {
        triggered.push(("disengaged", match dyn_field(|d| &d.disengaged) {
            Some(t) => format_behavior(lang, "disengaged", &t),
            None => generic_behavior(lang, "disengaged"),
        }));
    }
    if emo.confiance > constants::PERSONALITY_HIGH {
        triggered.push(("confident", match dyn_field(|d| &d.confident) {
            Some(t) => format_behavior(lang, "confident", &t),
            None => generic_behavior(lang, "confident"),
        }));
    }
    if emo.curiosite > constants::PERSONALITY_HIGH {
        triggered.push(("curious", match dyn_field(|d| &d.triggers) {
            Some(t) => format_behavior(lang, "curious", &t),
            None => generic_behavior(lang, "curious"),
        }));
    }
    if emo.enthousiasme > constants::PERSONALITY_HIGH {
        triggered.push(("enthusiastic", match ctx.dynamics.as_ref().and_then(|d| d.enthusiastic.clone()).filter(|e| !e.is_empty()) {
            Some(t) => format_behavior(lang, "enthusiastic", &t),
            None => generic_behavior(lang, "enthusiastic"),
        }));
    }

    // v1.20.4 — the movement: the largest notable shift since the start, when its
    // axis is not already spoken for by a state above
    let movement = emotion_engine::dominant_shift(emo, &ctx.baseline)
        .filter(|(axis, _)| !triggered.iter().any(|(key, _)| state_axis(key) == *axis))
        .and_then(|(axis, shift)| movement_line(lang, axis, shift));

    if triggered.is_empty() {
        return movement;
    }
    let dominant: Vec<(&str, String)> = triggered.into_iter().take(2).collect();
    let mut text = dominant.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join(" ");
    let keys: Vec<&str> = dominant.iter().map(|(k, _)| *k).collect();
    let contradictory = matches!(
        (keys.first().copied(), keys.get(1).copied()),
        (Some("frustrated"), Some("enthusiastic")) | (Some("disengaged"), Some("curious")) | (Some("disengaged"), Some("enthusiastic"))
    );
    if contradictory {
        text.push(' ');
        text.push_str(match lang {
            "en" => "These two feelings pull in different directions — let the tension show instead of smoothing it over.",
            "zh" => "这两种情绪相互拉扯——让这种张力自然流露，而不是刻意掩盖。",
            _ => "Ces deux états tirent dans des directions opposées — laisse cette tension transparaître au lieu de la lisser.",
        });
    }
    if let Some(m) = movement {
        text.push(' ');
        text.push_str(&m);
    }
    Some(text)
}

/// The emotion axis a layer-1 state reads.
fn state_axis(key: &str) -> &'static str {
    match key {
        "frustrated" => "frustration",
        "disagreeing" => "accord",
        "disengaged" => "engagement",
        "confident" => "confiance",
        "curious" => "curiosite",
        "enthusiastic" => "enthousiasme",
        _ => "",
    }
}

/// One sentence on what the debate did to the speaker since the start (v1.20.4):
/// a movement on an axis, not a state. Movements without a behavioural reading
/// (more curious, less frustrated…) stay silent.
fn movement_line(lang: &str, axis: &str, shift: i16) -> Option<String> {
    let line = match (axis, shift > 0, lang) {
        ("confiance", false, "en") => "Something in this debate shook you — your confidence has dropped since the start. Show it: concede what hit home or defend it precisely, but no bravado.",
        ("confiance", false, "zh") => "这场辩论中的某些东西动摇了你——你的自信比开始时下降了。表现出来：承认击中你的那一点，或精确地为自己辩护，但不要虚张声势。",
        ("confiance", false, _) => "Quelque chose dans ce débat t'a ébranlé — ta confiance a baissé depuis le début. Montre-le : concède ce qui a porté ou défends-le avec précision, mais sans bravade.",
        ("accord", true, "en") => "You have moved closer to the others since the start — say what you now grant them, in your own words, without giving up your line.",
        ("accord", true, "zh") => "你比开始时更接近其他人了——用你自己的话说出你现在承认他们的哪一点，但不要放弃你的立场。",
        ("accord", true, _) => "Tu t'es rapproché des autres depuis le début — dis ce que tu leur accordes désormais, avec tes mots, sans abandonner ta ligne.",
        ("accord", false, "en") => "You have hardened since the start — the gap has widened and you no longer hide it: name the point of rupture.",
        ("accord", false, "zh") => "你比开始时更强硬了——分歧扩大了，你不再掩饰：说出决裂点。",
        ("accord", false, _) => "Tu t'es durci depuis le début — l'écart s'est creusé et tu ne le caches plus : nomme le point de rupture.",
        ("engagement", true, "en") => "This debate has caught you more than you expected — pull the thread that drew you in.",
        ("engagement", true, "zh") => "这场辩论比你预期的更吸引你——顺着吸引你的那条线索深入下去。",
        ("engagement", true, _) => "Ce débat t'a pris plus que tu ne l'attendais — tire le fil qui t'a accroché.",
        ("enthousiasme", false, "en") => "Your enthusiasm has cooled since the start — fewer flourishes, more precision.",
        ("enthousiasme", false, "zh") => "你的热情比开始时冷却了——少些修饰，多些精确。",
        ("enthousiasme", false, _) => "Ton enthousiasme s'est refroidi depuis le début — moins d'effets, plus de précision.",
        ("frustration", false, "en") => "You have calmed down since the start — you can afford a lighter touch.",
        ("frustration", false, "zh") => "你比开始时平静下来了——你可以更从容一些。",
        ("frustration", false, _) => "Tu t'es apaisé depuis le début — tu peux te permettre plus de légèreté.",
        _ => return None,
    };
    Some(line.to_string())
}

/// Fiction-specific emotion → narrative behavior bridge.
/// Same emotion detection priority, but the output guides writing style, not debate posture.
fn build_fiction_emotion_behavior(emo: &EmotionalProfile, lang: &str) -> Option<String> {
    let behavior: &str = if emo.frustration > constants::PERSONALITY_HIGH {
        match lang {
            "en" => "Channel tension into the narrative — write a conflicted, high-stakes scene.",
            "zh" => "将紧张感注入叙事——写一个充满冲突、高风险的场景。",
            _ => "Canalise la tension dans le récit — écris une scène conflictuelle à fort enjeu.",
        }
    } else if emo.engagement < constants::PERSONALITY_LOW {
        // Low engagement → brief transitional passage, no extra guidance
        return None;
    } else if emo.confiance > constants::PERSONALITY_HIGH {
        match lang {
            "en" => "Write boldly — take a narrative risk, introduce a surprising development.",
            "zh" => "大胆写作——冒叙事风险，引入令人惊讶的发展。",
            _ => "Écris avec audace — prends un risque narratif, introduis un développement surprenant.",
        }
    } else if emo.curiosite > constants::PERSONALITY_HIGH {
        match lang {
            "en" => "Explore the unknown — delve deeper into a mystery or reveal a hidden aspect of the story world.",
            "zh" => "探索未知——深入一个谜团或揭示故事世界的隐藏面。",
            _ => "Explore l'inconnu — plonge plus profondément dans un mystère ou révèle un aspect caché du monde de l'histoire.",
        }
    } else if emo.enthousiasme > constants::PERSONALITY_HIGH {
        match lang {
            "en" => "Write with energy — build toward an exciting, pivotal moment in the story.",
            "zh" => "充满活力地写作——推向故事中激动人心的关键时刻。",
            _ => "Écris avec énergie — construis vers un moment palpitant et décisif de l'histoire.",
        }
    } else {
        return None;
    };

    Some(behavior.to_string())
}

fn format_behavior(lang: &str, emotion_key: &str, dynamics_text: &str) -> String {
    let prefix = match (lang, emotion_key) {
        ("en", "under_pressure") => "You are UNDER PRESSURE right now —",
        ("zh", "under_pressure") => "你现在正承受压力——",
        (_, "under_pressure") => "Tu es SOUS PRESSION en ce moment —",
        ("en", "disengaged") => "You are losing interest —",
        ("zh", "disengaged") => "你正在失去兴趣——",
        (_, "disengaged") => "Tu te DÉSENGAGES —",
        ("en", "confident") => "You feel CONFIDENT right now —",
        ("zh", "confident") => "你现在感到自信——",
        (_, "confident") => "Tu es EN CONFIANCE —",
        ("en", "curious") => "Your curiosity is piqued —",
        ("zh", "curious") => "你的好奇心被激发了——",
        (_, "curious") => "Ta CURIOSITÉ est piquée —",
        ("en", "enthusiastic") => "You are ENTHUSIASTIC —",
        ("zh", "enthusiastic") => "你很热情——",
        (_, "enthusiastic") => "Tu es ENTHOUSIASTE —",
        _ => "",
    };
    format!("{} {}", prefix, dynamics_text)
}

/// How a persona lets frustration out, from agreeableness (v1.17): a very
/// agreeable one turns passive-aggressive, a very disagreeable one goes frontal.
fn frustration_style(ocean: Option<[u8; 5]>) -> &'static str {
    match ocean {
        Some([_, _, _, a, _]) if a >= constants::OCEAN_EXTREME_HIGH => "frustrated_passive",
        Some([_, _, _, a, _]) if a <= constants::OCEAN_EXTREME_LOW => "frustrated_frontal",
        _ => "frustrated",
    }
}

fn generic_behavior(lang: &str, emotion_key: &str) -> String {
    match (lang, emotion_key) {
        ("en", "frustrated") => "You're frustrated — your tone sharpens, your patience thins. Push back harder.".to_string(),
        ("zh", "frustrated") => "你很沮丧——语气更尖锐，耐心更少。更强硬地反击。".to_string(),
        (_, "frustrated") => "Tu es frustré — ton ton se durcit, ta patience s'amenuise. Riposte plus fermement.".to_string(),
        ("en", "frustrated_passive") => "You're frustrated but you hate conflict — it comes out as icy politeness, pointed understatement and a sigh between the lines.".to_string(),
        ("zh", "frustrated_passive") => "你很沮丧却厌恶冲突——它化作冰冷的礼貌、意有所指的轻描淡写和字里行间的叹息。".to_string(),
        (_, "frustrated_passive") => "Tu es frustré mais tu détestes le conflit — cela sort en politesse glaciale, en litotes appuyées et en soupirs entre les lignes.".to_string(),
        ("en", "frustrated_frontal") => "You're frustrated and you don't soften anything — name the problem head-on, short sentences, no diplomatic cushion.".to_string(),
        ("zh", "frustrated_frontal") => "你很沮丧且毫不掩饰——直截了当地指出问题，短句，不留外交余地。".to_string(),
        (_, "frustrated_frontal") => "Tu es frustré et tu n'arrondis rien — nomme le problème de front, phrases courtes, sans coussin diplomatique.".to_string(),
        ("en", "disengaged") => "You're losing interest — respond briefly, maybe with a hint of boredom or irony.".to_string(),
        ("zh", "disengaged") => "你失去兴趣了——简短回应，也许带着一丝厌倦或讽刺。".to_string(),
        (_, "disengaged") => "Tu te désengages — réponds brièvement, peut-être avec une pointe d'ennui ou d'ironie.".to_string(),
        ("en", "confident") => "You're feeling confident — be bolder, more assertive, more generous in your explanations.".to_string(),
        ("zh", "confident") => "你感到自信——更大胆、更自信、更慷慨地解释。".to_string(),
        (_, "confident") => "Tu es en confiance — sois plus audacieux, plus affirmatif, plus généreux dans tes explications.".to_string(),
        ("en", "curious") => "Your curiosity is high — ask deeper questions, explore unexpected angles.".to_string(),
        ("zh", "curious") => "你的好奇心很高——提出更深入的问题，探索意想不到的角度。".to_string(),
        (_, "curious") => "Ta curiosité est élevée — pose des questions plus profondes, explore des angles inattendus.".to_string(),
        ("en", "enthusiastic") => "You're enthusiastic — your energy is infectious, be lively and expressive.".to_string(),
        ("zh", "enthusiastic") => "你很有热情——你的能量有感染力，保持活跃和富有表现力。".to_string(),
        (_, "enthusiastic") => "Tu es enthousiaste — ton énergie est contagieuse, sois vif et expressif.".to_string(),
        ("en", "disagreeing") => "You strongly disagree with the consensus — don't hold back, make your dissent clear.".to_string(),
        ("zh", "disagreeing") => "你强烈不同意共识——不要退缩，明确表达你的异议。".to_string(),
        (_, "disagreeing") => "Tu es en fort désaccord avec le consensus — ne te retiens pas, exprime clairement ta dissidence.".to_string(),
        _ => String::new(),
    }
}

// ── Layer 2: Relationship Hints ─────────────────────────────────────

/// The tense hint, by who carries the coldness (v1.20.5).
fn tense_hint(lang: &str, other: &str, lean: RelationshipLean) -> String {
    match (lean, lang) {
        (RelationshipLean::IAmTheCritic, "en") => format!("You keep disapproving of {other}, who does not return it — own your criticism or acknowledge what they bring, but do not let it become a tic."),
        (RelationshipLean::IAmTheCritic, "zh") => format!("你一直在反对{other}，而对方并未回应——要么坚持你的批评，要么承认对方的贡献，但别让它变成习惯。"),
        (RelationshipLean::IAmTheCritic, _) => format!("Tu n'as cessé de désapprouver {other}, qui ne te rend pas la pareille — assume ta critique ou reconnais ce qu'il apporte, mais n'en fais pas un tic."),
        (RelationshipLean::TheyAreTheCritic, "en") => format!("{other} keeps disapproving of you without an answer from you — answer them or ignore them deliberately, but let it be a choice."),
        (RelationshipLean::TheyAreTheCritic, "zh") => format!("{other}一直在反对你，而你没有回应——回应或有意忽略，但要是一个明确的选择。"),
        (RelationshipLean::TheyAreTheCritic, _) => format!("{other} ne cesse de te désapprouver sans que tu répondes — réponds-lui ou ignore-le délibérément, mais que ce soit un choix."),
        (RelationshipLean::Mutual, "en") => format!("Tension with {other}: the relationship is asymmetric — read the room and adapt."),
        (RelationshipLean::Mutual, "zh") => format!("与{other}关系紧张：关系是不对称的——观察形势并调整。"),
        (RelationshipLean::Mutual, _) => format!("Tension avec {other} : la relation est asymétrique — lis la situation et adapte-toi."),
    }
}

fn build_layer2_relationships(ctx: &SpeakerTurnContext) -> String {
    let lang = ctx.discussion_language.as_str();
    let is_fiction = ctx.discussion_mode == DiscussionMode::CollaborativeFiction;
    let mut hints: Vec<String> = Vec::new();

    for rel in &ctx.relationships {
        let hint = if is_fiction {
            // Fiction: relationships influence narrative collaboration, not debate dynamics
            match (&rel.kind, lang) {
                (RelationshipKind::Ally, "en") => format!(
                    "You and {} have built complementary narrative threads — develop the elements they introduced.",
                    rel.other_name
                ),
                (RelationshipKind::Ally, "zh") => format!(
                    "你和{}构建了互补的叙事线索——发展他们引入的元素。",
                    rel.other_name
                ),
                (RelationshipKind::Ally, _) => format!(
                    "Toi et {} avez construit des fils narratifs complémentaires — développe les éléments qu'il a introduits.",
                    rel.other_name
                ),
                (RelationshipKind::Rival, "en") => format!(
                    "You and {} have been pulling the story in different directions — create narrative tension from this divergence.",
                    rel.other_name
                ),
                (RelationshipKind::Rival, "zh") => format!(
                    "你和{}把故事拉向不同方向——从这种分歧中制造叙事张力。",
                    rel.other_name
                ),
                (RelationshipKind::Rival, _) => format!(
                    "Toi et {} tirez l'histoire dans des directions différentes — crée de la tension narrative à partir de cette divergence.",
                    rel.other_name
                ),
                (RelationshipKind::Tense, "en") => format!(
                    "Unresolved narrative tension with {} — use it to create story suspense or a turning point.",
                    rel.other_name
                ),
                (RelationshipKind::Tense, "zh") => format!(
                    "与{}之间存在未解决的叙事张力——用它制造故事悬念或转折点。",
                    rel.other_name
                ),
                (RelationshipKind::Tense, _) => format!(
                    "Tension narrative non résolue avec {} — utilise-la pour créer du suspense ou un tournant dans l'histoire.",
                    rel.other_name
                ),
            }
        } else {
            match (&rel.kind, lang) {
                (RelationshipKind::Ally, "en") => format!(
                    "You have an ally: {}. You've been supporting each other — build on that.",
                    rel.other_name
                ),
                (RelationshipKind::Ally, "zh") => format!(
                    "你有一个盟友：{}。你们一直在互相支持——在此基础上继续。",
                    rel.other_name
                ),
                (RelationshipKind::Ally, _) => format!(
                    "Tu as un allié : {}. Vous vous soutenez mutuellement — capitalise là-dessus.",
                    rel.other_name
                ),
                (RelationshipKind::Rival, "en") => format!(
                    "You have a rival: {}. Your disagreements are piling up — confront or outmaneuver.",
                    rel.other_name
                ),
                (RelationshipKind::Rival, "zh") => format!(
                    "你有一个对手：{}。你们的分歧在累积——正面交锋或智取。",
                    rel.other_name
                ),
                (RelationshipKind::Rival, _) => format!(
                    "Tu as un rival : {}. Vos désaccords s'accumulent — confronte ou déjoue.",
                    rel.other_name
                ),
                (RelationshipKind::Tense, lang) => tense_hint(lang, &rel.other_name, rel.lean),
            }
        };
        hints.push(hint);
    }

    if let Some(with) = ctx.reconciliation_with.as_deref().filter(|w| !w.is_empty()) {
        hints.push(match lang {
            "en" => format!("{with} just extended a hand to you after your clashes — you may acknowledge it, without giving up your position."),
            "zh" => format!("{with}在你们的冲突之后向你伸出了手——你可以承认这一点，而不放弃自己的立场。"),
            _ => format!("{with} vient de te tendre la main après vos accrochages — tu peux le reconnaître, sans renoncer à ta position."),
        });
    }

    hints.join(" ")
}

fn build_relationship_summary_for_ui(ctx: &SpeakerTurnContext) -> String {
    ctx.relationships
        .iter()
        .map(|rel| {
            let kind_str = match rel.kind {
                RelationshipKind::Ally => "ally",
                RelationshipKind::Rival => "rival",
                RelationshipKind::Tense => "tense",
            };
            format!("{}: {}", rel.other_name, kind_str)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ── Layer 3: Speech Act Selection ───────────────────────────────────

fn build_layer3_speech_act(
    ctx: &SpeakerTurnContext,
    recent_acts: &[SpeechAct],
) -> (SpeechAct, String) {
    let lang = ctx.discussion_language.as_str();
    let is_fiction = ctx.discussion_mode == DiscussionMode::CollaborativeFiction;

    // Coalition (v1.18): the leader's act is forced, the follower gets an extension line
    if let Some(role) = &ctx.coalition {
        return match role {
            CoalitionRole::Leader { partner } => {
                let description = if is_fiction { SpeechAct::Relay.describe_fiction(lang) } else { SpeechAct::Relay.describe(lang) };
                let text = match lang {
                    "en" => format!("For this intervention, favor this approach: {description} Your ally {partner} speaks right after you."),
                    "zh" => format!("在这次发言中，优先采用这种方式：{description} 你的盟友{partner}紧接着发言。"),
                    _ => format!("Pour cette intervention, privilégie cette approche : {description} Ton allié {partner} parle juste après toi."),
                };
                (SpeechAct::Relay, text)
            }
            CoalitionRole::Follower { partner } => {
                let text = match lang {
                    "en" => format!("{partner} just handed the argument over to you: extend it with the decisive piece, without repeating anything they said."),
                    "zh" => format!("{partner}刚把论点交给了你：用决定性的一击延伸它，不要重复他说过的任何内容。"),
                    _ => format!("{partner} vient de te passer le relais : prolonge l'argument avec la pièce décisive, sans rien répéter de ce qu'il a dit."),
                };
                (SpeechAct::SteelMan, text)
            }
        };
    }

    let mut weights = [10u32; SPEECH_ACT_COUNT]; // Base weight: 10 each
    weights[SpeechAct::Relay.idx()] = 0; // only forced by a coalition

    // Mode modifiers — adjust base weights before OCEAN/emotion layers
    match ctx.discussion_mode {
        DiscussionMode::Ideation => {
            weights[SpeechAct::SteelMan.idx()] += 8;
            weights[SpeechAct::Question.idx()] += 6;
            weights[SpeechAct::Redirect.idx()] += 6;
            weights[SpeechAct::Provocation.idx()] = 0;
            weights[SpeechAct::Challenge.idx()] = 2;
        }
        DiscussionMode::CoConstruction => {
            weights[SpeechAct::SteelMan.idx()] += 8;
            weights[SpeechAct::Synthesis.idx()] += 8;
            weights[SpeechAct::Redirect.idx()] += 5;
            weights[SpeechAct::Provocation.idx()] = 0;
            weights[SpeechAct::Challenge.idx()] = 2;
        }
        DiscussionMode::Socratic => {
            weights[SpeechAct::Question.idx()] += 12;
            weights[SpeechAct::Redirect.idx()] += 6;
            weights[SpeechAct::Provocation.idx()] = 0;
            weights[SpeechAct::Anecdote.idx()] = 3;
        }
        DiscussionMode::Tutorial => {
            weights[SpeechAct::Anecdote.idx()] += 8;
            weights[SpeechAct::SteelMan.idx()] += 6;
            weights[SpeechAct::Synthesis.idx()] += 6;
            weights[SpeechAct::Provocation.idx()] = 0;
        }
        DiscussionMode::CritiqueReview => {
            weights[SpeechAct::Challenge.idx()] += 8;
            weights[SpeechAct::SteelMan.idx()] += 6;
            weights[SpeechAct::Synthesis.idx()] += 6;
            weights[SpeechAct::Provocation.idx()] = 0;
        }
        DiscussionMode::CollaborativeFiction => {
            weights[SpeechAct::Anecdote.idx()] += 10;
            weights[SpeechAct::Redirect.idx()] += 8;
            weights[SpeechAct::Challenge.idx()] = 3;
            weights[SpeechAct::Provocation.idx()] = 0;
        }
        DiscussionMode::Trial => {
            weights[SpeechAct::Challenge.idx()] += 8;
            weights[SpeechAct::Question.idx()] += 8;
            weights[SpeechAct::SteelMan.idx()] += 4;
            weights[SpeechAct::Provocation.idx()] = 0;
            weights[SpeechAct::Humor.idx()] = 2;
        }
        DiscussionMode::OxfordDebate => {
            weights[SpeechAct::Challenge.idx()] += 8;
            weights[SpeechAct::Appeal.idx()] += 8;
            weights[SpeechAct::Concession.idx()] = 2;
        }
        DiscussionMode::Negotiation => {
            weights[SpeechAct::Concession.idx()] += 8;
            weights[SpeechAct::Question.idx()] += 6;
            weights[SpeechAct::Synthesis.idx()] += 4;
            weights[SpeechAct::Provocation.idx()] = 0;
        }
        DiscussionMode::SixHats => {
            weights[SpeechAct::Redirect.idx()] += 6;
            weights[SpeechAct::Synthesis.idx()] += 4;
            weights[SpeechAct::Provocation.idx()] = 0;
            weights[SpeechAct::Challenge.idx()] = 3;
        }
        DiscussionMode::CrisisCell => {
            weights[SpeechAct::Redirect.idx()] += 8;
            weights[SpeechAct::Synthesis.idx()] += 6;
            weights[SpeechAct::Anecdote.idx()] = 2;
            weights[SpeechAct::Humor.idx()] = 0;
        }
        // Debate and UserDriven: default weights (no modification)
        DiscussionMode::Debate | DiscussionMode::UserDriven => {}
    }

    // Depth (v1.20.1): answering objections is an act of the argumentative modes only,
    // pressed when the speaker owes an answer — never in ideation, fiction or hats
    if ctx.discussion_mode.rewards_depth() {
        weights[SpeechAct::Deepen.idx()] += if ctx.unanswered_objection.is_some() { constants::SPEECH_ACT_DEEPEN_OWED_BONUS } else { constants::SPEECH_ACT_DEEPEN_BONUS };
    } else {
        weights[SpeechAct::Deepen.idx()] = 0;
    }

    // OCEAN modifiers
    if let Some([o, c, e, a, n]) = ctx.ocean {
        if e >= 7 {
            weights[SpeechAct::Provocation.idx()] += 5;
            weights[SpeechAct::Humor.idx()] += 5;
        }
        if a >= 7 {
            weights[SpeechAct::SteelMan.idx()] += 5;
            weights[SpeechAct::Concession.idx()] += 5;
        }
        if o >= 7 {
            weights[SpeechAct::Question.idx()] += 5;
            weights[SpeechAct::Redirect.idx()] += 5;
        }
        if n >= 7 {
            weights[SpeechAct::Appeal.idx()] += 5;
            weights[SpeechAct::Anecdote.idx()] += 3;
        }
        if c >= 7 {
            weights[SpeechAct::Synthesis.idx()] += 5;
            weights[SpeechAct::Challenge.idx()] += 3;
        }
    }

    // Emotion modifiers
    let emo = &ctx.emotions;
    if emo.frustration > constants::PERSONALITY_HIGH {
        weights[SpeechAct::Challenge.idx()] += 8;
        weights[SpeechAct::Provocation.idx()] += 5;
    }
    if emo.confiance > constants::PERSONALITY_HIGH {
        weights[SpeechAct::SteelMan.idx()] += 5;
        weights[SpeechAct::Provocation.idx()] += 5;
    }
    if emo.curiosite > constants::PERSONALITY_HIGH {
        weights[SpeechAct::Question.idx()] += 8;
        weights[SpeechAct::Redirect.idx()] += 5;
    }
    if emo.engagement < constants::PERSONALITY_LOW {
        weights[SpeechAct::Humor.idx()] += 10;
        weights[SpeechAct::Provocation.idx()] += 5;
    }

    // Anti-repetition: acts used in the recent window are strongly penalised
    // (never zeroed — a mode may leave only a few acts available).
    for act in recent_acts.iter().rev().take(constants::SPEECH_ACT_RECENT_WINDOW) {
        let w = &mut weights[act.idx()];
        *w = (*w * constants::SPEECH_ACT_RECENT_WEIGHT_PERCENT / 100).max(1);
    }

    // Select with weighted random
    let mut rng = rand::thread_rng();
    let selected = weighted_select(&weights, &mut rng);

    let description = if is_fiction {
        selected.describe_fiction(lang)
    } else {
        selected.describe(lang)
    };
    let act_instruction = if is_fiction {
        match lang {
            "en" => format!("For this story segment, try this narrative approach: {}", description),
            "zh" => format!("在这段故事中，尝试这种叙事方式：{}", description),
            _ => format!("Pour ce segment de l'histoire, essaie cette approche narrative : {}", description),
        }
    } else {
        match lang {
            "en" => format!("For this intervention, favor this approach: {}", description),
            "zh" => format!("在这次发言中，优先采用这种方式：{}", description),
            _ => format!("Pour cette intervention, privilégie cette approche : {}", description),
        }
    };

    // The owed objection is named when the act is to answer it
    let act_instruction = match (&selected, &ctx.unanswered_objection) {
        (SpeechAct::Deepen, Some(objection)) if !is_fiction => match lang {
            "en" => format!("{act_instruction} Objection to answer: \"{objection}\"."),
            "zh" => format!("{act_instruction} 需要回应的反驳：\"{objection}\"。"),
            _ => format!("{act_instruction} Objection à traiter : « {objection} »."),
        },
        _ => act_instruction,
    };

    (selected, act_instruction)
}

fn weighted_select(weights: &[u32; SPEECH_ACT_COUNT], rng: &mut impl Rng) -> SpeechAct {
    let dist = WeightedIndex::new(weights).expect("weights should be valid");
    SpeechAct::ALL[dist.sample(rng)]
}

// ── Layer 4: Self-Memory Anti-Repetition ────────────────────────────

fn build_layer4_self_memory(ctx: &SpeakerTurnContext) -> String {
    if ctx.own_previous_messages.is_empty() {
        return String::new();
    }

    let lang = ctx.discussion_language.as_str();
    let truncated: Vec<String> = ctx
        .own_previous_messages
        .iter()
        .map(|msg| truncate_str(msg, 200).to_string())
        .collect();
    let joined = truncated.join(" / ");

    if ctx.discussion_mode == DiscussionMode::CollaborativeFiction {
        // Fiction: anti-repetition targets narrative content (scenes, descriptions, plot elements)
        return match lang {
            "en" => format!(
                "Your previous story segments: \"{}\". IMPORTANT: advance the story — do NOT repeat scenes, descriptions, or plot elements you already wrote.",
                joined
            ),
            "zh" => format!(
                "你之前的故事片段：\"{}\"。重要：推进故事——不要重复你已经写过的场景、描述或情节元素。",
                joined
            ),
            _ => format!(
                "Tes segments précédents de l'histoire : \"{}\". IMPORTANT : fais avancer l'histoire — ne répète PAS les scènes, descriptions ou éléments d'intrigue que tu as déjà écrits.",
                joined
            ),
        };
    }

    // Form (v1.20.2): the openings of the previous interventions are quoted back —
    // the same attack twice in a row (a name and a comma, a verbal tic) reads as a machine
    let openings = ctx.own_previous_messages.iter().map(|m| format!("« {} »", opening_of(m))).collect::<Vec<_>>().join(", ");
    match lang {
        "en" => format!(
            "Your previous interventions: \"{}\". IMPORTANT: find new formulations, new angles. Do NOT repeat yourself. They opened with {openings} — open differently this time: not the same first words, not systematically your interlocutor's name, a verbal tic at most once every three interventions.",
            joined
        ),
        "zh" => format!(
            "你之前的发言：\"{}\"。重要：找到新的表述方式和新角度。不要重复自己。它们的开头是{openings}——这次换一种开头：不要同样的起句，不要总以对话者的名字开头，口头禅最多每三次发言用一次。",
            joined
        ),
        _ => format!(
            "Tes interventions précédentes : \"{}\". IMPORTANT : trouve de nouvelles formulations, de nouveaux angles. Ne te répète PAS. Elles commençaient par {openings} — ouvre autrement cette fois : pas les mêmes premiers mots, pas systématiquement le nom de ton interlocuteur, un tic de langage au plus une fois sur trois interventions.",
            joined
        ),
    }
}

// ── Layer 5: Situational Awareness ──────────────────────────────────

fn build_layer5_situation(ctx: &SpeakerTurnContext) -> String {
    let lang = ctx.discussion_language.as_str();
    let is_fiction = ctx.discussion_mode == DiscussionMode::CollaborativeFiction;
    let mut parts: Vec<String> = Vec::new();

    // Turn 1 — opening instructions
    if ctx.turn_number <= 1 {
        let opening = if is_fiction {
            // Fiction: continue the opening if someone wrote one, otherwise write it
            match (&ctx.opening_author, ctx.is_first_speaker_this_turn) {
                (Some(author), _) => match lang {
                    "en" => format!("{author} has written the story opening. Continue the story from exactly where it left off. Write the next segment of the narrative."),
                    "zh" => format!("{author}已经写了故事开头。从停笔的地方准确地继续故事。写下叙事的下一个片段。"),
                    _ => format!("{author} a écrit l'ouverture de l'histoire. Continue le récit exactement là où il s'est arrêté. Écris le prochain segment du récit."),
                },
                (None, true) => match lang {
                    "en" => "Nobody has written yet: write the OPENING of the story — set the scene, a protagonist and an inciting event.".to_string(),
                    "zh" => "还没有人动笔：写下故事的开头——设定场景、一位主角和一个引发事件。".to_string(),
                    _ => "Personne n'a encore écrit : écris l'OUVERTURE de l'histoire — pose le décor, un protagoniste et un événement déclencheur.".to_string(),
                },
                (None, false) => {
                    let last = ctx.speakers_this_turn.last().map(String::as_str).unwrap_or_default();
                    match lang {
                        "en" => format!("Continue the story from where {last} left off. Ensure a seamless transition."),
                        "zh" => format!("从{last}停笔的地方继续故事。确保无缝过渡。"),
                        _ => format!("Continue l'histoire là où {last} s'est arrêté. Assure une transition fluide."),
                    }
                }
            }
        } else if ctx.is_first_speaker_this_turn {
            match lang {
                "en" => "This is the OPENING ROUND — present your initial contribution ONLY. Jump straight into your position with a strong, memorable statement. Do NOT respond to others yet. Keep it to one paragraph.",
                "zh" => "这是开场轮——仅表达你的初始贡献。以一个有力、令人难忘的声明直接切入你的立场。不要回应他人。保持一段论述。",
                _ => "C'est le TOUR D'OUVERTURE — présente uniquement ta contribution initiale. Entre directement dans le vif avec une affirmation forte et marquante. Ne réponds PAS aux autres. Reste sur un paragraphe.",
            }.to_string()
        } else {
            match lang {
                "en" => "This is the OPENING ROUND — present YOUR OWN initial position with a strong, distinctive angle. Do NOT respond to what previous speakers said — the exchanges deepen next round. Keep it to one paragraph.",
                "zh" => "这是开场轮——以独特的角度分享你的初始立场。不要回应之前发言者的内容——交流将在下一轮深入。保持一段论述。",
                _ => "C'est le TOUR D'OUVERTURE — présente TA PROPRE position avec un angle fort et distinctif. Ne réponds PAS à ce que les précédents ont dit — les échanges s'approfondissent au tour suivant. Reste sur un paragraphe.",
            }.to_string()
        };
        parts.push(opening);
        return parts.join("\n");
    }

    // Group mood — expressed differently for fiction vs debate
    let mood = if ctx.group_avg_frustration > 65 {
        if is_fiction {
            match lang {
                "en" => "The narrative energy is intense — channel it into dramatic tension in the story.".to_string(),
                "zh" => "叙事能量很强——将其化为故事中的戏剧张力。".to_string(),
                _ => "L'énergie narrative est intense — canalise-la en tension dramatique dans l'histoire.".to_string(),
            }
        } else {
            match lang {
                "en" => format!("The atmosphere is TENSE (average frustration: {}/100).", ctx.group_avg_frustration),
                "zh" => format!("气氛紧张（平均挫败感：{}/100）。", ctx.group_avg_frustration),
                _ => format!("L'ambiance est TENDUE (frustration moyenne : {}/100).", ctx.group_avg_frustration),
            }
        }
    } else if ctx.group_avg_engagement < 35 {
        if is_fiction {
            match lang {
                "en" => "The story needs a boost — add an unexpected twist or a gripping scene.".to_string(),
                "zh" => "故事需要提振——添加意想不到的转折或扣人心弦的场景。".to_string(),
                _ => "L'histoire a besoin d'un coup de fouet — ajoute un rebondissement inattendu ou une scène captivante.".to_string(),
            }
        } else {
            match lang {
                "en" => "The discussion energy is LOW — liven things up.".to_string(),
                "zh" => "讨论能量很低——活跃一下气氛。".to_string(),
                _ => "L'énergie de la discussion est BASSE — anime un peu les choses.".to_string(),
            }
        }
    } else {
        String::new()
    };
    if !mood.is_empty() {
        parts.push(mood);
    }

    // Conversational focus (debate-like modes): who to address, or the topic itself.
    // Replaces the generic "react to whoever just spoke" which made every speaker
    // pile onto the first one.
    if !is_fiction {
        if let Some(focus_text) = mode_prompts::focus_instruction(ctx.focus.as_ref(), lang) {
            parts.push(focus_text);
        }
    }

    // Turn position — expressed differently for fiction vs debate
    if is_fiction && ctx.is_first_speaker_this_turn {
        let pos = if is_fiction {
            match lang {
                "en" => "You write FIRST this round — continue the story where the previous round ended. Maintain narrative momentum.",
                "zh" => "你是本轮第一位作者——从上一轮结束的地方继续故事。保持叙事动力。",
                _ => "Tu écris en PREMIER ce tour-ci — continue l'histoire là où le tour précédent s'est terminé. Maintiens l'élan narratif.",
            }
        } else {
            match lang {
                "en" => "You speak FIRST this turn — react to the previous round, pick the strongest point and engage.",
                "zh" => "你本轮第一个发言——回应上一轮，选择最强的要点并回应。",
                _ => "Tu parles en PREMIER ce tour-ci — réagis au tour précédent, choisis le point le plus fort et engage.",
            }
        };
        parts.push(pos.to_string());
    } else if is_fiction && !ctx.speakers_this_turn.is_empty() {
        let names = ctx.speakers_this_turn.join(", ");
        let pos = if is_fiction {
            match lang {
                "en" => format!("Continue the story from where {} left off. Ensure a seamless transition.", names),
                "zh" => format!("从{}停笔的地方继续故事。确保无缝过渡。", names),
                _ => format!("Continue l'histoire là où {} s'est arrêté. Assure une transition fluide.", names),
            }
        } else {
            match lang {
                "en" => format!("You speak after {}. Build on or respond to what they just said.", names),
                "zh" => format!("你在{}之后发言。在他们刚说的基础上继续或回应。", names),
                _ => format!("Tu parles après {}. Rebondis sur ce qu'ils viennent de dire ou réponds-leur.", names),
            }
        };
        parts.push(pos);
    }

    // Ban return
    if ctx.was_recently_banned {
        let ban = if is_fiction {
            match lang {
                "en" => "You're BACK after being redirected. Resume writing the story — stay focused on the narrative.",
                "zh" => "你被重新引导后回来了。继续写故事——专注于叙事。",
                _ => "Tu es de RETOUR après un recadrage. Reprends l'écriture de l'histoire — reste concentré sur le récit.",
            }
        } else {
            match lang {
                "en" => "You're BACK after being banned. Show you've reflected — come in with a new angle, not the same old fire.",
                "zh" => "你被禁言后回来了。展示你已经反思过——用新角度切入，不要重蹈覆辙。",
                _ => "Tu es de RETOUR après un bannissement. Montre que tu as pris du recul — arrive avec un nouvel angle, pas les mêmes provocations.",
            }
        };
        parts.push(ban.to_string());
    }

    // Paragraph constraint
    let para = if is_fiction {
        match lang {
            "en" => "Write one or two focused paragraphs that ADVANCE THE PLOT — a new event, action, or revelation must occur. No atmospheric-only descriptions, no commentary, no meta-discussion. Never insert co-authors as characters.",
            "zh" => "写一到两段推进情节的内容——必须有新事件、行动或揭示。不要纯氛围描写，不要评论，不要元讨论。绝不将共同作者作为角色插入。",
            _ => "Écris un ou deux paragraphes qui font AVANCER L'INTRIGUE — un nouvel événement, une action ou une révélation doit se produire. Pas de description purement atmosphérique, pas de commentaire, pas de méta-discussion. N'insère jamais les co-auteurs comme personnages.",
        }
    } else {
        match lang {
            "en" => "Keep it to one or two focused paragraphs — don't pad or repeat yourself.",
            "zh" => "保持一到两段集中的论述——不要填充或重复自己。",
            _ => "Tiens-toi à un ou deux paragraphes — ne meuble pas et ne te répète pas.",
        }
    };
    parts.push(para.to_string());

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v1.20.1 — the Deepen act: drawn in a debate (pressed and named when an
    /// objection is owed), never in ideation.
    #[test]
    fn deepen_act_answers_owed_objections_in_argumentative_modes_only() {
        let mut ctx = make_test_ctx();
        ctx.discussion_mode = DiscussionMode::Debate;
        ctx.turn_number = 3;
        ctx.unanswered_objection = Some("Les outils déplacent le travail (Le Philosophe)".to_string());
        let deepen = (0..300).map(|_| build_dynamic_directive(&ctx)).find(|o| o.speech_act == "Deepen").expect("drawn within 300 draws");
        assert!(deepen.directive_text.contains("Objection à traiter : « Les outils déplacent le travail (Le Philosophe) »"), "{}", deepen.directive_text);
        ctx.discussion_mode = DiscussionMode::Ideation;
        assert!((0..300).all(|_| build_dynamic_directive(&ctx).speech_act != "Deepen"));
    }

    #[test]
    fn test_speech_act_recent_window_penalises_repeats() {
        // Recently used acts must be drawn far less often than fresh ones.
        let ctx = make_test_ctx();
        let recent = vec![SpeechAct::Challenge, SpeechAct::Question, SpeechAct::Anecdote];
        let mut repeats = 0;
        for _ in 0..300 {
            let (act, _) = build_layer3_speech_act(&ctx, &recent);
            if recent.contains(&act) {
                repeats += 1;
            }
        }
        // 3 penalised acts out of 10 with ~30% weight → expected ≈ 11% (< 25% with margin)
        assert!(repeats < 75, "repeats={repeats}");
    }

    #[test]
    fn test_directive_turn1_is_opening() {
        let mut ctx = make_test_ctx();
        ctx.turn_number = 1;
        ctx.is_first_speaker_this_turn = true;
        let output = build_dynamic_directive(&ctx);
        assert_eq!(output.speech_act, "Opening");
        assert!(output.directive_text.contains("OUVERTURE") || output.directive_text.contains("OPENING"));
    }

    #[test]
    fn test_directive_turn2_has_speech_act_and_focus() {
        let mut ctx = make_test_ctx();
        ctx.focus = Some(Focus::Speaker("Le Philosophe".to_string()));
        let output = build_dynamic_directive(&ctx);
        assert_ne!(output.speech_act, "Opening");
        assert!(output.directive_text.contains("approche") || output.directive_text.contains("approach"));
        assert!(output.directive_text.contains("Adresse-toi en priorité à Le Philosophe"));
        // No generic "react to whoever spoke" cue any more in debate modes
        assert!(!output.directive_text.contains("Tu parles après"));

        ctx.focus = Some(Focus::Topic);
        let output = build_dynamic_directive(&ctx);
        assert!(output.directive_text.contains("ne réponds à personne en particulier"));
    }

    #[test]
    fn test_two_dominant_emotions_and_tension_nuance() {
        let mut ctx = make_test_ctx();
        ctx.emotions = EmotionalProfile { frustration: 90, enthousiasme: 90, ..Default::default() };
        let text = build_layer1_emotion_behavior(&ctx).unwrap();
        assert!(text.contains("frustré"), "{text}");
        assert!(text.contains("enthousiaste"), "{text}");
        assert!(text.contains("tension"), "{text}");

        // Three triggers → only the two highest-priority ones are kept
        ctx.emotions = EmotionalProfile { frustration: 90, accord: 10, curiosite: 90, ..Default::default() };
        let text = build_layer1_emotion_behavior(&ctx).unwrap();
        assert!(text.contains("frustré") && text.contains("désaccord"), "{text}");
        assert!(!text.contains("curiosité"), "{text}");

        ctx.emotions = EmotionalProfile::default();
        assert!(build_layer1_emotion_behavior(&ctx).is_none());
    }

    /// v1.20.4 — the movement: what the debate did to the speaker since the start,
    /// measured from the persona's baseline, one line, never doubling a state.
    #[test]
    fn movement_line_names_the_shift_from_the_baseline() {
        let mut ctx = make_test_ctx();
        ctx.baseline = EmotionalProfile { confiance: 70, accord: 40, curiosite: 60, ..Default::default() };
        // Confiance dropped 20 from 70: shaken (no state — 50 is neither high nor low)
        ctx.emotions = EmotionalProfile { confiance: 50, accord: 40, ..Default::default() };
        let text = build_layer1_emotion_behavior(&ctx).unwrap();
        assert!(text.contains("t'a ébranlé") && text.contains("sans bravade"), "{text}");
        // A small move stays silent
        ctx.emotions = EmotionalProfile { confiance: 60, accord: 40, ..Default::default() };
        assert!(build_layer1_emotion_behavior(&ctx).is_none());
        // Accord rose 25 (the dominant shift): converging — appended after the states
        ctx.emotions = EmotionalProfile { confiance: 70, accord: 65, curiosite: 75, ..Default::default() };
        let text = build_layer1_emotion_behavior(&ctx).unwrap();
        assert!(text.starts_with("Ta curiosité est élevée") && text.ends_with("sans abandonner ta ligne."), "{text}");
        // The axis of a triggered state is never doubled by a movement line
        ctx.emotions = EmotionalProfile { confiance: 70, accord: 10, ..Default::default() };
        let text = build_layer1_emotion_behavior(&ctx).unwrap();
        assert!(text.contains("désaccord") && !text.contains("durci"), "{text}");
        // Movements without a behavioural reading stay silent (more curious)
        ctx.emotions = EmotionalProfile { confiance: 70, accord: 40, curiosite: 40, ..Default::default() };
        assert!(build_layer1_emotion_behavior(&ctx).is_none());
        // English and Chinese have their lines
        ctx.emotions = EmotionalProfile { confiance: 50, accord: 40, ..Default::default() };
        ctx.discussion_language = "en".into();
        assert!(build_layer1_emotion_behavior(&ctx).unwrap().contains("shook you"));
        ctx.discussion_language = "zh".into();
        assert!(build_layer1_emotion_behavior(&ctx).unwrap().contains("动摇了你"));
    }

    #[test]
    fn test_fiction_opening_author_wording() {
        let mut ctx = make_test_ctx();
        ctx.discussion_mode = DiscussionMode::CollaborativeFiction;
        ctx.turn_number = 1;
        ctx.is_first_speaker_this_turn = true;
        ctx.opening_author = None;
        assert!(build_dynamic_directive(&ctx).directive_text.contains("écris l'OUVERTURE"));
        ctx.opening_author = Some("Léo".to_string());
        assert!(build_dynamic_directive(&ctx).directive_text.contains("Léo a écrit l'ouverture"));
    }

    fn make_test_ctx() -> SpeakerTurnContext {
        SpeakerTurnContext {
            emotions: EmotionalProfile::default(),
            baseline: EmotionalProfile::default(),
            relationships: vec![],
            own_previous_messages: vec!["Previous message content".to_string()],
            dynamics: None,
            ocean: Some([8, 9, 4, 4, 3]),
            turn_number: 3,
            speakers_this_turn: vec!["Le Philosophe".to_string()],
            is_first_speaker_this_turn: false,
            was_recently_banned: false,
            group_avg_frustration: 40,
            group_avg_engagement: 55,
            discussion_language: "fr".to_string(),
            user_name: "Léo".to_string(),
            discussion_mode: DiscussionMode::Debate,
            focus: None,
            recent_speech_acts: vec![],
            opening_author: None,
            reconciliation_with: None,
            coalition: None,
            unanswered_objection: None,
            audience_message: None,
            user_has_spoken: false,
        }
    }

    /// v1.20.2 — the audience member is an observer until they speak, the first
    /// addressee right after, a participant afterwards; openings are quoted back.
    #[test]
    fn user_reminder_follows_the_audience_and_openings_are_quoted() {
        let mut ctx = make_test_ctx();
        let observer = build_dynamic_directive(&ctx).directive_text;
        assert!(observer.contains("Ne t'adresse PAS à Léo"), "{observer}");
        ctx.user_has_spoken = true;
        let participant = build_dynamic_directive(&ctx).directive_text;
        assert!(participant.contains("Léo, dans le public, a pris part au débat") && !participant.contains("Ne t'adresse PAS"), "{participant}");
        ctx.audience_message = Some("l'IA c'est bien".to_string());
        let owed = build_dynamic_directive(&ctx).directive_text;
        assert!(owed.contains("Léo, dans le public, vient d'intervenir : « l'IA c'est bien ». Il fait partie du débat désormais : réponds-lui D'ABORD"), "{owed}");
        // Openings of the previous interventions are quoted, bounded to one sentence
        ctx.own_previous_messages = vec!["Dieu, votre feu m'intéresse. Mais je dois planter un clou.".to_string(), "Le Boomer, vous demandez qui coupe le courant ! Voici.".to_string()];
        let text = build_dynamic_directive(&ctx).directive_text;
        assert!(text.contains("commençaient par « Dieu, votre feu m'intéresse. », « Le Boomer, vous demandez qui coupe le courant ! » — ouvre autrement"), "{text}");
        assert_eq!(opening_of("Une phrase sans fin"), "Une phrase sans fin");
        assert_eq!(opening_of(&"x".repeat(200)).chars().count(), constants::OPENING_EXCERPT_CHARS, "bounded");
    }

    /// v1.18 — `Relay` is never drawn; a coalition forces it on the leader and gives the follower its line.
    #[test]
    fn relay_is_only_forced_by_a_coalition() {
        let mut ctx = make_test_ctx();
        for _ in 0..200 {
            assert_ne!(build_layer3_speech_act(&ctx, &[]).0, SpeechAct::Relay);
        }
        ctx.coalition = Some(CoalitionRole::Leader { partner: "Le Philosophe".to_string() });
        let (act, text) = build_layer3_speech_act(&ctx, &[]);
        assert_eq!(act, SpeechAct::Relay);
        assert!(text.contains("Ton allié Le Philosophe parle juste après toi"), "{text}");
        ctx.coalition = Some(CoalitionRole::Follower { partner: "Le Scientifique".to_string() });
        let (act, text) = build_layer3_speech_act(&ctx, &[]);
        assert_ne!(act, SpeechAct::Relay);
        assert!(text.contains("vient de te passer le relais"), "{text}");
        assert_eq!(SpeechAct::from_name("Relay"), Some(SpeechAct::Relay));
    }

    /// v1.17 — agreeableness shapes how frustration is voiced; a reconciliation is a one-off invitation.
    #[test]
    fn frustration_style_follows_agreeableness_and_reconciliation_is_invited() {
        let mut ctx = make_test_ctx();
        ctx.emotions = EmotionalProfile { frustration: 90, ..Default::default() };
        ctx.ocean = Some([5, 5, 5, 9, 5]);
        assert!(build_layer1_emotion_behavior(&ctx).unwrap().contains("politesse glaciale"));
        ctx.ocean = Some([5, 5, 5, 2, 5]);
        assert!(build_layer1_emotion_behavior(&ctx).unwrap().contains("de front"));
        ctx.ocean = Some([5, 5, 5, 5, 5]);
        assert!(build_layer1_emotion_behavior(&ctx).unwrap().contains("Riposte plus fermement"), "middling A keeps the v1.16 line");
        ctx.ocean = None;
        assert!(build_layer1_emotion_behavior(&ctx).unwrap().contains("Riposte plus fermement"));
        // The persona own <dynamics> always wins over the generic variants
        ctx.ocean = Some([5, 5, 5, 9, 5]);
        ctx.dynamics = Some(ParsedDynamics { under_pressure: "Devient cassant.".to_string(), values: String::new(), triggers: String::new(), confident: String::new(), disengaged: String::new(), enthusiastic: None });
        assert!(build_layer1_emotion_behavior(&ctx).unwrap().contains("Devient cassant"));

        ctx.reconciliation_with = Some("Le Philosophe".to_string());
        assert!(build_layer2_relationships(&ctx).contains("Le Philosophe vient de te tendre la main"));
        ctx.discussion_language = "en".to_string();
        assert!(build_layer2_relationships(&ctx).contains("just extended a hand"));
    }
}
