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
use super::mode_prompts;
use super::truncate_str;
use crate::constants;
use crate::models::discussion::DiscussionMode;
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
    pub discussion_mode: DiscussionMode,
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

        // User reminder (observer in most modes, addressee in UserDriven, none in fiction)
        parts.push(build_user_reminder(lang, &ctx.user_name, &ctx.discussion_mode));

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
    parts.push(build_user_reminder(lang, &ctx.user_name, &ctx.discussion_mode));

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
fn build_user_reminder(lang: &str, user_name: &str, mode: &DiscussionMode) -> String {
    match mode {
        DiscussionMode::UserDriven => match lang {
            "en" => format!("Respond taking into account {}'s message.", user_name),
            "zh" => format!("在回应中考虑{}的消息。", user_name),
            _ => format!("Réponds en tenant compte du message de {}.", user_name),
        },
        DiscussionMode::CollaborativeFiction => String::new(),
        _ => mode_prompts::user_observer_clause(lang, user_name),
    }
}

// ── Layer 1: Emotion → Behavior Bridge ──────────────────────────────

fn build_layer1_emotion_behavior(ctx: &SpeakerTurnContext) -> Option<String> {
    let emo = &ctx.emotions;
    let lang = ctx.discussion_language.as_str();

    // CollaborativeFiction: emotions influence narrative writing style, not debate behavior
    if ctx.discussion_mode == DiscussionMode::CollaborativeFiction {
        return build_fiction_emotion_behavior(emo, lang);
    }

    // Priority: frustration > engagement > confiance > curiosité > enthousiasme > accord
    let behavior = if emo.frustration > constants::PERSONALITY_HIGH {
        match &ctx.dynamics {
            Some(d) if !d.under_pressure.is_empty() => format_behavior(lang, "under_pressure", &d.under_pressure),
            _ => generic_behavior(lang, "frustrated"),
        }
    } else if emo.engagement < constants::PERSONALITY_LOW {
        match &ctx.dynamics {
            Some(d) if !d.disengaged.is_empty() => format_behavior(lang, "disengaged", &d.disengaged),
            _ => generic_behavior(lang, "disengaged"),
        }
    } else if emo.confiance > constants::PERSONALITY_HIGH {
        match &ctx.dynamics {
            Some(d) if !d.confident.is_empty() => format_behavior(lang, "confident", &d.confident),
            _ => generic_behavior(lang, "confident"),
        }
    } else if emo.curiosite > constants::PERSONALITY_HIGH {
        match &ctx.dynamics {
            Some(d) if !d.triggers.is_empty() => format_behavior(lang, "curious", &d.triggers),
            _ => generic_behavior(lang, "curious"),
        }
    } else if emo.enthousiasme > constants::PERSONALITY_HIGH {
        match ctx.dynamics.as_ref().and_then(|d| d.enthusiastic.as_deref()).filter(|e| !e.is_empty()) {
            Some(enh) => format_behavior(lang, "enthusiastic", enh),
            None => generic_behavior(lang, "enthusiastic"),
        }
    } else if emo.accord < constants::PERSONALITY_LOW {
        generic_behavior(lang, "disagreeing")
    } else {
        return None;
    };

    Some(behavior)
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
            }
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
        // Debate and UserDriven: default weights (no modification)
        DiscussionMode::Debate | DiscussionMode::UserDriven => {}
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
    let is_fiction = ctx.discussion_mode == DiscussionMode::CollaborativeFiction;
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

    match lang {
        "en" => format!(
            "Your previous interventions: \"{}\". IMPORTANT: find new formulations, new angles. Do NOT repeat yourself.",
            joined
        ),
        "zh" => format!(
            "你之前的发言：\"{}\"。重要：找到新的表述方式和新角度。不要重复自己。",
            joined
        ),
        _ => format!(
            "Tes interventions précédentes : \"{}\". IMPORTANT : trouve de nouvelles formulations, de nouveaux angles. Ne te répète PAS.",
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
            // Fiction: all speakers on turn 1 continue the user's opening
            match lang {
                "en" => "The user has written the story opening. Continue the story from exactly where they left off. Write the next segment of the narrative.",
                "zh" => "用户已经写了故事开头。从他们停笔的地方准确地继续故事。写下叙事的下一个片段。",
                _ => "L'utilisateur a écrit l'ouverture de l'histoire. Continue le récit exactement là où il s'est arrêté. Écris le prochain segment du récit.",
            }
        } else if ctx.is_first_speaker_this_turn {
            match lang {
                "en" => "This is the OPENING ROUND — present your initial contribution ONLY. Jump straight into your position with a strong, memorable statement. Do NOT respond to others yet. Keep it to one paragraph.",
                "zh" => "这是开场轮——仅表达你的初始贡献。以一个有力、令人难忘的声明直接切入你的立场。不要回应他人。保持一段论述。",
                _ => "C'est le TOUR D'OUVERTURE — présente uniquement ta contribution initiale. Entre directement dans le vif avec une affirmation forte et marquante. Ne réponds PAS aux autres. Reste sur un paragraphe.",
            }
        } else {
            match lang {
                "en" => "This is the OPENING ROUND — present YOUR OWN initial position with a strong, distinctive angle. Do NOT respond to what previous speakers said — the exchanges deepen next round. Keep it to one paragraph.",
                "zh" => "这是开场轮——以独特的角度分享你的初始立场。不要回应之前发言者的内容——交流将在下一轮深入。保持一段论述。",
                _ => "C'est le TOUR D'OUVERTURE — présente TA PROPRE position avec un angle fort et distinctif. Ne réponds PAS à ce que les précédents ont dit — les échanges s'approfondissent au tour suivant. Reste sur un paragraphe.",
            }
        };
        parts.push(opening.to_string());
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

    // Turn position — expressed differently for fiction vs debate
    if ctx.is_first_speaker_this_turn {
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
    } else if !ctx.speakers_this_turn.is_empty() {
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
            discussion_mode: DiscussionMode::Debate,
        }
    }
}
