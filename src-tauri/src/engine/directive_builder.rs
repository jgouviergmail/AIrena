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
use super::truncate_str;
use crate::models::emotion::EmotionalProfile;

// ── Public types ────────────────────────────────────────────────────

/// Full context needed to build a directive for one speaker on one turn.
pub struct SpeakerTurnContext {
    pub emotions: EmotionalProfile,
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
}

pub struct RelationshipHint {
    pub other_name: String,
    pub kind: RelationshipKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipKind {
    Ally,
    Rival,
    Tense,
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
}

// Compile-time guarantee: enum discriminants match ALL array indices.
const _: () = assert!(SpeechAct::Challenge as usize == 0);
const _: () = assert!(SpeechAct::Synthesis as usize == 9);

impl SpeechAct {
    const ALL: [SpeechAct; 10] = [
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

            (SpeechAct::SteelMan, "en") => "Reformulate your opponent's argument in its strongest form, then respond to THAT version.",
            (SpeechAct::SteelMan, "zh") => "将对手的论点以最强形式重述，然后回应那个版本。",
            (SpeechAct::SteelMan, _) => "Reformule l'argument adverse dans sa version la plus forte, puis réponds à CETTE version.",

            (SpeechAct::Anecdote, "en") => "Illustrate your point with a personal story, a vivid example, or a striking analogy.",
            (SpeechAct::Anecdote, "zh") => "用个人故事、生动的例子或引人注目的类比来说明你的观点。",
            (SpeechAct::Anecdote, _) => "Illustre ton propos par une histoire personnelle, un exemple frappant ou une analogie saisissante.",

            (SpeechAct::Question, "en") => "Ask a probing, open question to a specific participant — genuinely explore their reasoning.",
            (SpeechAct::Question, "zh") => "向某个特定参与者提出一个深入的开放性问题——真正探索他们的推理。",
            (SpeechAct::Question, _) => "Pose une question ouverte et incisive à un participant précis — explore sincèrement son raisonnement.",

            (SpeechAct::Provocation, "en") => "Launch a deliberate provocation — a bold, spicy statement designed to shake up the debate.",
            (SpeechAct::Provocation, "zh") => "发起一次故意的挑衅——一个大胆、辛辣的声明，旨在打破辩论的沉闷。",
            (SpeechAct::Provocation, _) => "Lance une provocation délibérée — une affirmation audacieuse et piquante pour secouer le débat.",

            (SpeechAct::Concession, "en") => "Admit a point your opponent made well — then pivot to show why your position still holds.",
            (SpeechAct::Concession, "zh") => "承认对手提出的一个好观点——然后转向展示为什么你的立场仍然成立。",
            (SpeechAct::Concession, _) => "Admets un point bien formulé par un adversaire — puis pivote pour montrer pourquoi ta position tient toujours.",

            (SpeechAct::Redirect, "en") => "Shift the angle — bring up an aspect of the topic nobody has explored yet.",
            (SpeechAct::Redirect, "zh") => "转换角度——提出一个还没有人探讨过的话题方面。",
            (SpeechAct::Redirect, _) => "Change d'angle — aborde un aspect du sujet que personne n'a encore exploré.",

            (SpeechAct::Humor, "en") => "Defuse tension with humor — a witty remark, a clever comparison, or gentle mockery.",
            (SpeechAct::Humor, "zh") => "用幽默化解紧张——一句机智的话、巧妙的对比或温和的嘲讽。",
            (SpeechAct::Humor, _) => "Désamorce la tension par l'humour — une remarque piquante, une comparaison maligne ou une moquerie bienveillante.",

            (SpeechAct::Appeal, "en") => "Appeal to shared values or emotions — connect your argument to something everyone cares about.",
            (SpeechAct::Appeal, "zh") => "诉诸共同价值观或情感——将你的论点与大家关心的事物联系起来。",
            (SpeechAct::Appeal, _) => "Fais appel aux valeurs ou émotions partagées — relie ton argument à quelque chose qui touche tout le monde.",

            (SpeechAct::Synthesis, "en") => "Synthesize the debate so far — summarize key positions, then push forward with YOUR evolved stance.",
            (SpeechAct::Synthesis, "zh") => "综合迄今为止的辩论——总结关键立场，然后以你进化的立场推动讨论前进。",
            (SpeechAct::Synthesis, _) => "Synthétise le débat — résume les positions clés, puis fais avancer avec TA position enrichie.",
        }
    }
}

// ── Main builder ────────────────────────────────────────────────────

/// Build a dynamic behavioral directive for the given speaker context.
/// For turn 1, returns a basic directive with no emotion/relationship context.
/// For turns 2+, applies all 5 layers (emotion, relationships, speech acts, memory, situation).
pub fn build_dynamic_directive(
    ctx: &SpeakerTurnContext,
    last_speech_act: Option<&SpeechAct>,
) -> DirectiveOutput {
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
        let (selected_act, act_text) = build_layer3_speech_act(ctx, last_speech_act);

        parts.push(act_text);

        // Layer 4: Self-memory anti-repetition
        let self_memory = build_layer4_self_memory(ctx);
        if !self_memory.is_empty() {
            parts.push(self_memory);
        }

        // User observer reminder
        let observer_reminder = match lang {
            "en" => format!(
                "Do NOT address or speak to {} who is only an observer.",
                ctx.user_name
            ),
            "zh" => format!("不要对{}说话，此人只是观察者。", ctx.user_name),
            _ => format!(
                "Ne t'adresse PAS à {} qui n'est qu'un observateur.",
                ctx.user_name
            ),
        };
        parts.push(observer_reminder);

        let rel_summary_for_ui = build_relationship_summary_for_ui(ctx);

        return DirectiveOutput {
            directive_text: parts.join("\n"),
            speech_act: selected_act.name().to_string(),
            emotion_behavior,
            relationship_summary: rel_summary_for_ui,
        };
    }

    // Turn 1: only Layer 5 + observer reminder
    let observer_reminder = match lang {
        "en" => format!(
            "Do NOT address or speak to {} who is only an observer.",
            ctx.user_name
        ),
        "zh" => format!("不要对{}说话，此人只是观察者。", ctx.user_name),
        _ => format!(
            "Ne t'adresse PAS à {} qui n'est qu'un observateur.",
            ctx.user_name
        ),
    };
    parts.push(observer_reminder);

    DirectiveOutput {
        directive_text: parts.join("\n"),
        speech_act: "Opening".to_string(),
        emotion_behavior: None,
        relationship_summary: String::new(),
    }
}

// ── Layer 1: Emotion → Behavior Bridge ──────────────────────────────

const HIGH_THRESHOLD: u8 = 70;
const LOW_THRESHOLD: u8 = 30;

fn build_layer1_emotion_behavior(ctx: &SpeakerTurnContext) -> Option<String> {
    let emo = &ctx.emotions;
    let lang = ctx.discussion_language.as_str();

    // Priority: frustration > engagement > confiance > curiosité > enthousiasme > accord
    let behavior = if emo.frustration > HIGH_THRESHOLD {
        match &ctx.dynamics {
            Some(d) if !d.under_pressure.is_empty() => format_behavior(lang, "under_pressure", &d.under_pressure),
            _ => generic_behavior(lang, "frustrated"),
        }
    } else if emo.engagement < LOW_THRESHOLD {
        match &ctx.dynamics {
            Some(d) if !d.disengaged.is_empty() => format_behavior(lang, "disengaged", &d.disengaged),
            _ => generic_behavior(lang, "disengaged"),
        }
    } else if emo.confiance > HIGH_THRESHOLD {
        match &ctx.dynamics {
            Some(d) if !d.confident.is_empty() => format_behavior(lang, "confident", &d.confident),
            _ => generic_behavior(lang, "confident"),
        }
    } else if emo.curiosite > HIGH_THRESHOLD {
        match &ctx.dynamics {
            Some(d) if !d.triggers.is_empty() => format_behavior(lang, "curious", &d.triggers),
            _ => generic_behavior(lang, "curious"),
        }
    } else if emo.enthousiasme > HIGH_THRESHOLD {
        match ctx.dynamics.as_ref().and_then(|d| d.enthusiastic.as_deref()).filter(|e| !e.is_empty()) {
            Some(enh) => format_behavior(lang, "enthusiastic", enh),
            None => generic_behavior(lang, "enthusiastic"),
        }
    } else if emo.accord < LOW_THRESHOLD {
        generic_behavior(lang, "disagreeing")
    } else {
        return None;
    };

    Some(behavior)
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

fn generic_behavior(lang: &str, emotion_key: &str) -> String {
    match (lang, emotion_key) {
        ("en", "frustrated") => "You're frustrated — your tone sharpens, your patience thins. Push back harder.".to_string(),
        ("zh", "frustrated") => "你很沮丧——语气更尖锐，耐心更少。更强硬地反击。".to_string(),
        (_, "frustrated") => "Tu es frustré — ton ton se durcit, ta patience s'amenuise. Riposte plus fermement.".to_string(),
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

fn build_layer2_relationships(ctx: &SpeakerTurnContext) -> String {
    let lang = ctx.discussion_language.as_str();
    let mut hints: Vec<String> = Vec::new();

    for rel in &ctx.relationships {
        let hint = match (&rel.kind, lang) {
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
            (RelationshipKind::Tense, "en") => format!(
                "Tension with {}: the relationship is asymmetric — read the room and adapt.",
                rel.other_name
            ),
            (RelationshipKind::Tense, "zh") => format!(
                "与{}关系紧张：关系是不对称的——观察形势并调整。",
                rel.other_name
            ),
            (RelationshipKind::Tense, _) => format!(
                "Tension avec {} : la relation est asymétrique — lis la situation et adapte-toi.",
                rel.other_name
            ),
        };
        hints.push(hint);
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
    last_act: Option<&SpeechAct>,
) -> (SpeechAct, String) {
    let mut weights = [10u32; 10]; // Base weight: 10 each

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
    if emo.frustration > HIGH_THRESHOLD {
        weights[SpeechAct::Challenge.idx()] += 8;
        weights[SpeechAct::Provocation.idx()] += 5;
    }
    if emo.confiance > HIGH_THRESHOLD {
        weights[SpeechAct::SteelMan.idx()] += 5;
        weights[SpeechAct::Provocation.idx()] += 5;
    }
    if emo.curiosite > HIGH_THRESHOLD {
        weights[SpeechAct::Question.idx()] += 8;
        weights[SpeechAct::Redirect.idx()] += 5;
    }
    if emo.engagement < LOW_THRESHOLD {
        weights[SpeechAct::Humor.idx()] += 10;
        weights[SpeechAct::Provocation.idx()] += 5;
    }

    // Select with weighted random
    let mut rng = rand::thread_rng();
    let mut selected = weighted_select(&weights, &mut rng);

    // Anti-repetition: if same as last act, re-roll once
    if let Some(last) = last_act {
        if selected == *last {
            selected = weighted_select(&weights, &mut rng);
        }
    }

    let lang = ctx.discussion_language.as_str();
    let description = selected.describe(lang);
    let act_instruction = match lang {
        "en" => format!("For this intervention, favor this approach: {}", description),
        "zh" => format!("在这次发言中，优先采用这种方式：{}", description),
        _ => format!("Pour cette intervention, privilégie cette approche : {}", description),
    };

    (selected, act_instruction)
}

fn weighted_select(weights: &[u32; 10], rng: &mut impl Rng) -> SpeechAct {
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

    match lang {
        "en" => format!(
            "Your previous interventions: \"{}\". IMPORTANT: find new formulations, new angles. Do NOT repeat your arguments.",
            joined
        ),
        "zh" => format!(
            "你之前的发言：\"{}\"。重要：找到新的表述方式和新角度。不要重复你的论点。",
            joined
        ),
        _ => format!(
            "Tes interventions précédentes : \"{}\". IMPORTANT : trouve de nouvelles formulations, de nouveaux angles. Ne répète PAS tes arguments.",
            joined
        ),
    }
}

// ── Layer 5: Situational Awareness ──────────────────────────────────

fn build_layer5_situation(ctx: &SpeakerTurnContext) -> String {
    let lang = ctx.discussion_language.as_str();
    let mut parts: Vec<String> = Vec::new();

    // Turn 1 — opening instructions
    if ctx.turn_number <= 1 {
        let opening = if ctx.is_first_speaker_this_turn {
            match lang {
                "en" => "This is the OPENING ROUND — present your initial opinion ONLY. Jump straight into your position with a strong, memorable statement. Do NOT debate or respond to others yet. Keep it to one paragraph.",
                "zh" => "这是开场轮——仅表达你的初始观点。以一个有力、令人难忘的声明直接切入你的立场。不要辩论或回应他人。保持一段论述。",
                _ => "C'est le TOUR D'OUVERTURE — présente uniquement ton opinion initiale. Entre directement dans le vif avec une affirmation forte et marquante. Ne débats PAS et ne réponds PAS aux autres. Reste sur un paragraphe.",
            }
        } else {
            match lang {
                "en" => "This is the OPENING ROUND — present YOUR OWN initial position with a strong, distinctive angle. Do NOT debate or respond to what previous speakers said — the debate begins next round. Keep it to one paragraph.",
                "zh" => "这是开场轮——以独特的角度分享你的初始立场。不要辩论或回应之前发言者的内容——辩论从下一轮开始。保持一段论述。",
                _ => "C'est le TOUR D'OUVERTURE — présente TA PROPRE position avec un angle fort et distinctif. Ne débats PAS et ne réponds PAS à ce que les précédents ont dit — le débat commence au tour suivant. Reste sur un paragraphe.",
            }
        };
        parts.push(opening.to_string());
        return parts.join("\n");
    }

    // Group mood
    let mood = if ctx.group_avg_frustration > 65 {
        match lang {
            "en" => format!("The atmosphere is TENSE (average frustration: {}/100).", ctx.group_avg_frustration),
            "zh" => format!("气氛紧张（平均挫败感：{}/100）。", ctx.group_avg_frustration),
            _ => format!("L'ambiance est TENDUE (frustration moyenne : {}/100).", ctx.group_avg_frustration),
        }
    } else if ctx.group_avg_engagement < 35 {
        match lang {
            "en" => "The debate energy is LOW — liven things up.".to_string(),
            "zh" => "辩论能量很低——活跃一下气氛。".to_string(),
            _ => "L'énergie du débat est BASSE — anime un peu les choses.".to_string(),
        }
    } else {
        String::new()
    };
    if !mood.is_empty() {
        parts.push(mood);
    }

    // Turn position
    if ctx.is_first_speaker_this_turn {
        let pos = match lang {
            "en" => "You speak FIRST this turn — react to the previous round, pick the strongest argument and engage.",
            "zh" => "你本轮第一个发言——回应上一轮，选择最强的论点并回应。",
            _ => "Tu parles en PREMIER ce tour-ci — réagis au tour précédent, choisis l'argument le plus fort et engage.",
        };
        parts.push(pos.to_string());
    } else if !ctx.speakers_this_turn.is_empty() {
        let names = ctx.speakers_this_turn.join(", ");
        let pos = match lang {
            "en" => format!("You speak after {}. Build on or challenge what they just said.", names),
            "zh" => format!("你在{}之后发言。在他们刚说的基础上继续或挑战。", names),
            _ => format!("Tu parles après {}. Rebondis sur ce qu'ils viennent de dire ou conteste-les.", names),
        };
        parts.push(pos);
    }

    // Ban return
    if ctx.was_recently_banned {
        let ban = match lang {
            "en" => "You're BACK after being banned. Show you've reflected — come in with a new angle, not the same old fire.",
            "zh" => "你被禁言后回来了。展示你已经反思过——用新角度切入，不要重蹈覆辙。",
            _ => "Tu es de RETOUR après un bannissement. Montre que tu as pris du recul — arrive avec un nouvel angle, pas les mêmes provocations.",
        };
        parts.push(ban.to_string());
    }

    // Paragraph constraint
    let para = match lang {
        "en" => "Keep it to one or two focused paragraphs — don't pad or repeat yourself.",
        "zh" => "保持一到两段集中的论述——不要填充或重复自己。",
        _ => "Tiens-toi à un ou deux paragraphes — ne meuble pas et ne te répète pas.",
    };
    parts.push(para.to_string());

    parts.join("\n")
}

// ── Relationship building helper ────────────────────────────────────

/// Build relationship hints from cumulative reactions.
/// `reactions_from_me`: (target_id, target_name, likes_i_gave, dislikes_i_gave)
/// `reactions_to_me`: (source_id, source_name, likes_they_gave_me, dislikes_they_gave_me)
pub fn build_relationships(
    reactions_from_me: &[(String, String, u32, u32)],
    reactions_to_me: &[(String, String, u32, u32)],
) -> Vec<RelationshipHint> {
    let mut hints = Vec::new();

    for (target_id, target_name, my_likes, my_dislikes) in reactions_from_me {
        // Find reverse: how does target feel about me?
        let (their_likes, their_dislikes) = reactions_to_me
            .iter()
            .find(|(src_id, _, _, _)| src_id == target_id)
            .map(|(_, _, l, d)| (*l, *d))
            .unwrap_or((0, 0));

        let kind = classify_relationship(*my_likes, *my_dislikes, their_likes, their_dislikes);
        if let Some(kind) = kind {
            hints.push(RelationshipHint {
                other_name: target_name.clone(),
                kind,
            });
        }
    }

    hints
}

fn classify_relationship(
    my_likes: u32,
    my_dislikes: u32,
    their_likes: u32,
    their_dislikes: u32,
) -> Option<RelationshipKind> {
    let mutual_likes = my_likes.min(their_likes);
    let mutual_dislikes = my_dislikes.min(their_dislikes);

    if mutual_likes >= 2 {
        Some(RelationshipKind::Ally)
    } else if mutual_dislikes >= 2 {
        Some(RelationshipKind::Rival)
    } else if (my_likes >= 2 && their_dislikes >= 2) || (my_dislikes >= 2 && their_likes >= 2) {
        Some(RelationshipKind::Tense)
    } else {
        None // Neutral — don't inject
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_ally() {
        assert_eq!(
            classify_relationship(3, 0, 2, 0),
            Some(RelationshipKind::Ally)
        );
    }

    #[test]
    fn test_classify_rival() {
        assert_eq!(
            classify_relationship(0, 3, 0, 2),
            Some(RelationshipKind::Rival)
        );
    }

    #[test]
    fn test_classify_tense() {
        assert_eq!(
            classify_relationship(2, 0, 0, 3),
            Some(RelationshipKind::Tense)
        );
    }

    #[test]
    fn test_classify_neutral() {
        assert_eq!(classify_relationship(1, 0, 0, 1), None);
    }

    #[test]
    fn test_build_relationships() {
        let from_me = vec![
            ("id1".to_string(), "Alice".to_string(), 3, 0),
            ("id2".to_string(), "Bob".to_string(), 0, 3),
        ];
        let to_me = vec![
            ("id1".to_string(), "Alice".to_string(), 2, 0),
            ("id2".to_string(), "Bob".to_string(), 0, 2),
        ];
        let rels = build_relationships(&from_me, &to_me);
        assert_eq!(rels.len(), 2);
        assert_eq!(rels[0].kind, RelationshipKind::Ally);
        assert_eq!(rels[1].kind, RelationshipKind::Rival);
    }

    #[test]
    fn test_speech_act_anti_repetition() {
        // With fixed seed, verify that re-roll produces at least sometimes a different act
        let ctx = make_test_ctx();
        let last = SpeechAct::Challenge;
        let mut different_count = 0;
        for _ in 0..20 {
            let (act, _) = build_layer3_speech_act(&ctx, Some(&last));
            if act != last {
                different_count += 1;
            }
        }
        // At least some should be different (statistically near-certain with 20 tries)
        assert!(different_count > 0);
    }

    #[test]
    fn test_directive_turn1_is_opening() {
        let mut ctx = make_test_ctx();
        ctx.turn_number = 1;
        ctx.is_first_speaker_this_turn = true;
        let output = build_dynamic_directive(&ctx, None);
        assert_eq!(output.speech_act, "Opening");
        assert!(output.directive_text.contains("OUVERTURE") || output.directive_text.contains("OPENING"));
    }

    #[test]
    fn test_directive_turn2_has_speech_act() {
        let ctx = make_test_ctx();
        let output = build_dynamic_directive(&ctx, None);
        assert_ne!(output.speech_act, "Opening");
        // Should contain approach instruction
        assert!(
            output.directive_text.contains("approche") || output.directive_text.contains("approach"),
        );
    }

    fn make_test_ctx() -> SpeakerTurnContext {
        SpeakerTurnContext {
            emotions: EmotionalProfile::default(),
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
        }
    }
}
