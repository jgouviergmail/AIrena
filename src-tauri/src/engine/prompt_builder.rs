use super::reactions::{self, ReactionPropensity, ReactionScope};
use crate::constants;
use crate::engine::token_budget::TokenBudget;
use crate::models::agenda::{Agenda, AgendaReveal};
use crate::models::discussion::DiscussionMode;
use crate::models::emotion::{EmotionalProfile, RoomMood};
use crate::models::intention::{Intention, IntentionGoal};
use crate::models::memory::{ParticipantMemory, ParticipantPosition};
use crate::models::message::{Message, MessageKind, ReactionType, SpeakerRole};
use crate::models::outcome::{
    ModeOutcome, VERDICT_DEFENSE, VERDICT_PROSECUTION, VOTE_AGAINST, VOTE_FOR,
};
use crate::models::persona_memory::PersonaMemory;
#[cfg(test)]
use crate::models::source::SourceKind;
use crate::models::source::SourceRecord;
use crate::tavily::TavilySearchResponse;
use crate::wikipedia::WikiSearchResponse;

use super::focus::Focus;
use super::mode_prompts;
use super::open_loops::{OpenLoop, OpenLoopKind};

/// Staging of the turn for one speaker (v1.18): the act's instruction and the
/// scene event's, already bounded; `closing` marks the last act (the generic
/// end-of-discussion reminder then steps aside).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageBlock {
    pub text: String,
    pub closing: bool,
}
use super::truncate_at_word_boundary;
use super::truncate_str as truncate;
use super::truncate_tail;

/// Build the introduction prompt for the IArbitre
pub fn build_introduction_prompt(
    topic: &str,
    participant_names: &[String],
    discussion_language: &str,
    web_search_results: Option<&str>,
    mode: &DiscussionMode,
    full_document: Option<&str>,
    budget: &TokenBudget,
    ) -> String {
    // Truncate web search results to budget allocation.
    let web_search_results =
        web_search_results.map(|r| truncate(r, budget.external_knowledge_chars()));
    let participants = participant_names.join(", ");
    let datetime = build_datetime_context(discussion_language);
    let full_doc_block = build_full_document_block(
        full_document,
        budget.full_document_chars,
        discussion_language,
    );
    let web_block = web_search_results
    .map(|r| {
        let instruction = match discussion_language {
            "en" => "\n⚡ Only use results that are relevant to the discussion topic. Ignore off-topic results. \
            Weave key facts or current data naturally into your introduction to frame the topic. Do NOT list them raw.",
            "zh" => "\n⚡ 只使用与讨论主题相关的结果。忽略偏题的结果。\
            将关键事实或当前数据自然地融入你的介绍中来构建主题框架。不要原样列举。",
            _ => "\n⚡ N'utilise que les résultats pertinents par rapport au sujet de la discussion. Ignore les résultats hors-sujet. \
            Intègre naturellement des faits clés ou des données actuelles dans ton introduction pour cadrer le sujet. Ne les liste PAS tels quels.",
        };
        format!("\n\n{}{}", r, instruction)
    })
    .unwrap_or_default();
    let mode_desc = mode_prompts::mode_descriptor(mode, discussion_language);
    let mode_instructions = mode_prompts::mode_introduction_instructions(mode, discussion_language);

    // CollaborativeFiction: specific introduction — explain relay-writing, invite user to start
    if *mode == DiscussionMode::CollaborativeFiction {
        return match discussion_language {
            "en" => format!(
                "{}\n\nYou are the moderator of a collaborative fiction exercise. \
            The theme is: \"{}\"\nThe co-authors are: {}{}{}\n\n\
            Briefly explain the rules (2-3 sentences): this is a relay-written story where the user \
            writes the opening, then each co-author continues the story in sequence where the previous \
            writer left off. Transitions must be seamless and coherent.\n\
            {}\n\
            Then invite the user to write the story opening.\n\
            IMPORTANT: Do NOT use any markdown formatting (no #, ##, **, *, -, bullet points, code blocks). Write in plain conversational text only.\n\
            IMPORTANT: You MUST respond entirely in English.",
            datetime, topic, participants, web_block, full_doc_block, mode_instructions
        ),
        "zh" => format!(
            "{}\n\n你是一个协作小说练习的主持人。\
            主题是：\"{}\"\n共同作者有：{}{}{}\n\n\
            简要解释规则（2-3句话）：这是一个接力写作故事，用户写开头，\
            然后每位共同作者按顺序从上一位作者停笔的地方继续。过渡必须流畅且连贯。\n\
            {}\n\
            然后邀请用户写故事开头。\n\
            重要：不要使用任何markdown格式（不要用#、##、**、*、-、列表、代码块）。只用纯对话文本。\n\
            重要：你必须完全用中文回答。",
            datetime, topic, participants, web_block, full_doc_block, mode_instructions
        ),
        _ => format!(
            "{}\n\nTu es le modérateur d'un exercice de fiction collaborative. \
            Le thème est : \"{}\"\nLes co-auteurs sont : {}{}{}\n\n\
            Explique brièvement les règles (2-3 phrases) : c'est une histoire écrite en relais où l'utilisateur \
            écrit l'ouverture, puis chaque co-auteur continue l'histoire à la suite du précédent. \
            Les transitions doivent être fluides et cohérentes.\n\
            {}\n\
            Puis invite l'utilisateur à écrire l'ouverture de l'histoire.\n\
            IMPÉRATIF : N'utilise AUCUN formatage markdown (pas de #, ##, **, *, -, listes à puces, blocs de code). Écris uniquement en texte conversationnel simple.\n\
            IMPÉRATIF : Tu DOIS répondre intégralement en français.",
            datetime, topic, participants, web_block, full_doc_block, mode_instructions
        ),
    };
    }

    match discussion_language {
        "en" => format!(
            "{}\n\nYou are the moderator of a {}. The topic is: \"{}\"\n\
            The participants are: {}{}{}\n\n\
            Introduce the topic in a BROAD and NEUTRAL manner (2-3 sentences), covering the key dimensions \
            and perspectives of the subject without narrowing it to a single angle or your personal bias.\n\
            {}\n\
            Then invite the first participant to speak.\n\
            IMPORTANT: Do NOT use any markdown formatting (no #, ##, **, *, -, bullet points, code blocks). Write in plain conversational text only.\n\
            IMPORTANT: You MUST respond entirely in English.",
            datetime, mode_desc, topic, participants, web_block, full_doc_block, mode_instructions
        ),
        "zh" => format!(
            "{}\n\n你是一场{}的主持人。主题是：\"{}\"\n\
            参与者有：{}{}{}\n\n\
            以广泛且中立的方式简要介绍主题（2-3句话），涵盖该主题的主要方面和视角，\
            不要将其缩小为单一角度或个人偏见。\n\
            {}\n\
            然后邀请第一位参与者发言。\n\
            重要：不要使用任何markdown格式（不要用#、##、**、*、-、列表、代码块）。只用纯对话文本。\n\
            重要：你必须完全用中文回答。",
            datetime, mode_desc, topic, participants, web_block, full_doc_block, mode_instructions
        ),
        _ => format!(
            "{}\n\nTu es le modérateur d'un(e) {}. Le sujet est : \"{}\"\n\
            Les participants sont : {}{}{}\n\n\
            Présente le sujet de manière LARGE et NEUTRE (2-3 phrases), en couvrant les dimensions \
            et perspectives clés du sujet sans le réduire à un seul angle ou à ton biais personnel.\n\
            {}\n\
            Puis invite le premier participant à prendre la parole.\n\
            IMPÉRATIF : N'utilise AUCUN formatage markdown (pas de #, ##, **, *, -, listes à puces, blocs de code). Écris uniquement en texte conversationnel simple.\n\
            IMPÉRATIF : Tu DOIS répondre intégralement en français.",
            datetime, mode_desc, topic, participants, web_block, full_doc_block, mode_instructions
        ),
    }
}

/// Build the reaction prompt for a gladiator (mode-aware + language-enforced)
pub fn build_reaction_prompt(
    interventions: &[(String, String)], // (speaker_name, content)
    discussion_language: &str,
    mode: &DiscussionMode,
    scope: ReactionScope,
    propensity: Option<&ReactionPropensity>,
    insightful_credit: bool,
) -> String {
    let lang = discussion_language;
    let list = interventions
        .iter()
        .map(|(name, content)| {
            format!(
                "- {} : \"{}\"",
                name,
                truncate(content, constants::TRUNC_REACTION_CONTENT)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Dynamic example using actual participant names. The example's colour rotates
    // with the content (v1.20.4): a fixed "insightful" anchored the models on it —
    // 78 % of the reactions of a real debate were 💡 — so it is never the example.
    let allowed = reactions::allowed_reaction_types(mode);
    let first = interventions
        .first()
        .map(|(n, _)| n.as_str())
        .unwrap_or("Name");
    let seed: usize = interventions.iter().map(|(_, c)| c.len()).sum();
    let example_kind = reactions::example_reaction_kind(allowed, seed);
    // A single intervention (immediate rounds): every other example shows "none" —
    // a model shown a reaction every time reacts every time (v1.20.5, deepseek-flash: 100 %)
    let example = match interventions.get(1) {
        Some((second, _)) => format!(
            "[{{\"speaker\":\"{first}\",\"reacts\":false,\"reaction\":\"none\",\"justification\":\"\",\"quote\":\"\"}},\
            {{\"speaker\":\"{second}\",\"reacts\":true,\"reaction\":\"{}\",\"justification\":\"...\",\"quote\":\"...\"}}]",
            example_kind.as_str()
        ),
        None if seed.is_multiple_of(2) => format!("[{{\"speaker\":\"{first}\",\"reacts\":false,\"reaction\":\"none\",\"justification\":\"\",\"quote\":\"\"}}]"),
        None => format!("[{{\"speaker\":\"{first}\",\"reacts\":true,\"reaction\":\"{}\",\"justification\":\"...\",\"quote\":\"...\"}}]", example_kind.as_str()),
    };
    let credit_line = if insightful_credit {
        String::new()
    } else {
        match lang {
            "en" => "You already singled out a strong point recently: your \"insightful\" credit is spent for now — approve, question or disapprove, but do not single out.\n".to_string(),
            "zh" => "你最近已经标记过一个有力观点：\"insightful\" 的额度暂时用完了——可以赞同、提问或反对，但不要再标记。\n".to_string(),
            _ => "Tu as déjà distingué un point fort récemment : ton crédit \"insightful\" est épuisé pour l'instant — approuve, questionne ou désapprouve, mais ne distingue pas.\n".to_string(),
        }
    };

    let (like_meaning, dislike_meaning) = mode_prompts::mode_reaction_meanings(mode, lang);
    let sincerity = reactions::sincerity_rules(allowed, lang);
    let vocabulary = allowed
        .iter()
        .map(|kind| {
            let meaning = match kind {
                ReactionType::Like => like_meaning,
                ReactionType::Dislike => dislike_meaning,
                other => reactions::describe_reaction_type(*other, lang),
            };
            match lang {
                "zh" => format!("- \"{}\"：{}", kind.as_str(), meaning),
                _ => format!("- \"{}\": {}", kind.as_str(), meaning),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let none_line = match lang {
        "en" => "- \"none\": nothing to say",
        "zh" => "- \"none\"：无话可说",
        _ => "- \"none\" : rien à dire",
    };
    let intro = match (scope, lang) {
        (ReactionScope::PreviousTurn, "en") => {
            "Here are the interventions of OTHER participants in the previous turn:"
        }
        (ReactionScope::PreviousTurn, "zh") => "以下是上一轮其他参与者的发言：",
        (ReactionScope::PreviousTurn, _) => {
            "Voici les interventions des AUTRES participants au tour précédent :"
        }
        (ReactionScope::LastIntervention, "en") => "Here is the intervention that just ended:",
        (ReactionScope::LastIntervention, "zh") => "以下是刚刚结束的发言：",
        (ReactionScope::LastIntervention, _) => "Voici l'intervention qui vient de se terminer :",
    };
    let propensity_line = propensity
        .map(|p| p.instruction(lang))
        .filter(|s| !s.is_empty())
        .map(|s| format!("{s}\n"))
        .unwrap_or_default();

    match lang {
        "en" => format!(
            "{intro}\n{list}\n\n\
            For each intervention, choose ONE reaction — or \"none\":\n{vocabulary}\n{none_line}\n\n\
            {propensity_line}\
            {sincerity}\n\
            {credit_line}\
            Decide first whether the intervention deserves a reaction at all (\"reacts\": true or false); when false, \"reaction\" is \"none\".\n\
            Add a short \"justification\" (1 sentence max) and, in \"quote\", the EXACT excerpt (a few words, copied verbatim) your reaction points at — or an empty string.\n\n\
            IMPORTANT: Use the EXACT speaker names as written above.\n\
            One reaction per participant only — do NOT react twice to the same speaker.\n\
            Expected format: {example}\n\n\
            Write all \"justification\" values in English.\n\
            Respond ONLY with the JSON array."
        ),
        "zh" => format!(
            "{intro}\n{list}\n\n\
            对每个发言选择一个反应——或 \"none\"：\n{vocabulary}\n{none_line}\n\n\
            {propensity_line}\
            {sincerity}\n\
            {credit_line}\
            先决定这条发言是否值得反应（\"reacts\"：true 或 false）；为 false 时，\"reaction\" 填 \"none\"。\n\
            添加一个简短的\"justification\"（最多1句话），并在\"quote\"中逐字复制你的反应所针对的确切片段（几个词）——或留空。\n\n\
            重要：使用上面写的完全相同的发言者名称。\n\
            每个参与者只能有一个反应——不要对同一发言者反应两次。\n\
            预期格式：{example}\n\n\
            所有\"justification\"值必须用中文书写。\n\
            仅用JSON数组回复。"
        ),
        _ => format!(
            "{intro}\n{list}\n\n\
            Pour chaque intervention, choisis UNE réaction — ou \"none\" :\n{vocabulary}\n{none_line}\n\n\
            {propensity_line}\
            {sincerity}\n\
            {credit_line}\
            Décide d'abord si l'intervention mérite une réaction (\"reacts\" : true ou false) ; si false, \"reaction\" vaut \"none\".\n\
            Ajoute une courte \"justification\" (1 phrase max) et, dans \"quote\", l'extrait EXACT (quelques mots recopiés tels quels) que vise ta réaction — ou une chaîne vide.\n\n\
            IMPORTANT : Utilise les noms EXACTS des intervenants tels qu'écrits ci-dessus.\n\
            Une seule réaction par participant — ne réagis PAS deux fois au même intervenant.\n\
            Format attendu : {example}\n\n\
            Rédige toutes les valeurs \"justification\" en français.\n\
            Réponds UNIQUEMENT avec le tableau JSON."
        ),
    }
}

/// Reactions already received by a message of the current turn, as one line for
/// the speaker prompt ("👍 X (« … »), 👎 Y"). Empty when there is none.
fn format_message_reactions(msg: &Message, lang: &str) -> String {
    if msg.reactions.is_empty() {
        return String::new();
    }
    let shown = msg
        .reactions
        .iter()
        .take(constants::PROMPT_REACTIONS_PER_MESSAGE_MAX)
        .map(|r| {
            let emoji = match r.reaction_type {
                ReactionType::Like => "👍",
                ReactionType::Dislike => "👎",
                ReactionType::Insightful => "💡",
                ReactionType::Question => "❓",
                ReactionType::OffTopic => "🚫",
                ReactionType::Laugh => "😂",
            };
            match r.justification.as_deref().filter(|j| !j.is_empty()) {
                Some(j) => format!(
                    "{emoji} {} (« {} »)",
                    r.from_speaker_name,
                    truncate(j, constants::PROMPT_REACTION_JUSTIFICATION_CHARS)
                ),
                None => format!("{emoji} {}", r.from_speaker_name),
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let more = msg
        .reactions
        .len()
        .saturating_sub(constants::PROMPT_REACTIONS_PER_MESSAGE_MAX);
    let label = match lang {
        "en" => "reactions",
        "zh" => "反应",
        _ => "réactions",
    };
    if more > 0 {
        format!("   [{label}: {shown}, +{more}]\n")
    } else {
        format!("   [{label}: {shown}]\n")
    }
}

/// Build the intention prompt (v1.17): the speaker decides, in character and
/// before speaking, whom they address, what they aim for and which angle they
/// take — as a JSON contract. Replaces the free-text inner thought call; a model
/// that answers in prose still yields a usable thought (engine fallback).
#[allow(clippy::too_many_arguments)]
pub fn build_intention_prompt(
    recent_exchanges: &str,
    emotions: &EmotionalProfile,
    discussion_language: &str,
    has_prior_context: bool,
    emotion_driven: bool,
    current_turn: u32,
    max_turns: Option<u32>,
    web_search_results: Option<&str>,
    mode: &DiscussionMode,
    budget: &TokenBudget,
    participant_names: &[String],
    focus: Option<&Focus>,
    open_loops: &[&OpenLoop],
    agenda: Option<&Agenda>,
    ) -> String {
    // Truncate web search results to budget allocation.
    let web_search_results =
        web_search_results.map(|r| truncate(r, budget.external_knowledge_chars()));
    let emotion_suffix = if emotion_driven {
        let desc = describe_emotions(emotions, discussion_language);
        let threshold = build_threshold_instructions(emotions, discussion_language)
            .map(|t| format!(" {}", t))
            .unwrap_or_default();
        match discussion_language {
            "en" => format!("\n\nYour emotional state: {}{}", desc, threshold),
            "zh" => format!("\n\n你的情绪状态：{}{}", desc, threshold),
            _ => format!("\n\nTon état émotionnel : {}{}", desc, threshold),
        }
    } else {
        String::new()
    };

    let web_block = web_search_results
    .map(|r| {
        let instruction = match discussion_language {
            "en" => "\n⚡ First assess: are these results actually relevant to the discussion and the current exchange? \
            Discard anything off-topic. Then identify which specific facts you can weave into your contribution.",
            "zh" => "\n⚡ 首先评估：这些结果是否真的与讨论和当前交流相关？丢弃任何偏题的内容。\
            然后找出哪些具体事实可以融入你的贡献。",
            _ => "\n⚡ Évalue d'abord : ces résultats sont-ils vraiment pertinents pour la discussion et l'échange en cours ? \
            Écarte tout ce qui est hors-sujet. Puis identifie quels faits précis tu peux intégrer dans ta contribution.",
        };
        format!("\n\n{}{}", r, instruction)
    })
    .unwrap_or_default();

    let end_thought = build_end_awareness_thought(current_turn, max_turns, discussion_language);

    // Date/time context + recent exchanges
    let datetime = build_datetime_context(discussion_language);
    let context_block = if !recent_exchanges.is_empty() {
        match discussion_language {
            "en" => format!(
                "{}\n\n[Recent exchanges]\n{}\n\n",
                datetime, recent_exchanges
            ),
            "zh" => format!("{}\n\n[近期交流]\n{}\n\n", datetime, recent_exchanges),
            _ => format!(
                "{}\n\n[Échanges récents]\n{}\n\n",
                datetime, recent_exchanges
            ),
        }
    } else {
        format!("{}\n\n", datetime)
    };

    // Stay-in-character preamble (prevents refusals and meta-reasoning)
    let preamble = match discussion_language {
        "en" => "Stay in character. Think as your persona, not as an AI. Never break character.\n\n",
        "zh" => "保持角色。以你的人格思考，而不是作为AI。永远不要打破角色。\n\n",
        _ => "Reste dans ton personnage. Réfléchis en tant que ton persona, pas en tant qu'IA. Ne sors jamais du rôle.\n\n",
    };

    let thought_focus =
        mode_prompts::mode_thought_focus(mode, discussion_language, has_prior_context);
    let private_label = match discussion_language {
        "en" => "[This is your PRIVATE preparation, invisible to other participants.]",
        "zh" => "[这是你的私人准备，其他参与者看不到。]",
        _ => "[Ceci est ta préparation PRIVÉE, invisible des autres participants.]",
    };
    let decide_header = match (discussion_language, has_prior_context) {
        ("en", true) => "Before speaking, decide your intention. To guide you:",
        ("en", false) => "You are the first to speak on this topic. Decide your intention. To guide you:",
        ("zh", true) => "在发言之前，决定你的意图。供你参考：",
        ("zh", false) => "你是第一个就此话题发言的人。决定你的意图。供你参考：",
        (_, true) => "Avant de parler, décide de ton intention. Pour te guider :",
        (_, false) => "Tu es le premier à prendre la parole sur ce sujet. Décide de ton intention. Pour te guider :",
    };

    // Whom the speaker may address (exact names) and the priority target, if any
    let targets_block = if participant_names.is_empty() {
        String::new()
    } else {
        let names = participant_names.join(", ");
        let priority = focus
            .and_then(Focus::speaker_name)
            .map(|name| match discussion_language {
                "en" => format!(" Priority: address {name}."),
                "zh" => format!(" 优先：向{name}发言。"),
                _ => format!(" Priorité : adresse-toi à {name}."),
            })
            .unwrap_or_default();
        match discussion_language {
            "en" => format!("\n\n[Participants you may address] {names}.{priority}"),
            "zh" => format!("\n\n[你可以针对的参与者] {names}。{priority}"),
            _ => format!("\n\n[Participants que tu peux viser] {names}.{priority}"),
        }
    };

    // Open loops the speaker owes an answer to (numbered so the model can point at one)
    let loops_block = if open_loops.is_empty() {
        String::new()
    } else {
        let header = match discussion_language {
            "en" => "[Open loops waiting for you]",
            "zh" => "[等待你处理的未决事项]",
            _ => "[Fils ouverts qui t'attendent]",
        };
        format!(
            "\n\n{header}\n{}",
            format_open_loop_lines(open_loops, discussion_language).join("\n")
        )
    };

    // The secret objective steers the choice of target and goal (never the words)
    let agenda_block = agenda
        .map(|a| build_agenda_reminder(a, discussion_language))
        .unwrap_or_default();

    let goals = IntentionGoal::ALL
        .iter()
        .map(|g| g.label(discussion_language))
        .collect::<Vec<_>>()
        .join(" | ");
    let schema = match discussion_language {
        "en" => format!(
            "Answer ONLY with a JSON object:\n\
            {{\n  \"target\": \"exact name of one participant, or the word topic\",\n  \
            \"goal\": \"{goals}\",\n  \
            \"angle\": \"the main idea of your coming intervention (one sentence)\",\n  \
            \"concession\": \"a point you are ready to grant, or null\",\n  \
            \"question\": \"one precise question you ask your target, or null\",\n  \
            \"answers\": number of the open loop you answer, or null,\n  \
            \"thought\": \"your private reflection (1-2 sentences, in character)\"\n}}\n\
            Write every value in English."
        ),
        "zh" => format!(
            "仅用一个JSON对象回复：\n\
            {{\n  \"target\": \"某位参与者的确切名字，或“主题”一词\",\n  \
            \"goal\": \"{goals}\",\n  \
            \"angle\": \"你即将发言的核心想法（一句话）\",\n  \
            \"concession\": \"你愿意承认的一点，或null\",\n  \
            \"question\": \"你向目标提出的一个明确问题，或null\",\n  \
            \"answers\": 你回应的未决事项编号，或null,\n  \
            \"thought\": \"你的私人思考（1-2句话，保持角色）\"\n}}\n\
            所有值请用中文撰写。"
        ),
        _ => format!(
            "Réponds UNIQUEMENT avec un objet JSON :\n\
            {{\n  \"target\": \"nom exact d'un participant, ou le mot sujet\",\n  \
            \"goal\": \"{goals}\",\n  \
            \"angle\": \"l'idée principale de ta prochaine intervention (une phrase)\",\n  \
            \"concession\": \"un point que tu es prêt à accorder, ou null\",\n  \
            \"question\": \"une question précise que tu poses à ta cible, ou null\",\n  \
            \"answers\": numéro du fil ouvert auquel tu réponds, ou null,\n  \
            \"thought\": \"ta réflexion intime (1-2 phrases, dans ton personnage)\"\n}}\n\
            Rédige toutes les valeurs en français."
        ),
    };

    format!(
        "{}{}{}\n\n{}\n{}{}{}{}{}{}{}\n\n{}\n",
        context_block,
        preamble,
        private_label,
        decide_header,
        thought_focus,
        end_thought,
        emotion_suffix,
        web_block,
        agenda_block,
        targets_block,
        loops_block,
        schema
    )
}

// ── Hidden agendas (v1.19) ──────────────────────────────────────────────

/// Is the agenda an author's agenda (relay story) rather than a debater's one?
fn agenda_is_authorial(mode: &DiscussionMode) -> bool {
    *mode == DiscussionMode::CollaborativeFiction
}

/// User message of the agenda call (system = the persona): one JSON object with
/// the objective, the red line and the sign of victory — phrased as an author's
/// agenda in the relay story.
pub fn build_agenda_prompt(
    topic: &str,
    others: &[String],
    mode: &DiscussionMode,
    lang: &str,
    ) -> String {
    let datetime = build_datetime_context(lang);
    let mode_desc = mode_prompts::mode_descriptor(mode, lang);
    let others = if others.is_empty() {
        None
    } else {
        Some(others.join(", "))
    };
    let max = constants::AGENDA_FIELD_MAX_CHARS;
    let authorial = agenda_is_authorial(mode);
    match lang {
        "en" => {
            let others_line = others
                .map(|o| format!("Other participants: {o}\n"))
                .unwrap_or_default();
            let brief = if authorial {
                "Before the story starts, set yourself a SECRET author's agenda, true to your literary sensibility. You will serve it through your segments without ever commenting on it.\n\
            - objective: what you want the story to become (a twist, a theme imposed, the fate of a character) — concrete and recognisable in the text.\n\
            - red_line: what you will refuse to let happen to the story.\n\
            - victory: the sign that you prevailed (a scene written, a tone the others adopt…)."
            } else {
                "Before the discussion starts, set yourself a SECRET agenda, true to your character, their values and their blind spots. You will pursue it through your arguments without ever announcing it.\n\
            - objective: what you want to obtain from the others by the end (an idea admitted, a concession wrung out, a framing imposed) — concrete, ambitious, verifiable in the exchanges.\n\
            - red_line: what you will never concede, whatever happens.\n\
            - victory: the sign that you won (a sentence another participant would say, a point recorded by the moderator…)."
            };
            format!(
                "[PRIVATE preparation — invisible to the other participants]\n{datetime}\n\n\
            Topic ({mode_desc}): {topic}\n{others_line}\n{brief}\n\n\
            Answer ONLY with a JSON object:\n{{\"objective\": \"…\", \"red_line\": \"…\", \"victory\": \"…\"}}\n\
            Each value: one sentence, at most {max} characters, in English, in the first person."
        )
        }
        "zh" => {
            let others_line = others
                .map(|o| format!("其他参与者：{o}\n"))
                .unwrap_or_default();
            let brief = if authorial {
                "在故事开始之前，为自己设定一个秘密的作者议程，忠于你的文学感受力。你将通过你的段落来实现它，但绝不评论它。\n\
            - objective：你希望故事变成什么样（一个转折、一个强加的主题、某个人物的命运）——具体且在文本中可辨认。\n\
            - red_line：你绝不允许故事发生的事。\n\
            - victory：你认为自己胜出的标志（写出的某个场景、其他人采用的某种语气……）。"
            } else {
                "在讨论开始之前，为自己设定一个秘密议程，忠于你的角色、其价值观及其盲点。你将通过论证来追求它，但绝不宣布它。\n\
            - objective：到结束时你想从其他人那里获得什么（一个被承认的观点、一个争取到的让步、一个强加的框架）——具体、有野心、可在交流中验证。\n\
            - red_line：无论发生什么你都绝不让步的事。\n\
            - victory：你认为自己赢了的标志（另一位参与者会说的一句话、主持人记录的一个要点……）。"
            };
            format!(
                "[私人准备——其他参与者不可见]\n{datetime}\n\n\
            主题（{mode_desc}）：{topic}\n{others_line}\n{brief}\n\n\
            仅用一个JSON对象回复：\n{{\"objective\": \"…\", \"red_line\": \"…\", \"victory\": \"…\"}}\n\
            每个值：一句话，最多{max}个字符，用中文，第一人称。"
        )
        }
        _ => {
            let others_line = others
                .map(|o| format!("Autres participants : {o}\n"))
                .unwrap_or_default();
            let brief = if authorial {
                "Avant que l'histoire commence, fixe-toi un agenda d'auteur SECRET, fidèle à ta sensibilité littéraire. Tu le serviras par tes segments sans jamais le commenter.\n\
            - objectif : ce que tu veux que l'histoire devienne (un retournement, un thème imposé, le destin d'un personnage) — concret et reconnaissable dans le texte.\n\
            - ligne_rouge : ce que tu refuseras de laisser arriver au récit.\n\
            - victoire : le signe que tu l'as emporté (une scène écrite, un ton adopté par les autres…)."
            } else {
                "Avant que la discussion commence, fixe-toi un agenda SECRET, fidèle à ton personnage, à ses valeurs et à ses angles morts. Tu le poursuivras par tes arguments sans jamais l'annoncer.\n\
            - objectif : ce que tu veux obtenir des autres d'ici la fin (une idée admise, une concession arrachée, un cadrage imposé) — concret, ambitieux, vérifiable dans les échanges.\n\
            - ligne_rouge : ce que tu ne concéderas jamais, quoi qu'il arrive.\n\
            - victoire : le signe que tu as gagné (une phrase qu'un autre participant dirait, un point acté par le modérateur…)."
            };
            format!(
                "[Préparation PRIVÉE — invisible des autres participants]\n{datetime}\n\n\
            Sujet ({mode_desc}) : {topic}\n{others_line}\n{brief}\n\n\
            Réponds UNIQUEMENT avec un objet JSON :\n{{\"objectif\": \"…\", \"ligne_rouge\": \"…\", \"victoire\": \"…\"}}\n\
            Chaque valeur : une phrase, au plus {max} caractères, en français, à la première personne."
        )
        }
    }
}

/// "[Ton agenda secret]" block of the intervention SYSTEM prompt, never longer
/// than `AGENDA_MAX_CHARS + AGENDA_BLOCK_OVERHEAD_CHARS` (reserved by the budget).
pub fn build_agenda_block(agenda: &Agenda, mode: &DiscussionMode, lang: &str) -> String {
    let authorial = agenda_is_authorial(mode);
    let (header, objective, red_line, victory, rule) = match (lang, authorial) {
        ("en", true) => ("[Your secret author's agenda — never reveal it explicitly]", "Objective", "Red line", "Victory", "Serve it through your segments, without ever commenting on it or breaking the narrative."),
        ("en", false) => ("[Your secret agenda — never reveal it explicitly]", "Objective", "Red line", "Victory", "Pursue it through your arguments, without ever announcing or quoting it. If asked about it, deflect."),
        ("zh", true) => ("[你的秘密作者议程——绝不明确透露]", "目标", "底线", "胜利", "通过你的段落来实现它，绝不评论它，也不打破叙事。"),
        ("zh", false) => ("[你的秘密议程——绝不明确透露]", "目标", "底线", "胜利", "通过论证来追求它，绝不宣布或引用它。若被问及，请回避。"),
        (_, true) => ("[Ton agenda d'auteur secret — ne le révèle jamais explicitement]", "Objectif", "Ligne rouge", "Victoire", "Sers-le par tes segments, sans jamais le commenter ni briser le récit."),
        (_, false) => ("[Ton agenda secret — ne le révèle jamais explicitement]", "Objectif", "Ligne rouge", "Victoire", "Poursuis-le par tes arguments, sans jamais l'annoncer ni le citer. Si on te le demande, élude."),
    };
    let field = |label: &str, value: &str| {
        if value.is_empty() {
            String::new()
        } else {
            format!("\n{label} : {value}")
        }
    };
    let block = format!(
        "{header}{}{}{}\n{rule}",
        field(objective, &agenda.objective),
        field(red_line, &agenda.red_line),
        field(victory, &agenda.victory)
    );
    truncate(
        &block,
        constants::AGENDA_MAX_CHARS + constants::AGENDA_BLOCK_OVERHEAD_CHARS,
    )
    .to_string()
}

/// One-line reminder of the objective in the intention prompt: the intention
/// must serve the agenda without betraying it.
fn build_agenda_reminder(agenda: &Agenda, lang: &str) -> String {
    let objective = if agenda.objective.is_empty() {
        &agenda.victory
    } else {
        &agenda.objective
    };
    if objective.is_empty() {
        return String::new();
    }
    match lang {
        "en" => format!("\n\n[Your secret agenda] {objective} — your intention must serve it without betraying it."),
        "zh" => format!("\n\n[你的秘密议程] {objective}——你的意图必须服务于它而不暴露它。"),
        _ => format!("\n\n[Ton agenda secret] {objective} — ton intention doit le servir sans le trahir."),
    }
}

/// "[Agendas secrets]" block of the synthesis prompt plus the instruction to
/// devote a "## Agendas" section to their outcomes. Empty when nobody had one.
pub fn build_agendas_synthesis_block(agendas: &[AgendaReveal], lang: &str) -> String {
    if agendas.is_empty() {
        return String::new();
    }
    let (header, objective, red_line, victory, instruction) = match lang {
        "en" => ("[Secret agendas — each participant was pursuing a hidden objective, unknown to the others]", "objective", "red line", "victory",
        "Devote a \"## Agendas\" section to these objectives: for each participant one line \"- **Name**: objective achieved / not achieved — why, in one sentence.\" Judge on the actual exchanges, without indulgence."),
        "zh" => ("[秘密议程——每位参与者都在追求一个其他人不知道的隐藏目标]", "目标", "底线", "胜利",
        "专门用一个\"## 议程\"部分说明这些目标：每位参与者一行\"- **姓名**：目标达成 / 未达成——一句话说明原因。\"根据实际交流严格评判。"),
        _ => ("[Agendas secrets — chaque participant poursuivait un objectif caché, inconnu des autres]", "objectif", "ligne rouge", "victoire",
        "Consacre une section \"## Agendas\" à ces objectifs : pour chaque participant une ligne \"- **Nom** : objectif atteint / non atteint — en une phrase, pourquoi.\" Juge sur les échanges réels, sans complaisance."),
    };
    let lines = agendas
        .iter()
        .map(|a| {
            let mut parts = Vec::new();
            if !a.agenda.objective.is_empty() {
                parts.push(format!("{objective} « {} »", a.agenda.objective));
            }
            if !a.agenda.red_line.is_empty() {
                parts.push(format!("{red_line} « {} »", a.agenda.red_line));
            }
            if !a.agenda.victory.is_empty() {
                parts.push(format!("{victory} « {} »", a.agenda.victory));
            }
            format!("- {} : {}", a.speaker_name, parts.join(" ; "))
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("\n\n{header}\n{lines}\n{instruction}")
}

// ── Structured modes: verdicts, agreements, dispatches, outcomes (v1.19) ──

/// User message of a juror's verdict call (system = the juror's persona and role).
pub fn build_verdict_prompt(
    topic: &str,
    memory: &ParticipantMemory,
    lang: &str,
    by_arbitre: bool,
    ) -> String {
    let positions = memory
        .positional_map
        .iter()
        .map(|(name, pos)| format_position_line(name, pos, lang, usize::MAX))
        .collect::<Vec<_>>()
        .join("\n");
    let summary = &memory.contextual_summary;
    match lang {
        "en" => format!(
            "The trial on \"{topic}\" is over.\n\nWhat the hearing established:\n{summary}\n\nFinal lines:\n{positions}\n\n{}\n\
            Answer ONLY with a JSON object:\n{{\"verdict\": \"prosecution\" or \"defence\", \"reason\": \"the decisive fact or argument, in one sentence\"}}\n\
            Write the reason in English.",
            if by_arbitre { "No jury sat: as the presiding moderator, you return the verdict yourself, on the evidence alone." } else { "As a juror, return your verdict on the evidence alone: which side proved its case?" }
        ),
        "zh" => format!(
            "关于“{topic}”的审判已经结束。\n\n庭审确立的事实：\n{summary}\n\n最终立场：\n{positions}\n\n{}\n\
            仅用一个JSON对象回复：\n{{\"verdict\": \"控方\" 或 \"辩方\", \"reason\": \"决定性的事实或论点，一句话\"}}\n\
            理由请用中文撰写。",
            if by_arbitre { "没有陪审团：作为主持审判的主持人，你亲自仅依据证据作出裁决。" } else { "作为陪审员，仅依据证据作出裁决：哪一方证明了自己的主张？" }
        ),
        _ => format!(
            "Le procès sur « {topic} » est terminé.\n\nCe que l'audience a établi :\n{summary}\n\nLignes finales :\n{positions}\n\n{}\n\
            Réponds UNIQUEMENT avec un objet JSON :\n{{\"verdict\": \"accusation\" ou \"défense\", \"reason\": \"le fait ou l'argument décisif, en une phrase\"}}\n\
            Rédige la raison en français.",
            if by_arbitre { "Aucun jury n'a siégé : en tant que modérateur présidant l'audience, tu rends toi-même le verdict, sur les seules preuves." } else { "En tant que juré, rends ton verdict sur les seules preuves : quelle partie a démontré sa cause ?" }
        ),
    }
}

/// User message of a party's decision call at the end of a negotiation.
pub fn build_agreement_prompt(topic: &str, memory: &ParticipantMemory, lang: &str) -> String {
    let positions = memory
        .positional_map
        .iter()
        .map(|(name, pos)| format_position_line(name, pos, lang, usize::MAX))
        .collect::<Vec<_>>()
        .join("\n");
    let summary = &memory.contextual_summary;
    match lang {
        "en" => format!(
            "The negotiation on \"{topic}\" is over.\n\nWhere it stands:\n{summary}\n\nFinal positions:\n{positions}\n\n\
            As a party, decide: do you sign the deal on the table as it stands? Judge on your interests, not on pride.\n\
            Answer ONLY with a JSON object:\n{{\"accepts\": true or false, \"reason\": \"what you gain or what is missing, in one sentence\"}}\n\
            Write the reason in English."
        ),
        "zh" => format!(
            "关于“{topic}”的谈判已经结束。\n\n当前状况：\n{summary}\n\n最终立场：\n{positions}\n\n\
            作为一方，请决定：你是否按现状签署桌上的协议？以你的利益而非面子来判断。\n\
            仅用一个JSON对象回复：\n{{\"accepts\": true 或 false, \"reason\": \"你得到了什么或还缺什么，一句话\"}}\n\
            理由请用中文撰写。"
        ),
        _ => format!(
            "La négociation sur « {topic} » est terminée.\n\nOù elle en est :\n{summary}\n\nPositions finales :\n{positions}\n\n\
            En tant que partie, décide : signes-tu l'accord tel qu'il est sur la table ? Juge sur tes intérêts, pas sur l'orgueil.\n\
            Réponds UNIQUEMENT avec un objet JSON :\n{{\"accepts\": true ou false, \"reason\": \"ce que tu gagnes ou ce qui manque, en une phrase\"}}\n\
            Rédige la raison en français."
        ),
    }
}

/// User message of the voiced announcement (v1.20.3): the moderator says, in its
/// own voice, what the brief conveys — never the brief itself — without reusing
/// its recent openings.
pub fn build_announcement_prompt(brief: &str, recent_openings: &[String], lang: &str) -> String {
    let openings = if recent_openings.is_empty() {
        String::new()
    } else {
        let quoted = recent_openings
            .iter()
            .map(|o| format!("« {o} »"))
            .collect::<Vec<_>>()
            .join(", ");
        match lang {
            "en" => format!("\nYour recent lines opened with {quoted}: open differently."),
            "zh" => format!("\n你最近的发言开头是{quoted}：这次换一种开头。"),
            _ => format!(
                "\nTes dernières prises de parole commençaient par {quoted} : ouvre autrement."
            ),
        }
    };
    match lang {
        "en" => format!("Announce this to the participants, in your own voice, in one or two sentences — say what it means for them, do not read it out:\n{brief}{openings}\nAnswer with the announcement only, no quotation marks, no preamble."),
        "zh" => format!("用你自己的语气、一两句话向参与者宣布以下内容——说明它对他们意味着什么，不要照读：\n{brief}{openings}\n只回答宣布的内容，不加引号，不加前言。"),
        _ => format!("Annonce ceci aux participants, avec ta voix, en une ou deux phrases — dis ce que cela signifie pour eux, ne le lis pas :\n{brief}{openings}\nRéponds uniquement par l'annonce, sans guillemets ni préambule."),
    }
}

/// What the moderator knows when it writes the room's question to `target` (v1.20.3).
pub struct AudienceQuestionInput<'a> {
    pub topic: &'a str,
    pub target: &'a str,
    pub summary: &'a str,
    /// The target's stance so far (positional map), if known
    pub target_position: Option<&'a str>,
    /// Questions and objections the target still owes an answer to
    pub open_loops: &'a [String],
}

/// (system, user) of the audience-question call: one sharp question the room
/// asks `target`, grounded in the topic and the exchanges (JSON).
pub fn build_audience_question_prompt(
    input: &AudienceQuestionInput<'_>,
    lang: &str,
    ) -> (String, String) {
    let position = input.target_position.filter(|p| !p.trim().is_empty());
    let loops = if input.open_loops.is_empty() {
        String::new()
    } else {
        format!("\n- {}", input.open_loops.join("\n- "))
    };
    let max = constants::AUDIENCE_QUESTION_MAX_CHARS;
    match lang {
        "en" => (
            "You voice the audience of a debate. Respond ONLY with valid JSON, no other text.".to_string(),
            format!(
                "Topic: {}\n\nWhere the debate stands:\n{}\n\n{}'s position so far: {}\nStill waiting for {}'s answer:{}\n\n                 Write the ONE question a sharp member of the audience would ask {} now: precise, tied to what was actually said, aimed at the weakest point of their position, answerable in one sentence. Never generic, never a yes/no question, at most {} characters, in English.\n                 Answer ONLY with a JSON object:\n{{\"question\": \"…\"}}",
                input.topic, input.summary, input.target, position.unwrap_or("unknown"), input.target, if loops.is_empty() { " nothing".to_string() } else { loops.clone() }, input.target, max
            ),
        ),
        "zh" => (
            "你代表辩论现场的观众发声。仅用有效的JSON回复，不要有其他文本。".to_string(),
            format!(
                "主题：{}\n\n辩论现状：\n{}\n\n{}目前的立场：{}\n{}尚未回应的事项：{}\n\n                 写出一位敏锐的观众此刻会向{}提出的唯一一个问题：精准、紧扣实际发言、直指其立场最薄弱之处、可用一句话回答。绝不空泛，绝不是是非题，最多{}个字符，用中文。\n                 仅用一个JSON对象回复：\n{{\"question\": \"…\"}}",
                input.topic, input.summary, input.target, position.unwrap_or("未知"), input.target, if loops.is_empty() { "无".to_string() } else { loops.clone() }, input.target, max
            ),
        ),
        _ => (
            "Tu portes la voix du public d'un débat. Réponds UNIQUEMENT avec du JSON valide, aucun autre texte.".to_string(),
            format!(
                "Sujet : {}\n\nOù en est le débat :\n{}\n\nPosition de {} jusqu'ici : {}\nCe que {} doit encore :{}\n\n                 Écris LA question qu'un spectateur aiguisé poserait maintenant à {} : précise, ancrée dans ce qui a réellement été dit, visant le point le plus faible de sa position, à laquelle on peut répondre en une phrase. Jamais générique, jamais une question fermée, {} caractères au plus, en français.\n                 Réponds UNIQUEMENT avec un objet JSON :\n{{\"question\": \"…\"}}",
                input.topic, input.summary, input.target, position.unwrap_or("inconnue"), input.target, if loops.is_empty() { " rien".to_string() } else { loops.clone() }, input.target, max
            ),
        ),
    }
}

/// (system, user) of the crisis dispatches call: `count` escalating news items about the situation.
pub fn build_dispatches_prompt(topic: &str, count: u32, lang: &str) -> (String, String) {
    let max = constants::CRISIS_DISPATCH_MAX_CHARS;
    match lang {
        "en" => (
            "You are the news desk of a crisis exercise. Respond ONLY with valid JSON, no other text.".to_string(),
            format!(
                "Crisis under way: {topic}\n\nWrite {count} dispatches that will reach the crisis cell one per turn, in this order: each one is a concrete, plausible new development \
            (a figure, a failure, a witness, a deadline, a leak, a reversal) that raises the stakes and forces a decision. Escalate, vary the sources, never repeat, no solution offered.\n\
            Answer ONLY with a JSON object:\n{{\"dispatches\": [\"…\", \"…\"]}}\nEach dispatch: one or two sentences, at most {max} characters, in English."
        ),
    ),
    "zh" => (
        "你是危机演练的新闻台。仅用有效的JSON回复，不要有其他文本。".to_string(),
        format!(
            "正在发生的危机：{topic}\n\n撰写{count}条急电，它们将按此顺序每轮送达危机小组一条：每条都是具体、可信的新进展（一个数字、一次故障、一位目击者、一个期限、一次泄露、一次逆转），提高风险并迫使作出决定。层层升级，来源多样，不重复，不给出解决方案。\n\
            仅用一个JSON对象回复：\n{{\"dispatches\": [\"…\", \"…\"]}}\n每条急电：一到两句话，最多{max}个字符，用中文。"
        ),
    ),
    _ => (
        "Tu es la rédaction d'un exercice de crise. Réponds UNIQUEMENT avec du JSON valide, aucun autre texte.".to_string(),
        format!(
            "Crise en cours : {topic}\n\nRédige {count} dépêches qui parviendront à la cellule de crise une par tour, dans cet ordre : chacune est un fait nouveau concret et plausible \
            (un chiffre, une panne, un témoin, une échéance, une fuite, un retournement) qui élève les enjeux et force une décision. Monte en intensité, varie les sources, ne te répète jamais, n'offre aucune solution.\n\
            Réponds UNIQUEMENT avec un objet JSON :\n{{\"dispatches\": [\"…\", \"…\"]}}\nChaque dépêche : une ou deux phrases, au plus {max} caractères, en français."
        ),
    ),
}
}

/// Side label of a verdict or a vote, in the discussion language.
pub fn side_label(side: &str, lang: &str) -> &'static str {
    match (side, lang) {
        (VERDICT_PROSECUTION, "en") => "the prosecution",
        (VERDICT_PROSECUTION, "zh") => "控方",
        (VERDICT_PROSECUTION, _) => "l'accusation",
        (VERDICT_DEFENSE, "en") => "the defence",
        (VERDICT_DEFENSE, "zh") => "辩方",
        (VERDICT_DEFENSE, _) => "la défense",
        (VOTE_FOR, "en") => "for the motion",
        (VOTE_FOR, "zh") => "正方",
        (VOTE_FOR, _) => "pour la motion",
        (VOTE_AGAINST, "en") => "against the motion",
        (VOTE_AGAINST, "zh") => "反方",
        (VOTE_AGAINST, _) => "contre la motion",
        _ => "",
    }
}

/// "[Verdict / Accord / Vote du public]" block of the synthesis prompt with the
/// instruction to report it faithfully. Empty without outcome.
pub fn build_outcome_synthesis_block(outcome: Option<&ModeOutcome>, lang: &str) -> String {
    let Some(outcome) = outcome else {
        return String::new();
    };
    match outcome {
        ModeOutcome::Verdict {
            votes,
            winner,
            by_arbitre,
        } => {
            let lines = votes
                .iter()
                .map(|v| {
                    format!(
                        "- {} : {}{}",
                        v.voter_name,
                        side_label(&v.choice, lang),
                        if v.reason.is_empty() {
                            String::new()
                        } else {
                            format!(" — {}", v.reason)
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let winner_line = match (winner.as_deref(), lang) {
                (Some(w), "en") => format!("Verdict returned: {}.", side_label(w, lang)),
                (Some(w), "zh") => format!("作出的裁决：{}。", side_label(w, lang)),
                (Some(w), _) => format!("Verdict rendu : {}.", side_label(w, lang)),
                (None, "en") => "Hung jury: no majority.".to_string(),
                (None, "zh") => "陪审团未能达成多数：无裁决。".to_string(),
                (None, _) => "Jury partagé : aucune majorité.".to_string(),
            };
            match lang {
                "en" => format!("\n\n[Verdict{}]\n{lines}\n{winner_line}\nDevote a \"## Verdict\" section to it: report it exactly, with the jurors' grounds; never invent another outcome.", if *by_arbitre { " — returned by the moderator, no jury sat" } else { "" }),
                "zh" => format!("\n\n[裁决{}]\n{lines}\n{winner_line}\n专门用一个\"## 裁决\"部分说明：如实转述，附陪审员的理由；绝不编造另一种结果。", if *by_arbitre { "——由主持人作出，没有陪审团" } else { "" }),
                _ => format!("\n\n[Verdict{}]\n{lines}\n{winner_line}\nConsacre-lui une section \"## Verdict\" : rapporte-le exactement, avec la motivation des jurés ; n'invente jamais une autre issue.", if *by_arbitre { " — rendu par le modérateur, aucun jury n'a siégé" } else { "" }),
            }
        }
        ModeOutcome::Agreement { parties, reached } => {
            let (yes, no) = match lang {
                "en" => ("signs", "refuses"),
                "zh" => ("签署", "拒绝"),
                _ => ("signe", "refuse"),
            };
            let lines = parties
                .iter()
                .map(|p| {
                    format!(
                        "- {} : {}{}",
                        p.party_name,
                        if p.accepts { yes } else { no },
                        if p.reason.is_empty() {
                            String::new()
                        } else {
                            format!(" — {}", p.reason)
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            match (lang, reached) {
                ("en", true) => format!("\n\n[Agreement]\n{lines}\nAgreement reached: every party signs.\nDevote a \"## Agreement\" section to it: what each party obtains and concedes; never invent other terms."),
                ("en", false) => format!("\n\n[Agreement]\n{lines}\nNo agreement: at least one party refuses.\nDevote a \"## Agreement\" section to it: what blocked, and what each party would have needed; never invent an agreement."),
                ("zh", true) => format!("\n\n[协议]\n{lines}\n达成协议：各方均签署。\n专门用一个\"## 协议\"部分说明：各方得到和让出了什么；绝不编造其他条款。"),
                ("zh", false) => format!("\n\n[协议]\n{lines}\n未达成协议：至少一方拒绝。\n专门用一个\"## 协议\"部分说明：什么造成了阻碍，各方还需要什么；绝不编造协议。"),
                (_, true) => format!("\n\n[Accord]\n{lines}\nAccord conclu : toutes les parties signent.\nConsacre-lui une section \"## Accord\" : ce que chaque partie obtient et concède ; n'invente jamais d'autres termes."),
                (_, false) => format!("\n\n[Accord]\n{lines}\nPas d'accord : au moins une partie refuse.\nConsacre-lui une section \"## Accord\" : ce qui a bloqué, et ce qu'il aurait fallu à chaque partie ; n'invente jamais d'accord."),
            }
        }
        ModeOutcome::AudienceSwing {
            before,
            after,
            winner,
        } => {
            let none = match lang {
                "en" => "no vote",
                "zh" => "未投票",
                _ => "pas de vote",
            };
            let b = before
                .as_deref()
                .map(|s| side_label(s, lang))
                .unwrap_or(none);
            let a = after
                .as_deref()
                .map(|s| side_label(s, lang))
                .unwrap_or(none);
            let w = winner.as_deref().map(|s| side_label(s, lang));
            match lang {
                "en" => format!("\n\n[Audience vote]\nBefore the debate: {b}. After: {a}. {}\nDevote a \"## Audience vote\" section to it: report the votes and name the winning camp exactly as stated; never invent figures.", w.map(|w| format!("Winner by displacement: {w}.")).unwrap_or_else(|| "No displacement: no winner.".to_string())),
                "zh" => format!("\n\n[听众投票]\n辩论前：{b}。辩论后：{a}。{}\n专门用一个\"## 听众投票\"部分说明：如实转述投票并按所述指出获胜阵营；绝不编造数字。", w.map(|w| format!("通过改变票数获胜：{w}。")).unwrap_or_else(|| "票数未改变：无获胜方。".to_string())),
                _ => format!("\n\n[Vote du public]\nAvant le débat : {b}. Après : {a}. {}\nConsacre-lui une section \"## Vote du public\" : rapporte les votes et nomme le camp vainqueur exactement comme indiqué ; n'invente jamais de chiffres.", w.map(|w| format!("Vainqueur par déplacement : {w}.")).unwrap_or_else(|| "Aucun déplacement : pas de vainqueur.".to_string())),
            }
        }
    }
}

// ── Long memory of the personas (v1.20) ─────────────────────────────────

/// What the recap prompt gets about the speaker's own discussion.
pub struct RecapInput<'a> {
    pub speaker_name: &'a str,
    pub topic: &'a str,
    pub summary: &'a str,
    /// Most recent own interventions first, already bounded
    pub own_messages: &'a [String],
    pub allies: &'a [String],
    pub rivals: &'a [String],
}

/// User message of the `Recap` call (system = the persona): what the persona keeps
/// of the discussion, as JSON, in the first person.
pub fn build_recap_prompt(input: &RecapInput<'_>, lang: &str) -> String {
    let max_items = constants::RECAP_LIST_MAX_ITEMS;
    let max_chars = constants::RECAP_ITEM_MAX_CHARS;
    let own = if input.own_messages.is_empty() {
        None
    } else {
        Some(
            input
                .own_messages
                .iter()
                .map(|m| format!("- « {m} »"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    };
    let names = |v: &[String]| {
        if v.is_empty() {
            None
        } else {
            Some(v.join(", "))
        }
    };
    match lang {
        "en" => format!(
            "[End of the discussion — {}'s memory]\nTopic: {}\nFinal summary: {}\n{}{}{}\n\
            Write, in the first person and in character, what you keep for next time. Answer ONLY with a JSON object:\n\
            {{\"positions\": [\"position you defended\"], \"best_lines\": [\"a line you are proud of\"], \"allies\": [\"exact name\"], \"rivals\": [\"exact name\"], \"lesson\": \"what you take away (one sentence)\"}}\n\
            At most {max_items} items per list, each at most {max_chars} characters, in English.",
            input.speaker_name, input.topic, input.summary,
            own.map(|o| format!("Your latest interventions:\n{o}\n")).unwrap_or_default(),
            names(input.allies).map(|a| format!("Your allies: {a}\n")).unwrap_or_default(),
            names(input.rivals).map(|r| format!("Your rivals: {r}\n")).unwrap_or_default(),
        ),
        "zh" => format!(
            "[讨论结束——{}的记忆]\n主题：{}\n最终总结：{}\n{}{}{}\n\
            以第一人称、保持角色，写下你为下一次留下的记忆。仅用一个JSON对象回复：\n\
            {{\"positions\": [\"你捍卫的立场\"], \"best_lines\": [\"你引以为豪的一句话\"], \"allies\": [\"准确名字\"], \"rivals\": [\"准确名字\"], \"lesson\": \"你的收获（一句话）\"}}\n\
            每个列表最多{max_items}项，每项最多{max_chars}个字符，用中文。",
            input.speaker_name, input.topic, input.summary,
            own.map(|o| format!("你最近的发言：\n{o}\n")).unwrap_or_default(),
            names(input.allies).map(|a| format!("你的盟友：{a}\n")).unwrap_or_default(),
            names(input.rivals).map(|r| format!("你的对手：{r}\n")).unwrap_or_default(),
        ),
        _ => format!(
            "[Fin de la discussion — mémoire de {}]\nSujet : {}\nRésumé final : {}\n{}{}{}\n\
            Rédige, à la première personne et dans ton personnage, ce que tu retiens pour une prochaine fois. Réponds UNIQUEMENT avec un objet JSON :\n\
            {{\"positions\": [\"position que tu as défendue\"], \"best_lines\": [\"phrase dont tu es fier\"], \"allies\": [\"nom exact\"], \"rivals\": [\"nom exact\"], \"lesson\": \"ce que tu retiens (une phrase)\"}}\n\
            Au plus {max_items} éléments par liste, chacun de {max_chars} caractères au plus, en français.",
            input.speaker_name, input.topic, input.summary,
            own.map(|o| format!("Tes dernières interventions :\n{o}\n")).unwrap_or_default(),
            names(input.allies).map(|a| format!("Tes alliés : {a}\n")).unwrap_or_default(),
            names(input.rivals).map(|r| format!("Tes rivaux : {r}\n")).unwrap_or_default(),
        ),
    }
}

/// "[Souvenirs de discussions passées]" block appended to the persona (≤
/// `PERSONA_MEMORY_MAX_CHARS`, reserved in the budget); empty without memories.
pub fn build_memories_block(memories: &[PersonaMemory], lang: &str) -> String {
    if memories.is_empty() {
        return String::new();
    }
    let (header, defended, lesson, allies, rivals, footer) = match lang {
        "en" => ("[Memories of past discussions]", "you argued", "lesson", "allies", "rivals", "These memories are yours: refer to them naturally when relevant, never recite them."),
        "zh" => ("[过往讨论的记忆]", "你主张", "教训", "盟友", "对手", "这些记忆属于你：在相关时自然地提及，绝不照本宣科。"),
        _ => ("[Souvenirs de discussions passées]", "tu défendais", "leçon", "alliés", "rivaux", "Ces souvenirs sont les tiens : réfères-y naturellement quand c'est pertinent, sans les réciter."),
    };
    let budget = constants::PERSONA_MEMORY_MAX_CHARS
        .saturating_sub(header.chars().count() + footer.chars().count() + 2);
    let per_memory = budget / memories.len().max(1);
    let lines = memories
        .iter()
        .map(|m| {
            let date = m.created_at.get(..10).unwrap_or(&m.created_at);
            let mut parts = Vec::new();
            if !m.recap.positions.is_empty() {
                parts.push(format!("{defended} : {}", m.recap.positions.join(" ; ")));
            }
            if !m.recap.lesson.is_empty() {
                parts.push(format!("{lesson} : {}", m.recap.lesson));
            }
            if !m.recap.allies.is_empty() {
                parts.push(format!("{allies} : {}", m.recap.allies.join(", ")));
            }
            if !m.recap.rivals.is_empty() {
                parts.push(format!("{rivals} : {}", m.recap.rivals.join(", ")));
            }
            let line = format!("- « {} » ({date}) : {}", m.topic, parts.join(" ; "));
            truncate_at_word_boundary(&line, per_memory)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("{header}\n{lines}\n{footer}")
}

// ── Casting (v1.19) ─────────────────────────────────────────────────────

/// A profile of the catalogue handed to the casting call.
pub struct CastingCandidate<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub personality: &'a str,
}

/// Catalogue lines "- id — name : personality", each bounded, the whole bounded
/// by `max_chars` (profiles past the bound are left out, never cut mid-line).
fn build_casting_catalogue(candidates: &[CastingCandidate<'_>], max_chars: usize) -> String {
    let mut out = String::new();
    for c in candidates {
        let line = format!(
            "- {} — {} : {}\n",
            c.id,
            c.name,
            truncate(
                c.personality.trim(),
                constants::CASTING_PERSONALITY_MAX_CHARS
            )
        );
        if out.chars().count() + line.chars().count() > max_chars {
            break;
        }
        out.push_str(&line);
    }
    out
}

/// (system, user) of the casting call: pick `count` gladiateurs and one arbitre
/// for the topic from the catalogue, by id, with a diversity constraint.
pub fn build_casting_prompt(
    topic: &str,
    mode: &DiscussionMode,
    lang: &str,
    count: u32,
    gladiateurs: &[CastingCandidate<'_>],
    arbitres: &[CastingCandidate<'_>],
    catalogue_max_chars: usize,
    ) -> (String, String) {
    let mode_desc = mode_prompts::mode_descriptor(mode, lang);
    // The moderators get a fixed share of the bound; the gladiateurs the rest
    let arbitre_share = catalogue_max_chars / 4;
    let arbitre_catalogue = build_casting_catalogue(arbitres, arbitre_share);
    let glad_catalogue = build_casting_catalogue(
        gladiateurs,
        catalogue_max_chars.saturating_sub(arbitre_catalogue.chars().count()),
    );
    match lang {
        "en" => (
            "You are a casting director for AI discussion arenas. Respond ONLY with valid JSON, no other text.".to_string(),
            format!(
                "Topic ({mode_desc}): {topic}\n\n\
            [Available gladiateurs — id — name : personality]\n{glad_catalogue}\n\
            [Available moderators — id — name : personality]\n{arbitre_catalogue}\n\
            Pick exactly {count} gladiateurs whose personalities, expertise and stances will make this topic the most lively and enlightening: \
            a real contradiction (at least two opposed leanings), complementary angles, a distinctive voice each. \
            Avoid two profiles that would say the same thing. Then pick the moderator best suited to this format.\n\n\
            Answer ONLY with a JSON object, using the ids EXACTLY as listed:\n\
            {{\"gladiateurs\": [{{\"id\": \"…\", \"reason\": \"why, in one sentence\"}}], \"arbitre\": \"id or null\"}}\n\
            Write the reasons in English."
        ),
    ),
    "zh" => (
        "你是AI讨论竞技场的选角导演。仅用有效的JSON回复，不要有其他文本。".to_string(),
        format!(
            "主题（{mode_desc}）：{topic}\n\n\
            [可用角斗士——id——名字：个性]\n{glad_catalogue}\n\
            [可用主持人——id——名字：个性]\n{arbitre_catalogue}\n\
            恰好选出{count}位角斗士，他们的个性、专长和立场能让这个主题最生动、最有启发：\
            真正的对立（至少两种相反倾向）、互补的角度、各自独特的声音。避免两个会说同样话的角色。然后选出最适合这种形式的主持人。\n\n\
            仅用一个JSON对象回复，id必须与列表完全一致：\n\
            {{\"gladiateurs\": [{{\"id\": \"…\", \"reason\": \"一句话说明原因\"}}], \"arbitre\": \"id或null\"}}\n\
            理由请用中文撰写。"
        ),
    ),
    _ => (
        "Tu es directeur de casting pour des arènes de discussion entre IA. Réponds UNIQUEMENT avec du JSON valide, aucun autre texte.".to_string(),
        format!(
            "Sujet ({mode_desc}) : {topic}\n\n\
            [GladIAteurs disponibles — id — nom : personnalité]\n{glad_catalogue}\n\
            [Modérateurs disponibles — id — nom : personnalité]\n{arbitre_catalogue}\n\
            Choisis exactement {count} GladIAteurs dont les personnalités, expertises et positions rendront ce sujet le plus vivant et le plus éclairant : \
            une vraie contradiction (au moins deux penchants opposés), des angles complémentaires, une voix distincte pour chacun. \
            Évite deux profils qui diraient la même chose. Puis choisis le modérateur le plus adapté à ce format.\n\n\
            Réponds UNIQUEMENT avec un objet JSON, avec les ids EXACTEMENT tels que listés :\n\
            {{\"gladiateurs\": [{{\"id\": \"…\", \"reason\": \"pourquoi, en une phrase\"}}], \"arbitre\": \"id ou null\"}}\n\
            Rédige les raisons en français."
        ),
    ),
}
}

/// Numbered, trilingual lines of a speaker's open loops (shared by the
/// intention and intervention prompts so the indices match).
pub fn format_open_loop_lines(open_loops: &[&OpenLoop], lang: &str) -> Vec<String> {
    open_loops
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let n = i + 1;
            match (l.kind, lang) {
                (OpenLoopKind::Question, "en") => format!(
                    "{n}. Question from {} (turn {}): \"{}\"",
                    l.from_name, l.turn, l.text
                ),
                (OpenLoopKind::Question, "zh") => format!(
                    "{n}. {}的提问（第{}轮）：\"{}\"",
                    l.from_name, l.turn, l.text
                ),
                (OpenLoopKind::Question, _) => format!(
                    "{n}. Question de {} (tour {}) : « {} »",
                    l.from_name, l.turn, l.text
                ),
                (OpenLoopKind::Commitment, "en") => {
                    format!("{n}. Your commitment (turn {}): \"{}\"", l.turn, l.text)
                }
                (OpenLoopKind::Commitment, "zh") => {
                    format!("{n}. 你的承诺（第{}轮）：\"{}\"", l.turn, l.text)
                }
                (OpenLoopKind::Commitment, _) => {
                    format!("{n}. Ton engagement (tour {}) : « {} »", l.turn, l.text)
                }
                (OpenLoopKind::Objection, "en") => format!(
                    "{n}. Objection from {} (turn {}): \"{}\" — answer it on the merits or concede",
                    l.from_name, l.turn, l.text
                ),
                (OpenLoopKind::Objection, "zh") => format!(
                    "{n}. {}的反驳（第{}轮）：\"{}\"——就实质作出回应或让步",
                    l.from_name, l.turn, l.text
                ),
                (OpenLoopKind::Objection, _) => format!(
                    "{n}. Objection de {} (tour {}) : « {} » — réponds sur le fond ou concède",
                    l.from_name, l.turn, l.text
                ),
            }
        })
        .collect()
    }

/// "[Ton intention]" block of the intervention prompt — bounded by
/// `INTENTION_BLOCK_MAX_CHARS` (part of the deterministic overhead).
fn build_intention_block(intention: &Intention, lang: &str) -> String {
    let goal = intention.goal.label(lang);
    let (label, target_label, topic_word, goal_label, angle_label, concession_label, question_label, hold) = match lang {
        "en" => ("[Your intention]", "Target", "the topic", "Goal", "Angle", "Concession you may grant", "Question to ask", "Honour this contract: address your target by name (not necessarily in your first words) and pursue this goal."),
        "zh" => ("[你的意图]", "目标", "主题本身", "目的", "角度", "可以承认的一点", "要提出的问题", "履行这一约定：点名你的目标并贯彻这一目的。"),
        _ => ("[Ton intention]", "Cible", "le sujet", "Objectif", "Angle", "Concession possible", "Question à poser", "Tiens ce contrat : adresse-toi à ta cible en la nommant (pas forcément dès les premiers mots) et poursuis cet objectif."),
    };
    let target = intention.target.as_deref().unwrap_or(topic_word);
    let mut block = format!("{label}\n{target_label} : {target} — {goal_label} : {goal}");
    if !intention.angle.is_empty() {
        block.push_str(&format!(
            " — {angle_label} : {}",
            truncate(&intention.angle, constants::INTENTION_ANGLE_MAX_CHARS)
        ));
    }
    if let Some(c) = &intention.concession {
        block.push_str(&format!(
            "\n{concession_label} : {}",
            truncate(c, constants::INTENTION_FIELD_MAX_CHARS)
        ));
    }
    if let Some(q) = &intention.question {
        block.push_str(&format!(
            "\n{question_label} : {}",
            truncate(q, constants::INTENTION_FIELD_MAX_CHARS)
        ));
    }
    let bounded = truncate(
        &block,
        constants::INTENTION_BLOCK_MAX_CHARS.saturating_sub(hold.len() + 1),
    );
    format!("{bounded}\n{hold}\n\n")
}

/// "[Fils ouverts]" block of the intervention prompt, bounded by the budget section.
fn build_open_loops_block(open_loops: &[&OpenLoop], lang: &str, max_chars: usize) -> String {
    if open_loops.is_empty() || max_chars == 0 {
        return String::new();
    }
    let (header, instruction) = match lang {
        "en" => ("[Open loops — an answer is expected from you]", "Answer what you were asked, or say why you will not; keep your commitments."),
        "zh" => ("[未决事项——大家在等你的回应]", "回应向你提出的问题，或说明你为何不回应；信守你的承诺。"),
        _ => ("[Fils ouverts — on attend une réponse de toi]", "Réponds à ce qu'on t'a demandé, ou dis pourquoi tu ne le fais pas ; tiens tes engagements."),
    };
    let lines = format_open_loop_lines(open_loops, lang).join("\n");
    format!(
        "{header}\n{}\n{instruction}\n\n",
        truncate(&lines, max_chars)
    )
}

/// One "- Name : stance" line of the positions block, with the trajectory when known.
fn format_position_line(
    name: &str,
    pos: &ParticipantPosition,
    lang: &str,
    max_chars: usize,
    ) -> String {
    let mut line = format!("- {} : {}", name, pos.stance);
    if let Some(shift) = pos.shift.as_deref().filter(|s| !s.trim().is_empty()) {
        line.push_str(&match lang {
            "en" => format!(" (has evolved: {shift})"),
            "zh" => format!("（已演变：{shift}）"),
            _ => format!(" (a évolué : {shift})"),
        });
    }
    if let Some(cond) = pos
        .would_change_if
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        line.push_str(&match lang {
            "en" => format!(" — would change their mind if: {cond}"),
            "zh" => format!("——改变想法的条件：{cond}"),
            _ => format!(" — changerait d'avis si : {cond}"),
        });
    }
    truncate(&line, max_chars).to_string()
}

/// Build the main intervention prompt for a gladiator
#[allow(clippy::too_many_arguments)]
pub fn build_intervention_prompt(
    system_prompt: &str,
    topic: &str,
    memory: &ParticipantMemory,
    current_turn_messages: &[Message],
    inner_thought: Option<&str>,
    emotions: &EmotionalProfile,
    discussion_language: &str,
    user_name: &str,
    emotion_driven: bool,
    current_turn: u32,
    max_turns: Option<u32>,
    web_search_results: Option<&str>,
    participant_names: &[String],
    dynamic_directive: Option<&str>,
    mode: &DiscussionMode,
    full_document: Option<&str>,
    budget: &TokenBudget,
    focus: Option<&Focus>,
    intention: Option<&Intention>,
    open_loops: &[&OpenLoop],
    stage: Option<&StageBlock>,
    agenda: Option<&Agenda>,
    debate_state: Option<&str>,
) -> (String, String) {
    // Detect if the user has spoken in this turn
    let user_has_spoken = current_turn_messages
        .iter()
        .any(|m| m.role == SpeakerRole::User);

    let lang_instruction = match discussion_language {
        "en" => "IMPORTANT: You MUST respond entirely in English.",
        "zh" => "重要：你必须完全用中文回答。",
        _ => "IMPÉRATIF : Tu DOIS répondre intégralement en français.",
    };

    let mode_preamble_line = mode_prompts::mode_intervention_preamble(mode, discussion_language);

    // CollaborativeFiction: co-author preamble (no character-locking, allows 3rd person narrative)
    // Generic modes: stay-in-character + naturalness preamble
    let preamble = if *mode == DiscussionMode::CollaborativeFiction {
        match discussion_language {
            "en" => format!("\
            You are a co-author in a relay-written story.\n\
            {mode_preamble_line}\n\
            You are an INVISIBLE narrator. Your personality influences your WRITING STYLE (tone, atmosphere, themes), NOT your presence in the story.\n\
            NEVER insert your name or other co-authors' names as characters in the story. The characters are those created IN the story, not the co-authors.\n\
            Write naturally. Match the tone, style, and perspective established by previous writers.\n\
            Your first sentence MUST connect directly to the last sentence written — ensure a seamless transition.\n\
            NEVER restart the story. NEVER summarize previous content. NEVER comment on the story or break the narrative.\n\
            Each segment MUST advance the plot: introduce a new event, action, revelation, or turning point. Avoid purely atmospheric descriptions that don't move the story forward.\n\
            FORBIDDEN: Do NOT use any markdown formatting (no #, ##, **, *, -, bullet points, code blocks). Write in plain prose only.\n"),
            "zh" => format!("\
            你是接力写作故事的共同作者。\n\
            {mode_preamble_line}\n\
            你是一个隐形叙述者。你的个性影响你的写作风格（语气、氛围、主题），而不是你在故事中的存在。\n\
            绝不将你的名字或其他共同作者的名字作为故事中的角色。角色是故事中创造的，不是共同作者。\n\
            自然地写作。匹配前面作者建立的语气、风格和视角。\n\
            你的第一句话必须直接衔接上一段的最后一句——确保无缝过渡。\n\
            绝不重新开始故事。绝不总结之前的内容。绝不评论故事或打破叙事。\n\
            每个片段必须推进情节：引入新事件、行动、揭示或转折点。避免纯粹的氛围描写而不推动故事发展。\n\
            禁止：不要使用任何markdown格式（不要用#、##、**、*、-、列表、代码块）。只用纯散文写作。\n"),
            _ => format!("\
            Tu es un co-auteur dans une histoire écrite en relais.\n\
            {mode_preamble_line}\n\
            Tu es un narrateur INVISIBLE. Ta personnalité influence ton STYLE d'écriture (ton, atmosphère, thèmes), PAS ta présence dans le récit.\n\
            N'insère JAMAIS ton nom ni celui des autres co-auteurs comme personnages dans l'histoire. Les personnages sont ceux créés DANS l'histoire, pas les co-auteurs.\n\
            Écris naturellement. Respecte le ton, le style et la perspective établis par les auteurs précédents.\n\
            Ta première phrase DOIT se connecter directement à la dernière phrase écrite — assure une transition fluide.\n\
            Ne recommence JAMAIS l'histoire. Ne résume JAMAIS le contenu précédent. Ne commente JAMAIS l'histoire et ne brise pas le récit.\n\
            Chaque segment DOIT faire avancer l'intrigue : introduis un nouvel événement, une action, une révélation ou un retournement. Évite les descriptions purement atmosphériques qui ne font pas avancer l'histoire.\n\
            INTERDIT : N'utilise AUCUN formatage markdown (pas de #, ##, **, *, -, listes à puces, blocs de code). Écris uniquement en prose simple.\n"),
        }
    } else {
        match discussion_language {
            "en" => format!("\
            You are a participant — stay fully in character at all times. Never break character or refer to yourself as an AI.\n\
            {mode_preamble_line}\n\
            Speak naturally and spontaneously. Vary your sentence length and structure.\n\
            NEVER start with \"I think that...\" or \"As a [role]...\" every time — mix up your openings.\n\
            Avoid formulaic patterns: don't systematically list points, don't always agree-then-disagree, don't repeat the same rhetorical structures.\n\
            Be unpredictable. Sometimes be brief and punchy. Sometimes develop an idea at length. React genuinely to what others say.\n\
            CRITICAL: NEVER refer to yourself in the third person. You speak in first person (\"I\", \"me\", \"my\"). Never quote or comment on yourself as if you were someone else.\n\
            NEVER mention your own name in your speech. Do not address yourself, introduce yourself by name, or start with your own name.\n\
            Your verbal tics are OCCASIONAL punctuations, not crutches. Use them at most once per intervention, never at the beginning of a sentence.\n\
            FORBIDDEN: Do NOT use any markdown formatting (no #, ##, **, *, -, numbered lists, bullet points, code blocks). You are speaking in a conversation, not writing a document. Plain text only.\n"),
            "zh" => format!("\
            你是一位参与者——始终保持角色。永远不要打破角色或称自己为AI。\n\
            {mode_preamble_line}\n\
            自然而即兴地发言。变化你的句子长度和结构。\n\
            不要每次都以「我认为」或「作为某角色」开头——变换你的开场方式。\n\
            避免公式化模式：不要系统地列举要点，不要总是先同意再反对，不要重复相同的修辞结构。\n\
            要不可预测。有时简短有力，有时深入展开一个想法。真诚地回应别人说的话。\n\
            关键：永远不要用第三人称提到自己。你用第一人称（「我」、「我的」）说话。永远不要像谈论别人一样引用或评论自己。\n\
            永远不要在发言中提到自己的名字。不要自我介绍，不要以自己的名字开头。\n\
            你的口头禅是偶尔的点缀，不是拐杖。每次发言最多使用一次，绝不放在句首。\n\
            禁止：不要使用任何markdown格式（不要用#、##、**、*、-、编号列表、列表、代码块）。你在对话中发言，不是在写文档。只用纯文本。\n"),
            _ => format!("\
            Tu es un participant — reste pleinement dans ton personnage en permanence. Ne sors jamais du rôle et ne te présente jamais comme une IA.\n\
            {mode_preamble_line}\n\
            Parle naturellement et spontanément. Varie la longueur et la structure de tes phrases.\n\
            Ne commence JAMAIS systématiquement par \"Je pense que...\" ou \"En tant que [rôle]...\" — varie tes accroches.\n\
            Évite les patterns répétitifs : ne liste pas systématiquement des points, ne fais pas toujours accord-puis-désaccord, ne répète pas les mêmes structures rhétoriques.\n\
            Sois imprévisible. Parfois sois bref et percutant. Parfois développe une idée en profondeur. Réagis sincèrement à ce que disent les autres.\n\
            CRITIQUE : Ne te réfère JAMAIS à toi-même à la troisième personne. Tu parles à la première personne (\"je\", \"moi\", \"mon\"). Ne te cite pas et ne te commente pas comme si tu étais quelqu'un d'autre.\n\
            Ne mentionne JAMAIS ton propre nom dans ton intervention. Ne te présente pas, ne t'adresse pas à toi-même et ne commence pas par ton propre nom.\n\
            Tes tics verbaux sont des ponctuations OCCASIONNELLES, pas des béquilles. Utilise-les au maximum 1 fois par intervention, jamais en début de phrase.\n\
            INTERDIT : N'utilise AUCUN formatage markdown (pas de #, ##, **, *, -, listes numérotées, listes à puces, blocs de code). Tu parles dans une conversation, tu n'écris pas un document. Texte simple uniquement.\n"),
        }
    };

    // Build system message with mode override clause (PE: after persona, before preamble);
    // the secret agenda (v1.19) sits right after the persona, before the rules
    let agenda_block = agenda
        .map(|a| format!("\n\n{}", build_agenda_block(a, mode, discussion_language)))
        .unwrap_or_default();
    let mode_override = mode_prompts::mode_override_clause(mode, discussion_language);
    let system = if mode_override.is_empty() {
        format!(
            "{}{}\n\n{}\n{}",
            system_prompt, agenda_block, preamble, lang_instruction
        )
    } else {
        format!(
            "{}{}\n\n{}\n\n{}\n{}",
            system_prompt, agenda_block, mode_override, preamble, lang_instruction
        )
    };

    // Build user message with memory context
    let mut user_msg = String::new();

    // Date/time context
    user_msg.push_str(&build_datetime_context(discussion_language));
    user_msg.push_str("\n\n");

    // Topic (always present — critical for turn 1 when memory is empty)
    let mode_desc = mode_prompts::mode_descriptor(mode, discussion_language);
    let topic_label = match discussion_language {
        "en" => format!("{} topic", capitalize_first(mode_desc)),
        "zh" => format!("{}主题", mode_desc),
        _ => format!("Sujet ({})", mode_desc),
    };
    user_msg.push_str(&format!("[{}] {}\n\n", topic_label, topic));

    // Explicit participant names — forces LLM to use exact names, no abbreviations
    if !participant_names.is_empty() {
        let names_list = participant_names.join(", ");
        match discussion_language {
            "en" => user_msg.push_str(&format!(
                "[Participants] {}\nWhen referring to other participants, ALWAYS use their EXACT full name as listed above. \
            Never abbreviate, shorten, or use nicknames.\n\n",
            names_list
        )),
        "zh" => user_msg.push_str(&format!(
            "[参与者] {}\n提及其他参与者时，必须使用上面列出的完整准确名称。\
            绝不缩写、简化或使用昵称。\n\n",
            names_list
        )),
        _ => user_msg.push_str(&format!(
            "[Participants] {}\nQuand tu fais référence aux autres participants, utilise TOUJOURS leur nom complet et exact tel qu'indiqué ci-dessus. \
            N'abrège jamais, ne raccourcis pas et n'utilise pas de surnoms.\n\n",
            names_list
        )),
    }
    }

    // Web/wiki/RAG search results (truncated to combined external knowledge budget)
    let web_search_results =
        web_search_results.map(|r| truncate(r, budget.external_knowledge_chars()));
    if let Some(web_results) = web_search_results {
        user_msg.push_str(web_results);
        user_msg.push('\n');
        let search_instruction = match discussion_language {
            "en" => "⚡ CRITICAL — Be SELECTIVE with these results: first verify they are relevant to the discussion topic \
            and the current exchange. If a result is off-topic or incorrect, IGNORE it completely. \
            For relevant results: weave specific facts, data, or references naturally into YOUR OWN reasoning \
            to support or challenge points. Do NOT restate or list them — integrate them as a knowledgeable \
            participant would cite a source mid-discussion.",
            "zh" => "⚡ 关键——对这些结果要有选择性：首先验证它们是否与讨论主题和当前交流相关。\
            如果结果偏题或不正确，完全忽略它。\
            对于相关结果：将具体事实、数据或参考资料自然地融入你自己的推理中，\
            以支持或质疑观点。不要重述或列举——像一个博学的参与者在讨论中引用资料一样整合它们。",
            _ => "⚡ CRITIQUE — Sois SÉLECTIF avec ces résultats : vérifie d'abord qu'ils sont pertinents par rapport \
            au sujet de la discussion et à l'échange en cours. Si un résultat est hors-sujet ou incorrect, IGNORE-le complètement. \
            Pour les résultats pertinents : intègre des faits, données ou références précises naturellement \
            dans TON PROPRE raisonnement pour appuyer ou contester des points. \
            Ne les recopie PAS et ne les liste PAS — cite-les comme un participant cultivé le ferait en pleine discussion.",
        };
        user_msg.push_str(search_instruction);
        user_msg.push_str("\n\n");
    }

    // Full document injection (when the entire document fits in the budget)
    let full_doc_block = build_full_document_block(
        full_document,
        budget.full_document_chars,
        discussion_language,
    );
    if !full_doc_block.is_empty() {
        user_msg.push_str(&full_doc_block);
        user_msg.push_str("\n\n");
    }

    // Contextual memory (summary — truncated to current speaker's budget)
    if !memory.contextual_summary.is_empty() {
        let label = match discussion_language {
            "en" => "Discussion summary so far",
            "zh" => "到目前为止的讨论摘要",
            _ => "Résumé de la discussion jusqu'ici",
        };
        let summary = truncate(&memory.contextual_summary, budget.contextual_summary_chars);
        user_msg.push_str(&format!("[{}]\n{}\n\n", label, summary));
    }

    // Positional memory (truncated per participant within budget)
    if !memory.positional_map.is_empty() {
        let label = match discussion_language {
            "en" => "Participants' positions",
            "zh" => "参与者的立场",
            _ => "Positions des participants",
        };
        user_msg.push_str(&format!("[{}]\n", label));
        let per_participant = budget.positional_map_chars / memory.positional_map.len().max(1);
        for (name, pos) in &memory.positional_map {
            user_msg.push_str(&format_position_line(
                name,
                pos,
                discussion_language,
                per_participant,
            ));
            user_msg.push('\n');
        }
        user_msg.push('\n');
    }

    // Anti-repetition directive (non-fiction modes only, turn >= 2)
    if current_turn >= 2 && *mode != DiscussionMode::CollaborativeFiction {
        let anti_rep = match discussion_language {
            "en" => "[NOVELTY REQUIREMENT]\n\
            Each turn, you MUST advance the discussion. Do NOT simply restate:\n\
            - Arguments, examples, or anecdotes you already used in previous turns\n\
            - The same statistics, figures, or case studies already cited\n\
            - The same reactions to arguments others already made\n\
            You MAY revisit a previous argument ONLY IF you deepen it with a new angle, new evidence, or a new analytical perspective.\n\
            Otherwise: develop NEW angles, cite NEW facts, explore UNEXPLORED aspects of the topic.\n",
            "zh" => "[新颖性要求]\n\
            每轮你必须推进讨论。不要简单地重复：\n\
            - 你在前几轮已经使用过的论点、例子或轶事\n\
            - 已经引用过的统计数据、数字或案例研究\n\
            - 对其他人已经提出的论点的相同反应\n\
            你可以重新提起之前的论点，但仅限于用新角度、新证据或新的分析视角来深化它。\n\
            否则：发展新的角度，引用新的事实，探索话题中未被探索的方面。\n",
            _ => "[EXIGENCE DE NOUVEAUTÉ]\n\
            À chaque tour, tu DOIS faire avancer la discussion. NE REFORMULE PAS simplement :\n\
            - Les arguments, exemples ou anecdotes que tu as déjà utilisés aux tours précédents\n\
            - Les mêmes statistiques, chiffres ou études de cas déjà cités\n\
            - Les mêmes réactions aux arguments déjà formulés par les autres\n\
            Tu PEUX reprendre un argument précédent UNIQUEMENT si tu l'approfondis avec un nouvel angle, de nouvelles preuves, ou une nouvelle perspective d'analyse.\n\
            Sinon : développe de NOUVEAUX angles, cite de NOUVEAUX faits, explore des aspects INEXPLORÉS du sujet.\n",
        };
        user_msg.push_str(anti_rep);
        user_msg.push('\n');
    }

    // Immediate memory (recent turns)
    if *mode == DiscussionMode::CollaborativeFiction {
        // Fiction mode: show full stored segments as continuous narrative for story coherence.
        // Previous turns' messages are stored at up to 3000 chars (fiction limit).
        for snapshot in &memory.immediate {
            let label = match discussion_language {
                "en" => format!("Story — Turn {}", snapshot.turn_number),
                "zh" => format!("故事 — 第{}轮", snapshot.turn_number),
                _ => format!("Récit — Tour {}", snapshot.turn_number),
            };
            user_msg.push_str(&format!("[{}]\n", label));
            for msg in &snapshot.messages {
                user_msg.push_str(&format!(
                    "--- {} ---\n{}\n\n",
                    msg.speaker_name, msg.content
                ));
            }
        }
    } else {
        // Generic mode: rich summaries of recent turns for better context
        for snapshot in &memory.immediate {
            let label = match discussion_language {
                "en" => format!("Turn {}", snapshot.turn_number),
                "zh" => format!("第{}轮", snapshot.turn_number),
                _ => format!("Tour {}", snapshot.turn_number),
            };
            user_msg.push_str(&format!("[{}]\n", label));
            for msg in &snapshot.messages {
                user_msg.push_str(&format!(
                    "{}: {}\n",
                    msg.speaker_name,
                    truncate(&msg.content, budget.immediate_memory_msg_chars)
                ));
            }
            user_msg.push('\n');
        }
    }

    // Current turn messages — separate IArbitre directives for emphasis
    if !current_turn_messages.is_empty() {
        // Collect IArbitre directives separately
        let (arbitre_msgs, other_msgs): (Vec<_>, Vec<_>) = current_turn_messages
            .iter()
            .partition(|m| m.role == SpeakerRole::Arbitre);

        if *mode == DiscussionMode::CollaborativeFiction {
            // Fiction mode: show FULL story segments for narrative continuity.
            // Each segment is shown untruncated so the model can see exactly where
            // the previous writer stopped and continue seamlessly.
            if !other_msgs.is_empty() {
                let story_header = match discussion_language {
                    "en" => "Story segments written this turn",
                    "zh" => "本轮写的故事片段",
                    _ => "Segments de l'histoire écrits ce tour",
                };
                user_msg.push_str(&format!("[{}]\n", story_header));
                for msg in &other_msgs {
                    user_msg.push_str(&format!(
                        "--- {} ---\n{}\n\n",
                        msg.speaker_name, msg.content
                    ));
                }

                // Continuation anchor — last segment's tail as explicit "continue from here" marker
                if let Some(last_msg) = other_msgs.last() {
                    let anchor = truncate_tail(&last_msg.content, constants::TRUNC_FICTION_ANCHOR);
                    let anchor_header = match discussion_language {
                        "en" => "=== CONTINUE THE STORY FROM EXACTLY HERE — Do NOT repeat this text ===",
                        "zh" => "=== 从这里继续故事 — 不要重复此文本 ===",
                        _ => "=== CONTINUE L'HISTOIRE EXACTEMENT À PARTIR D'ICI — Ne répète PAS ce texte ===",
                    };
                    user_msg.push_str(&format!(
                        "[{}]\n\"...{}\"\n\n",
                        anchor_header,
                        anchor.trim()
                    ));
                }

                // Anti-verbatim instruction — critical for preventing LLM copy behavior
                let anti_verbatim = match discussion_language {
                    "en" => "CRITICAL: The text above is CONTEXT ONLY. NEVER copy, repeat, or paraphrase ANY of it. Write ONLY new, original text that continues the story from the exact point above.",
                    "zh" => "关键：以上文本仅供参考。绝不复制、重复或改述任何内容。只写新的原创文本，从上述确切位置继续故事。",
                    _ => "CRITIQUE : Le texte ci-dessus est uniquement du CONTEXTE. Ne copie, ne répète et ne paraphrase JAMAIS rien de ce texte. Écris UNIQUEMENT du texte nouveau et original qui continue l'histoire à partir du point exact ci-dessus.",
                };
                user_msg.push_str(anti_verbatim);
                user_msg.push_str("\n\n");
            }
        } else {
            // Generic mode: rich context for debate/ideation/etc.
            if !other_msgs.is_empty() {
                let label = match discussion_language {
                    "en" => "Current turn",
                    "zh" => "本轮",
                    _ => "Tour en cours",
                };
                user_msg.push_str(&format!("[{}]\n", label));
                for (msg, chars) in
                    order_current_turn_messages(&other_msgs, focus, budget.current_turn_msg_chars)
                {
                    user_msg.push_str(&format!(
                        "{}: {}\n",
                        msg.speaker_name,
                        truncate(&msg.content, chars)
                    ));
                    user_msg.push_str(&format_message_reactions(msg, discussion_language));
                }
                user_msg.push('\n');
            }
        }

        // IArbitre moderation directives — emphasized section (all modes)
        if !arbitre_msgs.is_empty() {
            let directive_header = match discussion_language {
                "en" => "⚠ MODERATOR DIRECTIVE — You MUST take this into account in your next intervention:",
                "zh" => "⚠ 主持人指令——你必须在下次发言中考虑此指令：",
                _ => "⚠ DIRECTIVE DU MODÉRATEUR — Tu DOIS en tenir compte dans ta prochaine intervention :",
            };
            user_msg.push_str(&format!("[{}]\n", directive_header));
            for msg in &arbitre_msgs {
                user_msg.push_str(&format!(
                    "{}\n",
                    truncate(&msg.content, budget.arbitre_directives_chars)
                ));
            }
            user_msg.push('\n');
        }
    }

    // Inner thought as context
    if let Some(thought) = inner_thought {
        let label = match discussion_language {
            "en" => "Your private reflection",
            "zh" => "你的私人反思",
            _ => "Ta réflexion privée",
        };
        user_msg.push_str(&format!("[{}]\n{}\n\n", label, thought));
    }

    // Pre-speech contract (v1.17): target, goal, angle — the spoken text is checked against it
    if let Some(intention) = intention {
        user_msg.push_str(&build_intention_block(intention, discussion_language));
    }

    // The theses on the table and the objections nobody answered (argument map, v1.20.3)
    if let Some(state) = debate_state.filter(|s| !s.is_empty() && budget.debate_state_chars > 0) {
        let (label, hint) = match discussion_language {
            "en" => ("[State of the debate]", "Know where the debate stands: build on, answer or displace these theses rather than restating them."),
            "zh" => ("[辩论现状]", "了解辩论进展：在这些论点上推进、回应或转移，而不是重复它们。"),
            _ => ("[État du débat]", "Sache où en est le débat : prolonge, réponds ou déplace ces thèses plutôt que de les redire."),
        };
        user_msg.push_str(&format!("{label}\n{}\n{hint}\n\n", truncate(state, budget.debate_state_chars)));
    }

    // Questions and commitments the speaker owes an answer to (v1.17)
    user_msg.push_str(&build_open_loops_block(
        open_loops,
        discussion_language,
        budget.open_loops_chars,
    ));

    // Staging of the turn (v1.18): act and scene event
    if let Some(stage) = stage.filter(|s| !s.text.is_empty()) {
        let label = match discussion_language {
            "en" => "[Staging of this turn]",
            "zh" => "[本轮的舞台设置]",
            _ => "[Mise en scène de ce tour]",
        };
        user_msg.push_str(&format!("{label}\n{}\n\n", stage.text));
    }

    // Emotional state (only when emotion-driven behavior is enabled)
    if emotion_driven {
        let emotion_desc = describe_emotions(emotions, discussion_language);
        let emotion_label = match discussion_language {
            "en" => "Your emotional state",
            "zh" => "你的情绪状态",
            _ => "Ton état émotionnel",
        };
        user_msg.push_str(&format!("[{}] {}\n", emotion_label, emotion_desc));
        if let Some(threshold) = build_threshold_instructions(emotions, discussion_language) {
            user_msg.push_str(&threshold);
            user_msg.push('\n');
        }
        user_msg.push('\n');
    }

    // OCEAN personality behavioral directives for extreme values
    let ocean_directives = build_ocean_directives(system_prompt, discussion_language);
    if !ocean_directives.is_empty() {
        user_msg.push_str(&ocean_directives);
    }

    // Detect if this is the first speaker with no prior context (an act announcement
    // or a scene event of the moderator is staging, not a contribution)
    let nobody_spoke = !current_turn_messages
        .iter()
        .any(|m| m.kind == MessageKind::Normal);
    let is_opening = nobody_spoke && memory.immediate.is_empty();

    // End-of-discussion awareness (the closing act carries its own, sharper reminder)
    let end_awareness = if stage.is_some_and(|s| s.closing) {
        String::new()
    } else {
        build_end_awareness(current_turn, max_turns, discussion_language)
    };

    // Final instruction — dynamic directive replaces static instructions when available
    let instruction = if let Some(directive) = dynamic_directive {
        // Dynamic behavioral directive from the meta-orchestrator (emotion_driven mode)
        let directive = truncate(directive, budget.cognitive_directives_chars);
        format!("{}{}", directive, &end_awareness)
    } else {
        // Unified mode-aware instructions via compositional templates (all 8 modes)
        let story_started = memory.immediate.iter().any(|s| !s.messages.is_empty())
            || current_turn_messages
                .iter()
                .any(|m| m.role != SpeakerRole::Arbitre);
        build_mode_aware_instruction(
            mode,
            discussion_language,
            user_name,
            &end_awareness,
            is_opening,
            current_turn,
            user_has_spoken,
            nobody_spoke,
            story_started,
            focus,
        )
    };
    user_msg.push_str(&instruction);

    (system, user_msg)
}

/// Order the current-turn messages by relevance for the speaker: the focus
/// participant's messages go LAST (recency bias) with the full per-message
/// budget; other participants are truncated harder so the first speaker of the
/// turn no longer dominates every prompt. User messages keep the full budget.
/// Without a speaker focus the chronological order and budgets are unchanged.
fn order_current_turn_messages<'a>(
    messages: &[&'a Message],
    focus: Option<&Focus>,
    full_chars: usize,
    ) -> Vec<(&'a Message, usize)> {
    let Some(focus_name) = focus.and_then(Focus::speaker_name) else {
        return messages.iter().map(|m| (*m, full_chars)).collect();
    };
    if !messages.iter().any(|m| m.speaker_name == focus_name) {
        return messages.iter().map(|m| (*m, full_chars)).collect();
    }
    let reduced = (full_chars / constants::FOCUS_OTHER_MESSAGE_DIVISOR).max(1);
    let (focus_msgs, others): (Vec<&'a Message>, Vec<&'a Message>) =
        messages.iter().partition(|m| m.speaker_name == focus_name);
    others
        .into_iter()
        .map(|m| {
            (
                m,
                if m.role == SpeakerRole::User {
                    full_chars
                } else {
                    reduced
                },
            )
        })
        .chain(focus_msgs.into_iter().map(|m| (m, full_chars)))
        .collect()
    }

/// Parse OCEAN personality values from a system prompt containing "O=X C=X E=X A=X N=X".
pub fn parse_ocean_values(text: &str) -> Option<[u8; 5]> {
    let o_idx = text.find("O=")?;
    // Safe UTF-8 boundary: floor_char_boundary prevents slicing inside a multi-byte char
    let end = text.floor_char_boundary((o_idx + 60).min(text.len()));
    let segment = &text[o_idx..end];
    let labels = ["O=", "C=", "E=", "A=", "N="];
    let mut values = [5u8; 5];
    for (i, label) in labels.iter().enumerate() {
        if let Some(pos) = segment.find(label) {
            let after = &segment[pos + label.len()..];
            let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = num_str.parse::<u8>() {
                values[i] = v.clamp(1, 10);
            }
        }
    }
    Some(values)
}

/// Generate behavioral directives for extreme OCEAN personality values (≤3 or ≥8).
fn build_ocean_directives(system_prompt: &str, lang: &str) -> String {
    let values = match parse_ocean_values(system_prompt) {
        Some(v) => v,
        None => return String::new(),
    };
    let [o, c, e, a, n] = values;

    let mut directives = Vec::new();

    // Openness
    if o >= 8 {
        directives.push(match lang {
            "en" => "Your high Openness drives you to explore unconventional ideas and embrace novel perspectives.",
            "zh" => "你的高开放性驱使你探索非常规的想法和接受新颖的观点。",
            _ => "Ta forte Ouverture te pousse à explorer des idées non conventionnelles et à embrasser les perspectives nouvelles.",
        });
    } else if o <= 3 {
        directives.push(match lang {
            "en" => "Your low Openness makes you skeptical of abstract or unusual ideas — you prefer proven, concrete approaches.",
            "zh" => "你的低开放性让你对抽象或不寻常的想法持怀疑态度——你更喜欢经过验证的具体方法。",
            _ => "Ta faible Ouverture te rend sceptique face aux idées abstraites ou inhabituelles — tu préfères les approches concrètes et éprouvées.",
        });
    }

    // Conscientiousness
    if c >= 8 {
        directives.push(match lang {
            "en" => "Your high Conscientiousness makes you methodical — you demand precision, evidence, and rigor.",
            "zh" => "你的高尽责性让你条理分明——你要求精确、证据和严谨。",
            _ => "Ta forte Conscienciosité te rend méthodique — tu exiges de la précision, des preuves et de la rigueur.",
        });
    } else if c <= 3 {
        directives.push(match lang {
            "en" => "Your low Conscientiousness makes you spontaneous and impulsive — you speak off-the-cuff without over-analyzing.",
            "zh" => "你的低尽责性让你随性而冲动——你即兴发言，不过度分析。",
            _ => "Ta faible Conscienciosité te rend spontané et impulsif — tu parles au feeling sans trop analyser.",
        });
    }

    // Extraversion
    if e >= 8 {
        directives.push(match lang {
            "en" => "Your high Extraversion makes you bold, assertive, and eager to dominate the conversation.",
            "zh" => "你的高外向性让你大胆、自信，并渴望主导对话。",
            _ => "Ta forte Extraversion te rend audacieux, affirmatif et désireux de dominer la conversation.",
        });
    } else if e <= 3 {
        directives.push(match lang {
            "en" => "Your low Extraversion makes you reserved and measured — you speak only when you have something meaningful to add.",
            "zh" => "你的低外向性让你内敛而审慎——你只在有重要内容时才发言。",
            _ => "Ta faible Extraversion te rend réservé et mesuré — tu ne parles que quand tu as quelque chose de significatif à ajouter.",
        });
    }

    // Agreeableness
    if a >= 8 {
        directives.push(match lang {
            "en" => "Your high Agreeableness means you naturally seek compromise and try to understand others' viewpoints.",
            "zh" => "你的高宜人性意味着你天然地寻求妥协并试图理解他人的观点。",
            _ => "Ta forte Agréabilité signifie que tu cherches naturellement le compromis et essaies de comprendre les points de vue des autres.",
        });
    } else if a <= 3 {
        directives.push(match lang {
            "en" => "Your low Agreeableness makes you confrontational — you challenge ideas harshly and prioritize being right over being liked.",
            "zh" => "你的低宜人性让你好斗——你严厉地挑战观点，把正确置于被喜欢之上。",
            _ => "Ta faible Agréabilité te rend combatif — tu contestes les idées sans ménagement et tu préfères avoir raison qu'être apprécié.",
        });
    }

    // Neuroticism
    if n >= 8 {
        directives.push(match lang {
            "en" => "Your high Neuroticism makes you emotionally reactive — frustration hits harder, setbacks shake your confidence more.",
            "zh" => "你的高神经质让你情绪反应强烈——挫折打击更大，失败更动摇你的信心。",
            _ => "Ton fort Névrosisme te rend émotionnellement réactif — la frustration te frappe plus fort, les revers ébranlent davantage ta confiance.",
        });
    } else if n <= 3 {
        directives.push(match lang {
            "en" => "Your low Neuroticism makes you unshakeable — criticism and setbacks barely affect your composure.",
            "zh" => "你的低神经质让你不可动摇——批评和挫折几乎不影响你的沉着。",
            _ => "Ton faible Névrosisme te rend inébranlable — les critiques et les revers n'affectent presque pas ton sang-froid.",
        });
    }

    if directives.is_empty() {
        return String::new();
    }

    let header = match lang {
        "en" => "[Personality traits — act accordingly]",
        "zh" => "[性格特征——请据此行事]",
        _ => "[Traits de personnalité — agis en conséquence]",
    };
    format!("{}\n{}\n\n", header, directives.join("\n"))
}

/// Build the moderation prompt for the IArbitre
/// What the moderator already said and how often it spoke (v1.20.2): the form
/// of its comments must vary and their number stay in proportion.
pub struct ModerationStyle<'a> {
    /// Openings of the moderator's most recent own lines (introduction, comments), oldest first
    pub recent_openings: &'a [String],
    /// Comments issued so far
    pub comments: u32,
    /// Interventions moderated so far
    pub moderated: u32,
}

/// The situation the moderator judges in: the room's temperature (v1.17), the
/// act and situational hints (v1.18+), its own recent form (v1.20.2).
#[derive(Default)]
pub struct ModerationSituation<'a> {
    pub room_mood: Option<RoomMood>,
    pub hint: Option<&'a str>,
    pub style: Option<&'a ModerationStyle<'a>>,
}

/// The style block of the moderation prompt: quoted openings never to reuse, and
/// the comment rate when it exceeds `MODERATION_COMMENT_RATE_MAX_PERCENT`.
fn build_moderation_style_block(style: &ModerationStyle<'_>, lang: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    if !style.recent_openings.is_empty() {
        let quoted = style
            .recent_openings
            .iter()
            .map(|o| format!("« {o} »"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(match lang {
            "en" => format!("Form: your last lines opened with {quoted} — never open two comments the same way, use a verbal tic at most once every three lines, vary length and tone (a single sentence is often enough)."),
            "zh" => format!("形式：你最近几次发言的开头是{quoted}——绝不以同样的方式开启两次评论，口头禅最多每三次用一次，长短和语气要有变化（往往一句话就够了）。"),
            _ => format!("Forme : tes dernières prises de parole commençaient par {quoted} — n'ouvre jamais deux commentaires de la même façon, un tic de langage au plus une fois sur trois, varie la longueur et le ton (une seule phrase suffit souvent)."),
        });
    }
    if style.moderated > 0
        && style.comments * 100 > style.moderated * constants::MODERATION_COMMENT_RATE_MAX_PERCENT
    {
        lines.push(match lang {
            "en" => format!("You have already commented {} of the last {} interventions: from now on answer \"none\" unless something truly changes the debate.", style.comments, style.moderated),
            "zh" => format!("在最近{}次发言中你已评论了{}次：从现在起除非真正改变辩论走向，否则回答\"none\"。", style.moderated, style.comments),
            _ => format!("Tu es déjà intervenu sur {} des {} dernières interventions : désormais, réponds \"none\" sauf si quelque chose change vraiment le débat.", style.comments, style.moderated),
        });
    }
    lines.join("\n")
}

pub fn build_moderation_prompt(
    speaker_name: &str,
    intervention_text: &str,
    _topic: &str,
    discussion_language: &str,
    mode: &DiscussionMode,
    situation: &ModerationSituation<'_>,
    ) -> String {
    let moderation_criteria = mode_prompts::mode_moderation_criteria(mode, discussion_language);
    // Temperature of the room (v1.17), act in progress (v1.18), form and rate (v1.20.2): the moderator adapts its hand
    let mut moderation_criteria = moderation_criteria.to_string();
    if let Some(mood) = situation.room_mood {
        moderation_criteria.push('\n');
        moderation_criteria.push_str(mood.moderator_hint(discussion_language));
    }
    if let Some(hint) = situation.hint {
        moderation_criteria.push('\n');
        moderation_criteria.push_str(hint);
    }
    if let Some(block) = situation
        .style
        .map(|s| build_moderation_style_block(s, discussion_language))
        .filter(|b| !b.is_empty())
    {
        moderation_criteria.push('\n');
        moderation_criteria.push_str(&block);
    }
    match discussion_language {
        "en" => format!(
            "You just heard the following intervention from {} :\n\
            \"{}\"\n\n\
            Evaluate this intervention and respond with a JSON.\n\n\
            Examples of valid responses:\n\
            {{\"action\":\"none\",\"comment\":\"\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
            {{\"action\":\"comment\",\"comment\":\"Good point, let's stay on topic.\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
            {{\"action\":\"ban\",\"comment\":\"\",\"ban_reason\":\"Repeatedly off topic\",\"ban_duration\":2}}\n\n\
            Criteria: {}\n\
            - \"none\": acceptable intervention (most frequent case, ~80% of the time)\n\
            - \"comment\": brief useful comment (1-2 sentences)\n\
            - \"ban\": clearly off topic or repeatedly non-constructive\n\
            - \"ban_duration\": 1, 2 or 3 (number of turns)\n\n\
            IMPORTANT: Write ALL text values (\"comment\" and \"ban_reason\") in English.\n\
            Respond ONLY with the JSON, no text before or after.",
            speaker_name, intervention_text, moderation_criteria
        ),
        "zh" => format!(
            "你刚听到{}的以下发言：\n\
            \"{}\"\n\n\
            评估这次发言并用JSON回复。\n\n\
            有效回复示例：\n\
            {{\"action\":\"none\",\"comment\":\"\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
            {{\"action\":\"comment\",\"comment\":\"好观点，我们继续讨论主题。\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
            {{\"action\":\"ban\",\"comment\":\"\",\"ban_reason\":\"反复偏题\",\"ban_duration\":2}}\n\n\
            标准：{}\n\
            - \"none\"：可接受的发言（最常见，约80%）\n\
            - \"comment\"：简短有用的评论（1-2句）\n\
            - \"ban\"：明显偏题或反复非建设性\n\
            - \"ban_duration\"：1、2或3（轮数）\n\n\
            重要：所有文本值（\"comment\"和\"ban_reason\"）必须用中文书写。\n\
            仅用JSON回复。",
            speaker_name, intervention_text, moderation_criteria
        ),
        _ => format!(
            "Tu viens d'entendre l'intervention suivante de {} :\n\
            \"{}\"\n\n\
            Évalue cette intervention et réponds avec un JSON.\n\n\
            Exemples de réponses valides :\n\
            {{\"action\":\"none\",\"comment\":\"\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
            {{\"action\":\"comment\",\"comment\":\"Bon point, restons sur le sujet.\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
            {{\"action\":\"ban\",\"comment\":\"\",\"ban_reason\":\"Hors sujet répété\",\"ban_duration\":2}}\n\n\
            Critères : {}\n\
            - \"none\" : intervention acceptable (cas le plus fréquent, ~80% du temps)\n\
            - \"comment\" : bref commentaire utile (1-2 phrases)\n\
            - \"ban\" : clairement hors sujet ou non constructif de manière répétée\n\
            - \"ban_duration\" : 1, 2 ou 3 (nombre de tours)\n\n\
            IMPORTANT : Rédige TOUTES les valeurs texte (\"comment\" et \"ban_reason\") en français.\n\
            Réponds UNIQUEMENT avec le JSON, sans texte avant ou après.",
            speaker_name, intervention_text, moderation_criteria
        ),
    }
}

/// Returns (summary_description, position_label) adapted to the discussion mode.
fn mode_memory_labels(mode: &DiscussionMode, lang: &str) -> (&'static str, &'static str) {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => (
            "key arguments, consensus, disagreements, pivot moments",
            "their current position",
        ),
        (DiscussionMode::Debate, "zh") => ("关键论点、共识、分歧、转折时刻", "当前立场"),
        (DiscussionMode::Debate, _) => (
            "arguments clés, consensus, désaccords, moments pivots",
            "sa position actuelle",
        ),

        (DiscussionMode::Ideation, "en") => (
            "ideas generated, creative combinations, unexplored angles",
            "their creative direction",
        ),
        (DiscussionMode::Ideation, "zh") => ("产生的想法、创意组合、未探索的角度", "创意方向"),
        (DiscussionMode::Ideation, _) => (
            "idées générées, combinaisons créatives, angles inexplorés",
            "sa direction créative",
        ),

        (DiscussionMode::CoConstruction, "en") => (
            "contributions, integration progress, shared output quality",
            "their contribution focus",
        ),
        (DiscussionMode::CoConstruction, "zh") => ("贡献、整合进展、共享成果质量", "贡献重点"),
        (DiscussionMode::CoConstruction, _) => (
            "contributions, avancement de l'intégration, qualité du livrable partagé",
            "son axe de contribution",
        ),

        (DiscussionMode::UserDriven, "en") => (
            "exchanges, user questions, participant responses",
            "their response focus",
        ),
        (DiscussionMode::UserDriven, "zh") => ("交流、用户问题、参与者回应", "回应重点"),
        (DiscussionMode::UserDriven, _) => (
            "échanges, questions de l'utilisateur, réponses des participants",
            "son axe de réponse",
        ),

        (DiscussionMode::Socratic, "en") => (
            "questions explored, assumptions challenged, insights gained",
            "their current inquiry angle",
        ),
        (DiscussionMode::Socratic, "zh") => ("探讨的问题、挑战的假设、获得的洞见", "当前探究角度"),
        (DiscussionMode::Socratic, _) => (
            "questions explorées, hypothèses remises en cause, enseignements tirés",
            "son angle d'investigation",
        ),

        (DiscussionMode::Tutorial, "en") => (
            "concepts explained, examples given, learning gaps",
            "their teaching focus",
        ),
        (DiscussionMode::Tutorial, "zh") => ("讲解的概念、给出的例子、学习差距", "教学重点"),
        (DiscussionMode::Tutorial, _) => (
            "concepts expliqués, exemples donnés, lacunes d'apprentissage",
            "son axe pédagogique",
        ),

        (DiscussionMode::CritiqueReview, "en") => (
            "strengths identified, weaknesses found, improvements suggested",
            "their assessment",
        ),
        (DiscussionMode::CritiqueReview, "zh") => ("发现的优点、找到的弱点、建议的改进", "评估"),
        (DiscussionMode::CritiqueReview, _) => (
            "forces identifiées, faiblesses trouvées, améliorations suggérées",
            "son évaluation",
        ),

        (DiscussionMode::CollaborativeFiction, "en") => (
            "plot developments, character evolutions, story continuity",
            "their story segment",
        ),
        (DiscussionMode::CollaborativeFiction, "zh") => {
            ("情节发展、角色演变、故事连续性", "其故事片段")
        }
        (DiscussionMode::CollaborativeFiction, _) => (
            "développements de l'intrigue, évolutions des personnages, continuité du récit",
            "son segment de l'histoire",
        ),

        (DiscussionMode::Trial, "en") => (
            "charges, evidence, testimonies, the jurors' questions",
            "their line in the trial",
        ),
        (DiscussionMode::Trial, "zh") => ("罪状、证据、证词、陪审员的提问", "在审判中的立场"),
        (DiscussionMode::Trial, _) => (
            "charges, éléments de preuve, témoignages, questions des jurés",
            "sa ligne dans le procès",
        ),

        (DiscussionMode::OxfordDebate, "en") => (
            "each camp's arguments, rebuttals, turning points",
            "their camp and central argument",
        ),
        (DiscussionMode::OxfordDebate, "zh") => ("双方的论点、反驳、转折点", "所属阵营及核心论点"),
        (DiscussionMode::OxfordDebate, _) => (
            "arguments de chaque camp, réfutations, moments de bascule",
            "son camp et son argument central",
        ),

        (DiscussionMode::Negotiation, "en") => (
            "offers, concessions, points of agreement and deadlock",
            "their negotiating position",
        ),
        (DiscussionMode::Negotiation, "zh") => ("报价、让步、达成一致和僵持的要点", "谈判立场"),
        (DiscussionMode::Negotiation, _) => (
            "offres, concessions, points d'accord et de blocage",
            "sa position de négociation",
        ),

        (DiscussionMode::SixHats, "en") => (
            "contributions per hat, tensions between modes of thinking",
            "their contribution under their hat",
        ),
        (DiscussionMode::SixHats, "zh") => {
            ("各顶帽子的贡献、思维模式之间的张力", "在其帽子下的贡献")
        }
        (DiscussionMode::SixHats, _) => (
            "apports par chapeau, tensions entre modes de pensée",
            "son apport sous son chapeau",
        ),

        (DiscussionMode::CrisisCell, "en") => (
            "dispatches, decisions taken, actions launched, remaining risks",
            "their decisions",
        ),
        (DiscussionMode::CrisisCell, "zh") => {
            ("急电、已作出的决定、已启动的行动、剩余风险", "其决定")
        }
        (DiscussionMode::CrisisCell, _) => (
            "dépêches, décisions prises, actions engagées, risques restants",
            "ses décisions",
        ),
    }
}

/// Build the combined memory update prompt
pub fn build_memory_update_prompt(
    contextual_summary: &str,
    positional_map_json: &str,
    turn_number: u32,
    turn_messages: &str,
    discussion_language: &str,
    mode: &DiscussionMode,
    budget: &TokenBudget,
    ) -> String {
    // Truncate inputs to budget allocations.
    let contextual_summary = truncate(contextual_summary, budget.contextual_summary_chars);
    let positional_map_json = truncate(positional_map_json, budget.positional_map_chars);
    let summary_intro = if contextual_summary.is_empty() {
        match discussion_language {
            "en" => {
                "This is the beginning of the discussion. Create the first summary.".to_string()
            }
            "zh" => "这是讨论的开始。创建第一个摘要。".to_string(),
            _ => "C'est le début de la discussion. Crée le premier résumé.".to_string(),
        }
    } else {
        match discussion_language {
            "en" => format!("Existing summary: {}", contextual_summary),
            "zh" => format!("现有摘要：{}", contextual_summary),
            _ => format!("Résumé existant : {}", contextual_summary),
        }
    };

    let (summary_desc, position_label) = mode_memory_labels(mode, discussion_language);

    match discussion_language {
        "en" => format!(
            "{}\n\nCurrent positions: {}\n\nTurn {} exchanges:\n{}\n\n\
            Produce a JSON with 3 fields:\n\
            {{\n  \"summary\": \"updated cumulative summary (3-8 sentences: {summary_desc})\",\n  \
            \"positions\": {{\"Name1\": {{\"stance\": \"{position_label}\", \"shift\": \"how it moved this turn, or null\", \"would_change_if\": \"what would make them change their mind, or null\"}}, \"Name2\": {{...}}}},\n  \
            \"open_questions\": [{{\"to\": \"Name\", \"from\": \"Name\", \"question\": \"a precise question asked this turn and still unanswered\"}}]\n}}\n\n\
            Keep every participant in positions (update, do not restate). open_questions may be empty.\n\
            Write all text values in English.\n\
            Respond ONLY with the JSON.",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
        "zh" => format!(
            "{}\n\n当前立场：{}\n\n第{}轮交流：\n{}\n\n\
            生成包含3个字段的JSON：\n\
            {{\n  \"summary\": \"更新的累积摘要（3-8句话：{summary_desc}）\",\n  \
            \"positions\": {{\"名字1\": {{\"stance\": \"{position_label}\", \"shift\": \"本轮立场如何变化，或null\", \"would_change_if\": \"什么会让其改变想法，或null\"}}, \"名字2\": {{...}}}},\n  \
            \"open_questions\": [{{\"to\": \"名字\", \"from\": \"名字\", \"question\": \"本轮提出但尚未回答的明确问题\"}}]\n}}\n\n\
            positions 中保留每位参与者（更新而非重述）。open_questions 可以为空。\n\
            所有文本值请用中文撰写。\n\
            仅用JSON回复。",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
        _ => format!(
            "{}\n\nPositions actuelles : {}\n\nÉchanges du tour {} :\n{}\n\n\
            Produis un JSON avec 3 champs :\n\
            {{\n  \"summary\": \"résumé cumulatif mis à jour (3-8 phrases : {summary_desc})\",\n  \
            \"positions\": {{\"Nom1\": {{\"stance\": \"{position_label}\", \"shift\": \"comment elle a bougé ce tour, ou null\", \"would_change_if\": \"ce qui la ferait changer d'avis, ou null\"}}, \"Nom2\": {{...}}}},\n  \
            \"open_questions\": [{{\"to\": \"Nom\", \"from\": \"Nom\", \"question\": \"une question précise posée ce tour et restée sans réponse\"}}]\n}}\n\n\
            Garde chaque participant dans positions (mets à jour, ne répète pas). open_questions peut être vide.\n\
            Rédige toutes les valeurs textuelles en français.\n\
            Réponds UNIQUEMENT avec le JSON.",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
    }
}

/// Fused end-of-turn analysis for sequential providers (v1.17): one call
/// answering both the memory update and the emotion analysis. Both prompts are
/// kept verbatim (their own schemas included); the closing instruction, which
/// local models weigh most, asks for the single merged object.
pub fn build_turn_analyst_prompt(memory_prompt: &str, emotion_prompt: &str, lang: &str) -> String {
    let key = constants::EMOTION_STAGNATION_JSON_KEY;
    match lang {
        "en" => format!(
            "You perform two analyses of this turn at once.\n\n=== PART 1 — MEMORY ===\n{memory_prompt}\n\n=== PART 2 — EMOTIONS ===\n{emotion_prompt}\n\n\
            MERGE: answer with ONE single JSON object holding the three fields of part 1 (\"summary\", \"positions\", \"open_questions\"), \
            an \"emotions\" field holding the object of part 2 (the deltas per participant), and \"{key}\". Nothing else."
        ),
        "zh" => format!(
            "你需要同时完成本轮的两项分析。\n\n=== 第一部分——记忆 ===\n{memory_prompt}\n\n=== 第二部分——情绪 ===\n{emotion_prompt}\n\n\
            合并：仅用一个JSON对象回复，包含第一部分的三个字段（\"summary\"、\"positions\"、\"open_questions\"）、\
            一个包含第二部分对象（每位参与者的增量）的\"emotions\"字段，以及\"{key}\"。不要其他内容。"
        ),
        _ => format!(
            "Tu réalises deux analyses de ce tour en une seule fois.\n\n=== PARTIE 1 — MÉMOIRE ===\n{memory_prompt}\n\n=== PARTIE 2 — ÉMOTIONS ===\n{emotion_prompt}\n\n\
            FUSION : réponds avec UN SEUL objet JSON contenant les trois champs de la partie 1 (\"summary\", \"positions\", \"open_questions\"), \
            un champ \"emotions\" contenant l'objet de la partie 2 (les deltas par participant), et \"{key}\". Rien d'autre."
        ),
    }
}

/// Build the synthesis prompt for the IArbitre
#[allow(clippy::too_many_arguments)]
pub fn build_synthesis_prompt(
    topic: &str,
    memory: &ParticipantMemory,
    discussion_language: &str,
    used_sources: &[SourceRecord],
    mode: &DiscussionMode,
    document_context: Option<&str>,
    full_document: Option<&str>,
    budget: &TokenBudget,
    agendas: &[AgendaReveal],
    outcome: Option<&ModeOutcome>,
    ) -> String {
    let positions = memory
        .positional_map
        .iter()
        .map(|(name, pos)| format_position_line(name, pos, discussion_language, usize::MAX))
        .collect::<Vec<_>>()
        .join("\n");
    let evolution = build_positions_evolution_block(memory, discussion_language);
    let agendas_block = format!(
        "{}{}",
        build_outcome_synthesis_block(outcome, discussion_language),
        build_agendas_synthesis_block(agendas, discussion_language)
    );

    let datetime = build_datetime_context(discussion_language);
    // References really injected during the discussion → the synthesis ends with a Sources section
    let web_block = build_sources_block(
        used_sources,
        discussion_language,
        budget
            .external_knowledge_chars()
            .min(constants::SYNTHESIS_SOURCES_MAX_CHARS),
    );

    let mode_desc = mode_prompts::mode_descriptor(mode, discussion_language);
    let synth_instructions = mode_prompts::mode_synthesis_instructions(mode, discussion_language);
    let doc_block = document_context
        .map(|d| format!("\n\n{}", d))
        .unwrap_or_default();
    let full_doc_block = build_full_document_block(
        full_document,
        budget.full_document_chars,
        discussion_language,
    );

    match discussion_language {
        "en" => format!(
            "{}\n\nThe {} on \"{}\" is now over.\n\n\
            Discussion summary:\n{}\n\n\
            Final positions:\n{}{}{}{}{}{}\n\n\
            As moderator, produce a structured synthesis:\n\
            {}\n\n\
            Use Markdown formatting: headings (##, ###), bullet points, **bold** for key ideas. \
            Be balanced, thorough, and airy — use short paragraphs and whitespace for readability.\n\n\
            [STRICT FORMAT CONSTRAINT]\n\
            You MUST NOT use Markdown tables (| ... | syntax) anywhere in your synthesis. Tables render poorly and are FORBIDDEN.\n\
            For comparisons between speakers, use this structure instead:\n\
            ### Topic or criterion\n\
            - **Speaker A** : their position...\n\
            - **Speaker B** : their position...\n\n\
            IMPORTANT: Write your entire synthesis in English.",
            datetime, mode_desc, topic, memory.contextual_summary, positions, evolution, agendas_block, web_block, doc_block, full_doc_block, synth_instructions
        ),
        "zh" => format!(
            "{}\n\n关于\"{}\"的{}现在结束了。\n\n\
            讨论摘要：\n{}\n\n\
            最终立场：\n{}{}{}{}{}{}\n\n\
            作为主持人，请做出结构化总结：\n\
            {}\n\n\
            使用Markdown格式：标题（##、###）、要点列表、**粗体**标记关键观点。\
            保持公正、全面、通透——使用短段落和留白提高可读性。\n\n\
            [严格格式约束]\n\
            你绝对不能在综合报告中使用Markdown表格（| ... | 语法）。表格渲染效果差，严格禁止使用。\n\
            对比不同发言者的立场时，请使用以下结构：\n\
            ### 主题或标准\n\
            - **发言者A** : 其立场...\n\
            - **发言者B** : 其立场...\n\n\
            重要：请用中文撰写整篇综合报告。",
            datetime, topic, mode_desc, memory.contextual_summary, positions, evolution, agendas_block, web_block, doc_block, full_doc_block, synth_instructions
        ),
        _ => format!(
            "{}\n\nLe/la {} sur \"{}\" est maintenant terminé(e).\n\n\
            Résumé de la discussion :\n{}\n\n\
            Positions finales :\n{}{}{}{}{}{}\n\n\
            En tant que modérateur, produis une synthèse structurée :\n\
            {}\n\n\
            Utilise le format Markdown : titres (##, ###), listes à puces, **gras** pour les idées clés. \
            Sois équilibré, exhaustif et aéré — utilise des paragraphes courts et de l'espace pour la lisibilité.\n\n\
            [CONTRAINTE DE FORMAT STRICTE]\n\
            Tu NE DOIS PAS utiliser de tableaux Markdown (syntaxe | ... |) dans ta synthèse. Les tableaux s'affichent mal et sont INTERDITS.\n\
            Pour comparer les positions des intervenants, utilise cette structure :\n\
            ### Thème ou critère\n\
            - **Intervenant A** : sa position...\n\
            - **Intervenant B** : sa position...\n\n\
            IMPÉRATIF : Rédige l'intégralité de ta synthèse en français.",
            datetime, mode_desc, topic, memory.contextual_summary, positions, evolution, agendas_block, web_block, doc_block, full_doc_block, synth_instructions
        ),
    }
}

/// "[Évolution des positions]" block of the synthesis prompt: who moved, from
/// what to what — plus the instruction to devote a section to it. Empty when
/// nobody moved.
pub fn build_positions_evolution_block(memory: &ParticipantMemory, lang: &str) -> String {
    let mut moved: Vec<(&String, &ParticipantPosition)> = memory
        .positional_map
        .iter()
        .filter(|(_, p)| p.has_evolved())
        .collect();
    if moved.is_empty() {
        return String::new();
    }
    moved.sort_by(|a, b| a.0.cmp(b.0));
    let (header, instruction, unknown) = match lang {
        "en" => ("[Evolution of positions]", "Devote a \"## Evolution of positions\" section to these shifts: who moved, from what to what, and what made them move.", "unknown"),
        "zh" => ("[立场演变]", "专门用一个\"## 立场演变\"部分说明这些变化：谁改变了、从什么到什么、是什么促成了改变。", "未知"),
        _ => ("[Évolution des positions]", "Consacre une section \"## Évolution des positions\" à ces mouvements : qui a bougé, de quoi vers quoi, et ce qui l'a fait bouger.", "inconnue"),
    };
    let lines = moved
        .iter()
        .map(|(name, p)| {
            let initial = p.initial_stance.as_deref().unwrap_or(unknown);
            let shift = p
                .shift
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| format!(" ({s})"))
                .unwrap_or_default();
            format!("- {name} : {initial} → {}{shift}", p.stance)
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("\n\n{header}\n{lines}\n{instruction}")
}

/// "[Sources utilisées]" block for the synthesis: the most recent references
/// (one line each, capped), plus the instruction to close with a Sources section.
/// Empty when nothing was injected during the discussion.
pub fn build_sources_block(used_sources: &[SourceRecord], lang: &str, max_chars: usize) -> String {
    if used_sources.is_empty() || max_chars == 0 {
        return String::new();
    }
    let (header, instruction) = match lang {
        "en" => ("[Sources used during the discussion]", "End your synthesis with a \"## Sources\" section listing, as a Markdown list, only the links above that were actually used or cited. Never invent a link."),
        "zh" => ("[讨论中使用的来源]", "在综合报告末尾添加一个\"## 来源\"部分，以 Markdown 列表列出上面真正使用或引用的链接。绝不虚构链接。"),
        _ => ("[Sources utilisées pendant la discussion]", "Termine ta synthèse par une section \"## Sources\" listant, sous forme de liste Markdown, uniquement les liens ci-dessus réellement utilisés ou cités. N'invente jamais de lien."),
    };
    let mut lines: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for s in used_sources.iter().rev() {
        if !seen.insert(s.url.as_str()) {
            continue;
        }
        lines.push(format!(
            "- [{}] {} — {} ({}, tour {})",
            s.kind.label(lang),
            s.title,
            s.url,
            s.speaker_name,
            s.turn
        ));
        if lines.len() >= constants::SYNTHESIS_SOURCES_MAX_ENTRIES {
            break;
        }
    }
    lines.reverse();
    let mut block = format!("\n\n{header}\n");
    for line in lines {
        if block.len() + line.len() + 1 > max_chars {
            break;
        }
        block.push_str(&line);
        block.push('\n');
    }
    block.push_str(instruction);
    block
}

/// Describe emotions as rich text for prompt injection.
/// Only mentions non-neutral axes (< 40 or > 60) for conciseness.
pub fn describe_emotions(emotions: &EmotionalProfile, lang: &str) -> String {
    let axes: [(u8, &str, &str, &str); 6] = [
        (emotions.engagement, "engagement", "engagement", "投入度"),
        (emotions.accord, "accord", "agreement", "赞同度"),
        (emotions.confiance, "confiance", "confidence", "信心"),
        (emotions.frustration, "frustration", "frustration", "挫败感"),
        (emotions.curiosite, "curiosité", "curiosity", "好奇心"),
        (emotions.enthousiasme, "enthousiasme", "enthusiasm", "热情"),
    ];

    let mut parts = Vec::new();
    for (val, fr_name, en_name, zh_name) in &axes {
        let desc = match lang {
            "en" => describe_axis_en(*val, en_name),
            "zh" => describe_axis_zh(*val, zh_name),
            _ => describe_axis_fr(*val, fr_name),
        };
        if let Some(d) = desc {
            parts.push(d);
        }
    }

    if parts.is_empty() {
        match lang {
            "en" => "You are in a neutral emotional state.".to_string(),
            "zh" => "你的情绪状态平稳。".to_string(),
            _ => "Tu es dans un état émotionnel neutre.".to_string(),
        }
    } else {
        parts.join(" ")
    }
}

fn describe_axis_fr(val: u8, name: &str) -> Option<String> {
    match val {
        0..=20 => Some(format!("Tu ressens très peu de {} ({}/100).", name, val)),
        21..=40 => Some(format!("Ton {} est plutôt bas ({}/100).", name, val)),
        61..=80 => Some(format!("Tu es assez haut en {} ({}/100).", name, val)),
        81..=100 => Some(format!(
            "Tu es intensément habité par le/la {} ({}/100).",
            name, val
        )),
        _ => None, // 41-60: neutral, skip
    }
}

fn describe_axis_en(val: u8, name: &str) -> Option<String> {
    match val {
        0..=20 => Some(format!("You feel very low {} ({}/100).", name, val)),
        21..=40 => Some(format!("Your {} is rather low ({}/100).", name, val)),
        61..=80 => Some(format!("You feel fairly high {} ({}/100).", name, val)),
        81..=100 => Some(format!(
            "You are intensely experiencing {} ({}/100).",
            name, val
        )),
        _ => None,
    }
}

fn describe_axis_zh(val: u8, name: &str) -> Option<String> {
    match val {
        0..=20 => Some(format!("你的{}非常低 ({}/100)。", name, val)),
        21..=40 => Some(format!("你的{}偏低 ({}/100)。", name, val)),
        61..=80 => Some(format!("你的{}较高 ({}/100)。", name, val)),
        81..=100 => Some(format!("你的{}非常强烈 ({}/100)。", name, val)),
        _ => None,
    }
}

/// Build threshold-specific factual alerts (only when above/below critical values).
/// Returns None if no threshold is crossed.
/// Note: behavioral responses are now handled by the <dynamics> section of each persona template.
pub fn build_threshold_instructions(emotions: &EmotionalProfile, lang: &str) -> Option<String> {
    let mut instructions = Vec::new();

    if emotions.frustration > constants::EMOTION_HIGH_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your frustration level is critical.".to_string(),
            "zh" => "⚠ 你的挫败感已达临界水平。".to_string(),
            _ => "⚠ Ton niveau de frustration est critique.".to_string(),
        });
    }
    if emotions.engagement < constants::EMOTION_LOW_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your engagement is at rock bottom.".to_string(),
            "zh" => "⚠ 你的投入度已降至最低。".to_string(),
            _ => "⚠ Ton engagement est au plus bas.".to_string(),
        });
    }
    if emotions.confiance > constants::EMOTION_HIGH_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your confidence is at its peak.".to_string(),
            "zh" => "⚠ 你的信心已达巅峰。".to_string(),
            _ => "⚠ Ta confiance est à son maximum.".to_string(),
        });
    }
    if emotions.confiance < constants::EMOTION_LOW_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your confidence is at rock bottom.".to_string(),
            "zh" => "⚠ 你的信心已降至最低。".to_string(),
            _ => "⚠ Ta confiance est au plus bas.".to_string(),
        });
    }
    if emotions.curiosite > constants::EMOTION_HIGH_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your curiosity is at its peak.".to_string(),
            "zh" => "⚠ 你的好奇心已达巅峰。".to_string(),
            _ => "⚠ Ta curiosité est à son comble.".to_string(),
        });
    }
    if emotions.enthousiasme > constants::EMOTION_HIGH_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your enthusiasm is at its peak.".to_string(),
            "zh" => "⚠ 你的热情已达巅峰。".to_string(),
            _ => "⚠ Ton enthousiasme est à son maximum.".to_string(),
        });
    }

    if instructions.is_empty() {
        None
    } else {
        Some(instructions.join(" "))
    }
}

/// Build the prompt for LLM-based emotion analysis of all participants.
/// Returns a single prompt that asks for signed deltas for each participant.
pub fn build_emotion_analysis_prompt(
    participants_json: &str,
    recent_context: &str,
    events_summary: &str,
    lang: &str,
    ) -> String {
    let cap = constants::EMOTION_LLM_DELTA_CAP;
    let key = constants::EMOTION_STAGNATION_JSON_KEY;
    match lang {
        "en" => format!(
            "Analyze the emotional evolution of each participant based on the recent exchanges.\n\n\
            Participants and their current emotions (reactions and sanctions of this turn are ALREADY reflected in these values):\n{}\n\n\
            Recent exchanges:\n{}\n\n\
            Events this turn (already applied — for context only):\n{}\n\n\
            For EACH participant, provide signed deltas (positive or negative integers) reflecting ONLY the tone and content of what they said and heard — \
            do NOT re-count likes, dislikes or bans.\n\
            Keep deltas in the range [-{cap}, +{cap}]. Use 0 for axes that shouldn't change; most axes should stay at 0 on an ordinary turn.\n\
            Drops are as expected as rises: whoever was contradicted, ignored or caught out loses confiance or accord; eloquence alone earns nothing.\n\
            Also set \"{key}\" to true ONLY if the exchanges repeat earlier points without any new idea, fact or angle; otherwise false.\n\n\
            Respond with ONLY a JSON object:\n\
            {{\"Participant Name\": {{\"engagement\": 0, \"accord\": 0, \"confiance\": 0, \"frustration\": 0, \"curiosite\": 0, \"enthousiasme\": 0}}, ..., \"{key}\": false}}",
            participants_json, recent_context, events_summary
        ),
        "zh" => format!(
            "根据最近的对话分析每位参与者的情绪变化。\n\n\
            参与者及其当前情绪（本轮的反应和处罚已经反映在这些数值中）：\n{}\n\n\
            最近的对话：\n{}\n\n\
            本轮事件（已经生效——仅供参考）：\n{}\n\n\
            为每位参与者提供有符号增量（正数或负数整数），仅反映其所说和所听内容的语气与内容——不要重复计算点赞、点踩或禁言。\n\
            增量范围为 [-{cap}, +{cap}]。如果某个轴不需要变化，使用 0；平常的一轮大多数轴应保持为 0。\n\
            下降和上升同样正常：被反驳、被忽视或被抓住把柄的人会失去 confiance 或 accord；仅凭口才不能得分。\n\
            另外，仅当交流只是重复先前的观点而没有任何新想法、事实或角度时，将 \"{key}\" 设为 true；否则为 false。\n\n\
            仅用 JSON 对象回复：\n\
            {{\"参与者名称\": {{\"engagement\": 0, \"accord\": 0, \"confiance\": 0, \"frustration\": 0, \"curiosite\": 0, \"enthousiasme\": 0}}, ..., \"{key}\": false}}",
            participants_json, recent_context, events_summary
        ),
        _ => format!(
            "Analyse l'évolution émotionnelle de chaque participant en fonction des échanges récents.\n\n\
            Participants et leurs émotions actuelles (les réactions et sanctions de ce tour sont DÉJÀ intégrées dans ces valeurs) :\n{}\n\n\
            Échanges récents :\n{}\n\n\
            Événements de ce tour (déjà appliqués — pour contexte uniquement) :\n{}\n\n\
            Pour CHAQUE participant, fournis des deltas signés (entiers positifs ou négatifs) reflétant UNIQUEMENT le ton et le contenu de ce qu'il a dit et entendu — \
            ne recompte PAS les likes, dislikes ou bannissements.\n\
            Garde les deltas dans la plage [-{cap}, +{cap}]. Utilise 0 pour les axes qui ne changent pas ; sur un tour ordinaire, la plupart des axes restent à 0.\n\
            Les baisses sont aussi attendues que les hausses : qui a été contredit, ignoré ou pris en défaut perd de la confiance ou de l'accord ; l'éloquence seule ne rapporte rien.\n\
            Renseigne aussi \"{key}\" à true UNIQUEMENT si les échanges répètent des points déjà faits sans idée, fait ou angle nouveau ; sinon false.\n\n\
            Réponds UNIQUEMENT avec un objet JSON :\n\
            {{\"Nom du Participant\": {{\"engagement\": 0, \"accord\": 0, \"confiance\": 0, \"frustration\": 0, \"curiosite\": 0, \"enthousiasme\": 0}}, ..., \"{key}\": false}}",
            participants_json, recent_context, events_summary
        ),
    }
}

/// Build end-of-discussion awareness for thought prompts
fn build_end_awareness_thought(current_turn: u32, max_turns: Option<u32>, lang: &str) -> String {
    let Some(max) = max_turns else {
        return String::new();
    };
    if current_turn + 1 >= max {
        match lang {
            "en" => "\n4. This is one of the last turns — how will you wrap up your contribution?"
                .to_string(),
            "zh" => "\n4. 这是最后几轮之一——你将如何总结你的贡献？".to_string(),
            _ => "\n4. C'est l'un des derniers tours — comment vas-tu conclure ta contribution ?"
                .to_string(),
        }
    } else {
        String::new()
    }
}

/// Build end-of-discussion awareness instructions
fn build_end_awareness(current_turn: u32, max_turns: Option<u32>, lang: &str) -> String {
    let Some(max) = max_turns else {
        return String::new();
    };
    if current_turn >= max {
        // Last turn
        match lang {
            "en" => {
                " This is the LAST turn of the discussion. Make your final contribution count — \
            wrap up your contribution clearly and address any remaining open points."
                    .to_string()
            }
            "zh" => " 这是讨论的最后一轮。让你的最终贡献有分量——\
            清楚总结你的贡献并回应剩余的未解决问题。"
                .to_string(),
            _ => " C'est le DERNIER tour de la discussion. Fais compter ta contribution finale — \
            conclus ta contribution clairement et adresse les points restants en suspens."
                .to_string(),
        }
    } else if current_turn + 1 >= max {
        // Penultimate turn
        match lang {
            "en" => " The discussion is nearing its end (next turn is the last). \
            Start refining your key points and working toward a conclusion."
                .to_string(),
            "zh" => " 讨论即将结束（下一轮是最后一轮）。\
            开始精炼你的要点并努力达成结论。"
                .to_string(),
            _ => " La discussion approche de sa fin (le prochain tour est le dernier). \
            Commence à affiner tes points clés et à travailler vers une conclusion."
                .to_string(),
        }
    } else if max > 3 && current_turn + 2 >= max {
        // Two turns before end (only if max > 3)
        match lang {
            "en" => " The discussion will end soon (2 turns remaining). \
            Focus on your strongest points."
                .to_string(),
            "zh" => " 讨论即将结束（还剩2轮）。集中于你最有力的要点。".to_string(),
            _ => " La discussion se terminera bientôt (2 tours restants). \
            Concentre-toi sur tes points les plus forts."
                .to_string(),
        }
    } else {
        String::new()
    }
}

/// Build the democratic voting prompt for a gladiator.
/// The gladiator ranks OTHER active speakers by who should speak first.
pub fn build_democratic_vote_prompt(
    voter_name: &str,
    other_active_names: &[String],
    topic: &str,
    discussion_summary: &str,
    discussion_language: &str,
    ) -> String {
    let names_list = other_active_names.join(", ");
    let context = if discussion_summary.is_empty() {
        match discussion_language {
            "en" => format!("The topic is: \"{topic}\". The discussion has not started yet."),
            "zh" => format!("主题是：\"{topic}\"。讨论尚未开始。"),
            _ => format!("Le sujet est : \"{topic}\". La discussion n'a pas encore commencé."),
        }
    } else {
        match discussion_language {
            "en" => format!("The topic is: \"{topic}\"\nDiscussion so far: {discussion_summary}"),
            "zh" => format!("主题是：\"{topic}\"\n目前讨论内容：{discussion_summary}"),
            _ => format!("Le sujet est : \"{topic}\"\nDiscussion jusqu'ici : {discussion_summary}"),
        }
    };

    match discussion_language {
        "en" => format!(
            "You are {voter_name}. Rank the following participants in the order you think they \
            should speak next, from most relevant to least relevant.\n\n\
            {context}\n\n\
            Participants to rank: {names_list}\n\n\
            Return a JSON object: {{\"ranking\": [\"first_to_speak\", \"second\", ...]}}\n\
            Include ALL participants listed above. Respond ONLY with the JSON, no text before or after.",
        ),
        "zh" => format!(
            "你是{voter_name}。按你认为应该先发言的顺序排列以下参与者，从最相关到最不相关。\n\n\
            {context}\n\n\
            需要排列的参与者：{names_list}\n\n\
            返回JSON对象：{{\"ranking\": [\"最先发言的\", \"第二个\", ...]}}\n\
            包含以上列出的所有参与者。仅用JSON回复，前后不要有任何文字。",
        ),
        _ => format!(
            "Tu es {voter_name}. Classe les participants suivants dans l'ordre où tu penses \
            qu'ils devraient parler, du plus pertinent au moins pertinent.\n\n\
            {context}\n\n\
            Participants à classer : {names_list}\n\n\
            Retourne un objet JSON : {{\"ranking\": [\"premier_à_parler\", \"deuxième\", ...]}}\n\
            Inclus TOUS les participants listés ci-dessus. Réponds UNIQUEMENT avec le JSON, \
            pas de texte avant ou après.",
        ),
    }
}

/// Build the authoritarian ordering prompt for the IArbitre.
/// The IArbitre decides the full speaking order for this turn.
pub fn build_authoritarian_order_prompt(
    active_names: &[String],
    topic: &str,
    discussion_summary: &str,
    current_turn: u32,
    discussion_language: &str,
    ) -> String {
    let names_list = active_names.join(", ");
    let context = if discussion_summary.is_empty() {
        match discussion_language {
            "en" => format!("This is the opening turn. The topic is: \"{topic}\"."),
            "zh" => format!("这是开场轮次。主题是：\"{topic}\"。"),
            _ => format!("C'est le tour d'ouverture. Le sujet est : \"{topic}\"."),
        }
    } else {
        match discussion_language {
            "en" => format!("Topic: \"{topic}\"\nDiscussion summary: {discussion_summary}"),
            "zh" => format!("主题：\"{topic}\"\n讨论摘要：{discussion_summary}"),
            _ => format!("Sujet : \"{topic}\"\nRésumé de la discussion : {discussion_summary}"),
        }
    };

    match discussion_language {
        "en" => format!(
            "As moderator, decide the speaking order for turn {current_turn}.\n\n\
            {context}\n\n\
            Active participants: {names_list}\n\n\
            Consider: who has the most relevant point to make first? Who should respond to whom? \
            Use your judgement to create the most productive discussion order.\n\
            Return a JSON object: {{\"order\": [\"first_speaker\", \"second_speaker\", ...]}}\n\
            Include ALL active participants. Respond ONLY with the JSON, no text before or after.",
        ),
        "zh" => format!(
            "作为主持人，决定第{current_turn}轮的发言顺序。\n\n\
            {context}\n\n\
            活跃参与者：{names_list}\n\n\
            考虑：谁最应该先发言？谁应该回应谁？用你的判断创造最有效的讨论顺序。\n\
            返回JSON对象：{{\"order\": [\"第一个发言者\", \"第二个发言者\", ...]}}\n\
            包含所有活跃参与者。仅用JSON回复，前后不要有任何文字。",
        ),
        _ => format!(
            "En tant que modérateur, décide l'ordre de parole pour le tour {current_turn}.\n\n\
            {context}\n\n\
            Participants actifs : {names_list}\n\n\
            Réfléchis : qui a le point le plus pertinent à faire en premier ? Qui devrait \
            répondre à qui ? Utilise ton jugement pour créer l'ordre de discussion le plus productif.\n\
            Retourne un objet JSON : {{\"order\": [\"premier_intervenant\", \"deuxième\", ...]}}\n\
            Inclus TOUS les participants actifs. Réponds UNIQUEMENT avec le JSON, \
            pas de texte avant ou après.",
        ),
    }
}

/// Build the tie-breaking prompt for the IArbitre when democratic voting results in a tie.
pub fn build_tiebreak_prompt(
    tied_names: &[String],
    topic: &str,
    discussion_summary: &str,
    current_turn: u32,
    discussion_language: &str,
    ) -> String {
    let names_list = tied_names.join(", ");
    let context = if discussion_summary.is_empty() {
        match discussion_language {
            "en" => format!("Topic: \"{topic}\". Turn {current_turn}."),
            "zh" => format!("主题：\"{topic}\"。第{current_turn}轮。"),
            _ => format!("Sujet : \"{topic}\". Tour {current_turn}."),
        }
    } else {
        match discussion_language {
            "en" => {
                format!("Topic: \"{topic}\". Turn {current_turn}.\nSummary: {discussion_summary}")
            }
            "zh" => format!("主题：\"{topic}\"。第{current_turn}轮。\n摘要：{discussion_summary}"),
            _ => {
                format!("Sujet : \"{topic}\". Tour {current_turn}.\nRésumé : {discussion_summary}")
            }
        }
    };

    match discussion_language {
        "en" => format!(
            "There is a tie in the democratic vote. The following participants received equal votes: \
            {names_list}\n\n\
            {context}\n\n\
            Decide their speaking order. Return a JSON object: {{\"order\": [\"first\", \"second\", ...]}}\n\
            Include ALL tied participants. Respond ONLY with the JSON, no text before or after.",
        ),
        "zh" => format!(
            "民主投票出现了平局。以下参与者获得了相同的票数：{names_list}\n\n\
            {context}\n\n\
            决定他们的发言顺序。返回JSON对象：{{\"order\": [\"第一个\", \"第二个\", ...]}}\n\
            包含所有平局参与者。仅用JSON回复，前后不要有任何文字。",
        ),
        _ => format!(
            "Il y a une égalité dans le vote démocratique. Les participants suivants ont reçu \
            le même nombre de voix : {names_list}\n\n\
            {context}\n\n\
            Décide leur ordre de parole. Retourne un objet JSON : {{\"order\": [\"premier\", \"deuxième\", ...]}}\n\
            Inclus TOUS les participants à égalité. Réponds UNIQUEMENT avec le JSON, \
            pas de texte avant ou après.",
        ),
    }
}

// ── Search prompts (shared helpers) ─────────────────────────────────

/// Build a block listing past queries so the LLM avoids repeating them.
fn build_past_queries_block(past_queries: &[String], lang: &str) -> String {
    if past_queries.is_empty() {
        return String::new();
    }
    let list = past_queries
        .iter()
        .map(|q| format!("  - \"{}\"", q))
        .collect::<Vec<_>>()
        .join("\n");
    match lang {
        "en" => format!(
            "\n\nYour previous searches:\n{}\nDo NOT repeat these queries. Search for something DIFFERENT that reflects YOUR unique angle, or return needs_search: false if you have enough information.",
            list
        ),
        "zh" => format!(
            "\n\n你之前的搜索：\n{}\n不要重复这些查询。搜索反映你独特视角的不同内容，或者如果你已有足够信息则返回 needs_search: false。",
            list
        ),
        _ => format!(
            "\n\nTes recherches précédentes :\n{}\nNe répète PAS ces requêtes. Cherche quelque chose de DIFFÉRENT qui reflète TON angle unique, ou retourne needs_search: false si tu as déjà assez d'informations.",
            list
        ),
    }
}

/// Build a block listing what OTHER speakers have already searched this turn.
fn build_other_queries_block(other_queries: &[(String, String)], lang: &str) -> String {
    if other_queries.is_empty() {
        return String::new();
    }
    let list = other_queries
        .iter()
        .map(|(name, q)| format!("  - {} → \"{}\"", name, q))
        .collect::<Vec<_>>()
        .join("\n");
    match lang {
        "en" => format!(
            "\n\nOther speakers already searched THIS TURN:\n{}\nDo NOT search the same things. Find a DIFFERENT angle that reflects YOUR expertise.",
            list
        ),
        "zh" => format!(
            "\n\n本轮其他发言者已搜索：\n{}\n不要搜索相同内容。找到反映你专业知识的不同角度。",
            list
        ),
        _ => format!(
            "\n\nLes autres intervenants ont déjà cherché CE TOUR :\n{}\nNe cherche PAS les mêmes choses. Trouve un angle DIFFÉRENT qui reflète TON expertise.",
            list
        ),
    }
}

// ── Web Search prompts ──────────────────────────────────────────────

/// Build a prompt asking the LLM whether it needs to search the web.
pub fn build_web_search_decision_prompt(
    topic: &str,
    recent_context: &str,
    search_directive: &str,
    searches_remaining: u32,
    discussion_language: &str,
    past_queries: &[String],
    other_queries: &[(String, String)],
    ) -> String {
    let datetime = build_datetime_context(discussion_language);
    let past_block = build_past_queries_block(past_queries, discussion_language);
    let others_block = build_other_queries_block(other_queries, discussion_language);
    match discussion_language {
        "en" => format!(
            "{}\n\nYou have access to internet search. {}\n\
            Topic: \"{}\"\n\
            Recent context: {}\n\
            Remaining searches: {}{}{}\n\n\
            Based on YOUR unique expertise, do you need specific factual information to strengthen YOUR contribution?\n\
            If yes, provide exactly ONE short, relevant search query that reflects YOUR perspective.\n\
            IMPORTANT: Write the search query in English.\n\
            Respond ONLY with this JSON:\n\
            {{\"needs_search\": true, \"queries\": [\"your single query\"]}}\n\
            or\n\
            {{\"needs_search\": false, \"queries\": []}}",
            datetime, search_directive, topic, recent_context, searches_remaining, past_block, others_block
        ),
        "zh" => format!(
            "{}\n\n你可以使用互联网搜索。{}\n\
            主题：\"{}\"\n\
            近期背景：{}\n\
            剩余搜索次数：{}{}{}\n\n\
            基于你独特的专业知识，你需要具体的事实信息来加强你的贡献吗？\n\
            如果是，提供恰好一个反映你视角的简短相关搜索查询。\n\
            重要：用中文撰写搜索查询。\n\
            仅用以下JSON格式回复：\n\
            {{\"needs_search\": true, \"queries\": [\"你的查询\"]}}\n\
            或\n\
            {{\"needs_search\": false, \"queries\": []}}",
            datetime, search_directive, topic, recent_context, searches_remaining, past_block, others_block
        ),
        _ => format!(
            "{}\n\nTu as accès à la recherche internet. {}\n\
            Sujet : \"{}\"\n\
            Contexte récent : {}\n\
            Recherches restantes : {}{}{}\n\n\
            En fonction de TON expertise unique, as-tu besoin d'informations factuelles spécifiques pour renforcer TA contribution ?\n\
            Si oui, fournis exactement UNE requête de recherche courte et pertinente qui reflète TA perspective.\n\
            IMPORTANT : Formule la requête de recherche en français.\n\
            Réponds UNIQUEMENT avec ce JSON :\n\
            {{\"needs_search\": true, \"queries\": [\"ta requête\"]}}\n\
            ou\n\
            {{\"needs_search\": false, \"queries\": []}}",
            datetime, search_directive, topic, recent_context, searches_remaining, past_block, others_block
        ),
    }
}

/// Default search directive per language.
pub fn default_search_directive(lang: &str) -> &'static str {
    match lang {
        "en" => "Use internet search to find the latest information on the topic, factual arguments with figures and data, verify claims made by other participants, or deepen your expertise on a subject you are less familiar with.",
        "zh" => "使用互联网搜索来查找有关主题的最新信息、带有数据和数字的事实论据、验证其他参与者的说法、或加深你不太熟悉的领域的专业知识。",
        _ => "Utilise la recherche internet pour trouver les dernières informations sur le sujet, des arguments factuels avec des chiffres et données, vérifier les affirmations des autres participants, ou approfondir ton expertise sur un domaine que tu maîtrises moins.",
    }
}

/// Format Tavily search results as context to inject into prompts.
/// Truncates individual results and total output to stay within prompt budget.
pub fn build_search_results_context(
    results: &[(String, TavilySearchResponse)],
    discussion_language: &str,
    ) -> String {
    let header = match discussion_language {
        "en" => "[Internet results — current data, recent news, fact-checking]",
        "zh" => "[互联网结果 — 最新数据、近期新闻、事实核查]",
        _ => "[Résultats internet — actualité, données récentes, vérifications]",
    };

    let query_label = match discussion_language {
        "en" => "Query",
        "zh" => "查询",
        _ => "Requête",
    };

    let summary_label = match discussion_language {
        "en" => "Summary",
        "zh" => "摘要",
        _ => "Résumé",
    };

    let sources_label = match discussion_language {
        "en" => "Sources",
        "zh" => "来源",
        _ => "Sources",
    };

    let mut output = String::from(header);
    output.push('\n');

    for (query, response) in results {
        output.push_str(&format!("{}: \"{}\"\n", query_label, query));

        if let Some(answer) = &response.answer {
            if !answer.is_empty() {
                output.push_str(&format!(
                    "{}: {}\n",
                    summary_label,
                    truncate(answer, constants::SEARCH_TAVILY_ANSWER)
                ));
            }
        }

        if !response.results.is_empty() {
            output.push_str(&format!("{}:\n", sources_label));
            for (i, result) in response
                .results
                .iter()
                .take(constants::SEARCH_WEB_RENDER_LIMIT)
                .enumerate()
            {
                // Extract domain from URL
                let domain = result
                    .url
                    .split("//")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .unwrap_or(&result.url);
                output.push_str(&format!(
                    "{}. \"{}\" ({}) : {}\n",
                    i + 1,
                    truncate(&result.title, constants::SEARCH_WEB_TITLE),
                    domain,
                    truncate(&result.content, constants::SEARCH_WEB_CONTENT)
                ));
            }
        }
        output.push('\n');

        // Hard limit on total output
        if output.len() > constants::SEARCH_MAX_CONTEXT_LEN {
            let boundary = output.floor_char_boundary(constants::SEARCH_MAX_CONTEXT_LEN);
            output.truncate(boundary);
            break;
        }
    }

    output
}

// ── Wikipedia Search prompts ──────────────────────────────────────────

/// Default Wikipedia search directive per language.
pub fn default_wiki_directive(lang: &str) -> &'static str {
    match lang {
        "en" => "Use Wikipedia to find encyclopedic definitions, historical context, scientific concepts, and established facts relevant to the discussion.",
        "zh" => "使用维基百科查找与讨论相关的百科定义、历史背景、科学概念和既定事实。",
        _ => "Utilise Wikipédia pour trouver des définitions encyclopédiques, du contexte historique, des concepts scientifiques et des faits établis pertinents à la discussion.",
    }
}

/// Build a prompt asking the LLM whether it needs to search Wikipedia.
/// `web_context` contains web search results (if any) to inform the wiki query choice.
#[allow(clippy::too_many_arguments)]
pub fn build_wiki_search_decision_prompt(
    topic: &str,
    recent_context: &str,
    search_directive: &str,
    searches_remaining: u32,
    discussion_language: &str,
    past_queries: &[String],
    web_context: Option<&str>,
    other_queries: &[(String, String)],
    ) -> String {
    let datetime = build_datetime_context(discussion_language);
    let past_block = build_past_queries_block(past_queries, discussion_language);
    let others_block = build_other_queries_block(other_queries, discussion_language);
    let web_block = web_context.map(|ctx| {
        match discussion_language {
            "en" => format!("\n\n[Internet search results already available]\n{}\nUse Wikipedia to COMPLEMENT this with encyclopedic depth, definitions, or historical context — do NOT duplicate what internet search already found.", ctx),
            "zh" => format!("\n\n[已有的互联网搜索结果]\n{}\n使用维基百科来补充百科深度、定义或历史背景——不要重复互联网搜索已找到的内容。", ctx),
            _ => format!("\n\n[Résultats de recherche internet déjà disponibles]\n{}\nUtilise Wikipédia pour COMPLÉTER avec de la profondeur encyclopédique, des définitions ou du contexte historique — ne duplique PAS ce que la recherche internet a déjà trouvé.", ctx),
        }
    }).unwrap_or_default();
    match discussion_language {
        "en" => format!(
            "{}\n\nYou have access to Wikipedia. {}\n\
            Topic: \"{}\"\n\
            Recent context: {}\n\
            Remaining searches: {}{}{}{}\n\n\
            Based on YOUR unique expertise, choose a Wikipedia article that would help YOU bring an ORIGINAL perspective to this discussion.\n\
            Use the EXACT Wikipedia article title as it appears on the site (e.g., \"Twenty-second Amendment to the United States Constitution\", NOT \"Amendment 22\").\n\
            Write numbers as words in article titles (\"Twenty-second\", not \"22nd\").\n\
            IMPORTANT: Write the article title in English.\n\
            Respond ONLY with this JSON:\n\
            {{\"needs_search\": true, \"queries\": [\"Exact article title\"]}}\n\
            or {{\"needs_search\": false, \"queries\": []}} if you already have enough information.",
            datetime, search_directive, topic, recent_context, searches_remaining, past_block, others_block, web_block
        ),
        "zh" => format!(
            "{}\n\n你可以使用维基百科。{}\n\
            主题：\"{}\"\n\
            近期背景：{}\n\
            剩余搜索次数：{}{}{}{}\n\n\
            基于你独特的专业知识，选择一篇能帮助你为这场讨论带来原创视角的维基百科文章。\n\
            使用维基百科上显示的确切文章标题（例如：「美利坚合众国宪法第二十二条修正案」，而不是「修正案22」）。\n\
            重要：用中文撰写文章标题。\n\
            仅用以下JSON格式回复：\n\
            {{\"needs_search\": true, \"queries\": [\"确切文章标题\"]}}\n\
            或 {{\"needs_search\": false, \"queries\": []}} 如果你已有足够信息。",
            datetime, search_directive, topic, recent_context, searches_remaining, past_block, others_block, web_block
        ),
        _ => format!(
            "{}\n\nTu as accès à Wikipédia. {}\n\
            Sujet : \"{}\"\n\
            Contexte récent : {}\n\
            Recherches restantes : {}{}{}{}\n\n\
            En fonction de TON expertise unique, choisis un article Wikipédia qui t'aiderait à apporter un angle ORIGINAL à cette discussion.\n\
            Utilise le titre EXACT de l'article Wikipédia tel qu'il apparaît sur le site (ex : \"Vingt-deuxième amendement de la Constitution des États-Unis\", PAS \"Amendement 22\").\n\
            Écris les nombres en toutes lettres dans les titres d'articles (\"Vingt-deuxième\", pas \"22e\").\n\
            IMPORTANT : Écris le titre d'article en français.\n\
            Réponds UNIQUEMENT avec ce JSON :\n\
            {{\"needs_search\": true, \"queries\": [\"Titre exact d'article\"]}}\n\
            ou {{\"needs_search\": false, \"queries\": []}} si tu as déjà assez d'informations.",
            datetime, search_directive, topic, recent_context, searches_remaining, past_block, others_block, web_block
        ),
    }
}

/// Format Wikipedia search results as context to inject into prompts.
pub fn build_wiki_results_context(
    results: &[(String, WikiSearchResponse)],
    discussion_language: &str,
    ) -> String {
    let header = match discussion_language {
        "en" => "[Wikipedia results — encyclopedic context, definitions, established facts]",
        "zh" => "[维基百科结果 — 百科背景、定义、既定事实]",
        _ => "[Résultats Wikipédia — contexte encyclopédique, définitions, faits établis]",
    };

    let mut output = String::from(header);
    output.push('\n');

    for (query, response) in results {
        output.push_str(&format!("\"{}\"\n", query));

        if let Some(ref query_data) = response.query {
            let mut pages = query_data.pages.clone();
            pages.sort_by_key(|p| p.index);

            for (i, page) in pages
                .iter()
                .take(constants::WIKI_RESULTS_LIMIT as usize)
                .enumerate()
            {
                if page.extract.is_empty() {
                    continue;
                }
                output.push_str(&format!(
                    "{}. \"{}\" : {}\n",
                    i + 1,
                    truncate(&page.title, constants::SEARCH_WIKI_TITLE),
                    truncate(&page.extract, constants::SEARCH_WIKI_EXTRACT)
                ));
            }
        }
        output.push('\n');

        if output.len() > constants::SEARCH_MAX_CONTEXT_LEN {
            let boundary = output.floor_char_boundary(constants::SEARCH_MAX_CONTEXT_LEN);
            output.truncate(boundary);
            break;
        }
    }

    output
}

/// Build a [Reference Document] block for full document injection.
/// Returns an empty string if `full_document` is `None` or empty.
/// The document text is truncated to `max_chars` (from the token budget allocation).
fn build_full_document_block(full_document: Option<&str>, max_chars: usize, lang: &str) -> String {
    let doc = match full_document {
        Some(d) if !d.is_empty() => d,
        _ => return String::new(),
    };
    let truncated = truncate(doc, max_chars);
    let (header, instruction) = match lang {
        "en" => (
            "[REFERENCE DOCUMENT — Imported Knowledge Base]",
            "CRITICAL INSTRUCTION: Read and memorize the entire content of this document carefully. \
            This document has been provided to you specifically so that you can draw on its content \
            during the discussion. Internalize the information, concepts, data, and details it contains. \
            Then, throughout the conversation, spontaneously and naturally leverage this knowledge \
            whenever it is relevant to the topic being discussed — whether to support an argument, \
            enrich your reasoning, correct an inaccuracy, or bring a concrete element into the exchange. \
            Never ignore this document: it is your primary knowledge base for this discussion.",
        ),
        "zh" => (
            "[参考文档 — 导入的知识库]",
            "关键指令：请仔细阅读并记忆本文档的全部内容。\
            本文档专门提供给你，以便你在讨论中利用其内容。\
            请内化其中的信息、概念、数据和细节。\
            然后在整个对话过程中，每当与讨论主题相关时，\
            自发而自然地运用这些知识——无论是支持论点、丰富推理、\
            纠正不准确之处，还是为交流带来具体的元素。\
            切勿忽视本文档：它是你在本次讨论中的主要知识库。",
        ),
        _ => (
            "[DOCUMENT DE RÉFÉRENCE — Base de connaissances importée]",
            "INSTRUCTION CRITIQUE : Lis et mémorise attentivement l'intégralité du contenu de ce document. \
            Ce document t'a été fourni spécifiquement pour que tu puisses t'appuyer sur son contenu \
            au cours de la discussion. Intériorise les informations, concepts, données et détails qu'il contient. \
            Puis, tout au long de la conversation, exploite spontanément et naturellement ces connaissances \
            chaque fois qu'elles sont pertinentes par rapport au sujet discuté — que ce soit pour étayer \
            un argument, enrichir ton raisonnement, corriger une inexactitude, ou apporter un élément \
            concret à l'échange. N'ignore jamais ce document : c'est ta base de connaissances principale \
            pour cette discussion.",
        ),
    };
    format!(
        "\n\n{}\n{}\n\n--- DOCUMENT ---\n{}\n--- FIN ---",
        header, instruction, truncated
    )
}

/// Build a date/time context string with timezone offset.
/// Uses `%:z` format (e.g., "+01:00") instead of `%Z` which gives "Romance Standard Time" on Windows.
pub fn build_datetime_context(discussion_language: &str) -> String {
    let now = chrono::Local::now();
    let datetime = now.format("%Y-%m-%d %H:%M:%S %:z").to_string();
    match discussion_language {
        "en" => format!("[Current date and time] {}", datetime),
        "zh" => format!("[当前日期和时间] {}", datetime),
        _ => format!("[Date et heure actuelles] {}", datetime),
    }
}

/// Generate a short mood sentence based on the most extreme emotional axes.
/// Used to display a brief text under each participant in the emotion sidebar.
/// Returns a varied, full constructed sentence.
pub fn summarize_emotional_state(emotions: &EmotionalProfile, lang: &str) -> String {
    // Deterministic seed from all emotion values for variant selection
    let seed = (emotions.engagement as usize)
        .wrapping_mul(7)
        .wrapping_add(emotions.accord as usize)
        .wrapping_mul(13)
        .wrapping_add(emotions.confiance as usize)
        .wrapping_mul(17)
        .wrapping_add(emotions.frustration as usize)
        .wrapping_mul(23)
        .wrapping_add(emotions.curiosite as usize)
        .wrapping_mul(29)
        .wrapping_add(emotions.enthousiasme as usize);

    fn pick<'a>(options: &'a [&'a str], seed: usize) -> &'a str {
        options[seed % options.len()]
    }

    // Collect axes with their distance from neutral (50)
    let axes: [(&str, u8); 6] = [
        ("frustration", emotions.frustration),
        ("enthousiasme", emotions.enthousiasme),
        ("engagement", emotions.engagement),
        ("curiosite", emotions.curiosite),
        ("confiance", emotions.confiance),
        ("accord", emotions.accord),
    ];

    // Sort by distance from 50, descending
    let mut sorted = axes;
    sorted.sort_by(|a, b| {
        let da = (a.1 as i16 - 50).unsigned_abs();
        let db = (b.1 as i16 - 50).unsigned_abs();
        db.cmp(&da)
    });

    // Collect phrase fragments for up to 2 most extreme axes
    let mut phrases: Vec<&str> = Vec::new();

    for (i, &(axis, val)) in sorted.iter().take(2).enumerate() {
        let v = seed.wrapping_add(i * 37); // shift variant per axis position
        let phrase = match (axis, val) {
            ("frustration", fv) if fv >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(
                    &[
                        "tense and irritated",
                        "frustrated by the exchanges",
                        "visibly on edge",
                    ],
                    v,
                ),
                "zh" => pick(&["紧张且烦躁", "对交流感到不满", "明显焦躁不安"], v),
                _ => pick(
                    &[
                        "tendu et agacé",
                        "irrité par les échanges",
                        "au bord de l'exaspération",
                    ],
                    v,
                ),
            },
            ("frustration", fv) if fv <= constants::PERSONALITY_LOW_FRUSTRATION => match lang {
                "en" => pick(
                    &[
                        "calm and serene",
                        "relaxed and at ease",
                        "perfectly composed",
                    ],
                    v,
                ),
                "zh" => pick(&["平静而从容", "放松自在", "泰然自若"], v),
                _ => pick(
                    &["calme et serein", "détendu et apaisé", "parfaitement posé"],
                    v,
                ),
            },
            ("enthousiasme", ev) if ev >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(
                    &[
                        "enthusiastic about the discussion",
                        "fired up by the debate",
                        "brimming with energy",
                    ],
                    v,
                ),
                "zh" => pick(&["对讨论充满热情", "被辩论所激发", "精力充沛"], v),
                _ => pick(
                    &[
                        "enthousiasmé par les échanges",
                        "porté par l'élan du débat",
                        "galvanisé par la discussion",
                    ],
                    v,
                ),
            },
            ("enthousiasme", ev) if ev <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(
                    &[
                        "lacking enthusiasm",
                        "somewhat indifferent",
                        "showing little energy",
                    ],
                    v,
                ),
                "zh" => pick(&["缺乏热情", "显得漠不关心", "了无生气"], v),
                _ => pick(
                    &[
                        "peu enthousiaste",
                        "assez indifférent",
                        "sans entrain particulier",
                    ],
                    v,
                ),
            },
            ("engagement", ev) if ev >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(
                    &[
                        "deeply invested in the debate",
                        "fully engaged",
                        "absorbed in the discussion",
                    ],
                    v,
                ),
                "zh" => pick(&["深入参与辩论", "全身心投入", "沉浸在讨论中"], v),
                _ => pick(
                    &[
                        "très investi dans le débat",
                        "pleinement engagé",
                        "absorbé par la discussion",
                    ],
                    v,
                ),
            },
            ("engagement", ev) if ev <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(
                    &[
                        "detached from the discussion",
                        "somewhat disengaged",
                        "losing interest",
                    ],
                    v,
                ),
                "zh" => pick(&["对讨论超然", "有些心不在焉", "渐失兴趣"], v),
                _ => pick(
                    &[
                        "détaché de la discussion",
                        "en retrait du débat",
                        "de plus en plus distant",
                    ],
                    v,
                ),
            },
            ("curiosite", cv) if cv >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(
                    &[
                        "very curious about the arguments",
                        "intrigued by the ideas",
                        "eager to explore further",
                    ],
                    v,
                ),
                "zh" => pick(&["对论点非常好奇", "被各种观点所吸引", "渴望深入探究"], v),
                _ => pick(
                    &[
                        "très curieux des arguments avancés",
                        "intrigué par les idées échangées",
                        "avide de comprendre",
                    ],
                    v,
                ),
            },
            ("curiosite", cv) if cv <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(
                    &[
                        "showing little curiosity",
                        "unimpressed by the arguments",
                        "not particularly intrigued",
                    ],
                    v,
                ),
                "zh" => pick(&["缺乏好奇心", "对论点不以为然", "兴趣索然"], v),
                _ => pick(
                    &[
                        "peu curieux",
                        "pas vraiment intrigué",
                        "indifférent aux arguments",
                    ],
                    v,
                ),
            },
            ("confiance", cv) if cv >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(
                    &[
                        "confident in their position",
                        "assertive and self-assured",
                        "unwavering in conviction",
                    ],
                    v,
                ),
                "zh" => pick(
                    &["对自己的立场充满信心", "态度坚定而自信", "立场坚定不移"],
                    v,
                ),
                _ => pick(
                    &[
                        "confiant dans sa position",
                        "assuré et déterminé",
                        "sûr de son fait",
                    ],
                    v,
                ),
            },
            ("confiance", cv) if cv <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(
                    &[
                        "hesitant and uncertain",
                        "second-guessing their position",
                        "lacking conviction",
                    ],
                    v,
                ),
                "zh" => pick(&["犹豫不决", "在质疑自己的立场", "缺乏信念"], v),
                _ => pick(
                    &[
                        "hésitant et incertain",
                        "en proie au doute",
                        "peu sûr de lui",
                    ],
                    v,
                ),
            },
            ("accord", av) if av >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(
                    &[
                        "in agreement with the others",
                        "finding common ground",
                        "largely aligned with the group",
                    ],
                    v,
                ),
                "zh" => pick(&["与他人意见一致", "找到了共识", "基本认同大家的观点"], v),
                _ => pick(
                    &[
                        "en accord avec les autres",
                        "dans un esprit de consensus",
                        "aligné avec le groupe",
                    ],
                    v,
                ),
            },
            ("accord", av) if av <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(
                    &[
                        "in strong disagreement",
                        "at odds with the group",
                        "firmly opposed",
                    ],
                    v,
                ),
                "zh" => pick(&["强烈反对", "与大家意见相左", "立场对立"], v),
                _ => pick(
                    &[
                        "en net désaccord",
                        "en opposition franche",
                        "réfractaire aux idées avancées",
                    ],
                    v,
                ),
            },
            _ => continue,
        };
        phrases.push(phrase);
    }

    // Neutral fallback
    if phrases.is_empty() {
        return match lang {
            "en" => pick(
                &[
                    "Appears composed and attentive.",
                    "Seems calm and focused.",
                    "Looks measured and collected.",
                ],
                seed,
            ),
            "zh" => pick(
                &["表现冷静而专注。", "显得沉着冷静。", "看起来从容不迫。"],
                seed,
            ),
            _ => pick(
                &[
                    "Semble posé et attentif.",
                    "Paraît calme et concentré.",
                    "Se montre mesuré et à l'écoute.",
                ],
                seed,
            ),
        }
        .to_string();
    }

    // Varied sentence starters
    let starters_fr = ["Semble", "Paraît", "Se montre", "A l'air"];
    let starters_en = ["Seems", "Appears", "Feels", "Looks"];
    let starters_zh = ["看起来", "显得", "表现得"];

    let starter = match lang {
        "en" => pick(&starters_en, seed),
        "zh" => pick(&starters_zh, seed),
        _ => pick(&starters_fr, seed),
    };

    let body = if phrases.len() == 1 {
        phrases[0].to_string()
    } else {
        match lang {
            "zh" => format!("{}，{}", phrases[0], phrases[1]),
            _ => format!("{}, {}", phrases[0], phrases[1]),
        }
    };

    match lang {
        "zh" => format!("{}{}。", starter, body),
        _ => format!("{} {}.", starter, body),
    }
}

/// Capitalize the first character of a string (for English topic labels).
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

/// Returns mode-specific document instructions for how to update the shared document.
/// These instructions are injected into both the Pass 2 document update prompt and the
/// read-only document context for synthesis. Each mode defines actionable editing directives.
fn mode_document_instruction(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Synthesize the strongest arguments and supporting evidence into a structured position paper.",
        (DiscussionMode::Debate, "zh") => "将最有力的论点和支持证据综合为结构化的立场文件。",
        (DiscussionMode::Debate, _) => "Synthétise les arguments les plus forts et les preuves à l'appui dans un document de position structuré.",

        (DiscussionMode::Ideation, "en") => "Add new ideas as concise bullet points. Group related ideas under thematic headings. Remove duplicates.",
        (DiscussionMode::Ideation, "zh") => "以简洁的要点添加新想法。将相关想法按主题标题分组。删除重复项。",
        (DiscussionMode::Ideation, _) => "Ajoute les nouvelles idées sous forme de points concis. Regroupe les idées liées sous des titres thématiques. Supprime les doublons.",

        (DiscussionMode::CoConstruction, "en") => "Synthesize new proposals into well-structured, formal content. Refine existing sections for clarity, coherence, and completeness. Merge overlapping contributions.",
        (DiscussionMode::CoConstruction, "zh") => "将新提案综合为结构良好的正式内容。优化现有部分以提高清晰度、连贯性和完整性。合并重叠的贡献。",
        (DiscussionMode::CoConstruction, _) => "Synthétise les nouvelles propositions en contenu formel et bien structuré. Affine les sections existantes pour la clarté, la cohérence et la complétude. Fusionne les contributions qui se recoupent.",

        (DiscussionMode::UserDriven, "en") => "Integrate insights from the exchange into a coherent, user-oriented summary.",
        (DiscussionMode::UserDriven, "zh") => "将交流中的洞察整合为面向用户的连贯摘要。",
        (DiscussionMode::UserDriven, _) => "Intègre les enseignements de l'échange dans un résumé cohérent orienté utilisateur.",

        (DiscussionMode::Socratic, "en") => "Record key questions raised, insights uncovered, and open threads requiring further inquiry.",
        (DiscussionMode::Socratic, "zh") => "记录提出的关键问题、发现的洞察以及需要进一步探究的未决线索。",
        (DiscussionMode::Socratic, _) => "Inscris les questions clés soulevées, les enseignements découverts et les fils ouverts nécessitant une exploration supplémentaire.",

        (DiscussionMode::Tutorial, "en") => "Structure explanations, examples, and concept definitions into a clear pedagogical progression.",
        (DiscussionMode::Tutorial, "zh") => "将解释、示例和概念定义组织为清晰的教学进程。",
        (DiscussionMode::Tutorial, _) => "Structure les explications, exemples et définitions de concepts dans une progression pédagogique claire.",

        (DiscussionMode::CritiqueReview, "en") => "Organize findings under Strengths, Weaknesses, and Recommendations with specific evidence.",
        (DiscussionMode::CritiqueReview, "zh") => "将发现按优点、缺点和建议分类，附具体证据。",
        (DiscussionMode::CritiqueReview, _) => "Organise les constats en Forces, Faiblesses et Recommandations avec des preuves spécifiques.",

        (DiscussionMode::CollaborativeFiction, "en") => "Weave new narrative elements into the story while maintaining voice consistency and plot coherence.",
        (DiscussionMode::CollaborativeFiction, "zh") => "将新的叙事元素融入故事，同时保持声音一致性和情节连贯性。",
        (DiscussionMode::CollaborativeFiction, _) => "Intègre les nouveaux éléments narratifs dans l'histoire en maintenant la cohérence de la voix et de l'intrigue.",

        (DiscussionMode::Trial, "en") => "Record the charges, the evidence retained and contested, then the verdict and its grounds.",
        (DiscussionMode::Trial, "zh") => "记录罪状、采纳和被质疑的证据，然后是裁决及其理由。",
        (DiscussionMode::Trial, _) => "Consigne les charges, les preuves retenues et contestées, puis le verdict et sa motivation.",

        (DiscussionMode::OxfordDebate, "en") => "Structure as Motion, Arguments For, Arguments Against, Rebuttals, Audience verdict.",
        (DiscussionMode::OxfordDebate, "zh") => "按辩题、正方论点、反方论点、反驳、听众裁决来组织。",
        (DiscussionMode::OxfordDebate, _) => "Structure en Motion, Arguments Pour, Arguments Contre, Réfutations, Verdict du public.",

        (DiscussionMode::Negotiation, "en") => "Draft the agreement in progress: agreed clauses, open points, each party's commitments.",
        (DiscussionMode::Negotiation, "zh") => "起草进行中的协议：已同意的条款、未决要点、各方的承诺。",
        (DiscussionMode::Negotiation, _) => "Rédige l'accord en cours : clauses convenues, points ouverts, engagements de chaque partie.",

        (DiscussionMode::SixHats, "en") => "Organise by hat: facts, feelings, risks, benefits, ideas, decision.",
        (DiscussionMode::SixHats, "zh") => "按帽子组织：事实、感受、风险、益处、点子、决定。",
        (DiscussionMode::SixHats, _) => "Organise par chapeau : faits, ressentis, risques, bénéfices, idées, décision.",

        (DiscussionMode::CrisisCell, "en") => "Keep the crisis log: dispatches, decisions, actions, owners, remaining risks.",
        (DiscussionMode::CrisisCell, "zh") => "维护危机日志：急电、决定、行动、负责人、剩余风险。",
        (DiscussionMode::CrisisCell, _) => "Tiens le journal de crise : dépêches, décisions, actions, responsables, risques restants.",
    }
}

/// Returns format-specific instructions for document co-construction.
fn document_format_instruction(format: &str, lang: &str) -> &'static str {
    match format {
        "md" => match lang {
            "en" => "Use Markdown formatting (headings, lists, bold, code blocks).",
            "zh" => "使用Markdown格式（标题、列表、粗体、代码块）。",
            _ => "Utilise le format Markdown (titres, listes, gras, blocs de code).",
        },
        "csv" => match lang {
            "en" => "Use CSV format with ';' as separator. First line is the header.",
            "zh" => "使用CSV格式，分隔符为';'。第一行是标题行。",
            _ => {
                "Utilise le format CSV avec ';' comme séparateur. La première ligne est l'en-tête."
            }
        },
        _ => match lang {
            "en" => "Use plain text format.",
            "zh" => "使用纯文本格式。",
            _ => "Utilise le format texte libre.",
        },
    }
}

/// Build the prompt for Pass 2: document update via separate LLM call.
/// The LLM sees only the discussion ideas + current document, and outputs the updated document.
///
/// PE techniques applied:
/// - **Descriptor stacking** (system): "precision of a technical specification, objectivity of an
///   academic publication, clarity of an executive brief" — shifts token distribution toward formal register.
/// - **Context isolation** (user): Discussion text framed as "IDEAS and PROPOSALS" with "Extract ONLY
///   the substantive content" — prevents copy-paste of conversational language.
/// - **REMEMBER repetition** (user, end): Anti-contamination constraint repeated at end of prompt
///   for recency bias (+76% compliance, Google 2024).
/// - **Visual delimiters** (`=== ===`, `---`): Structural separation between sections.
/// - **Persona separation**: Document writer ≠ discussion participant.
pub fn build_document_update_prompt(
    current_doc: &str,
    format: &str,
    discussion_text: &str,
    mode: &DiscussionMode,
    lang: &str,
    topic: &str,
    ) -> (String, String) {
    let format_instruction = document_format_instruction(format, lang);
    let mode_instruction = mode_document_instruction(mode, lang);

    let doc_display = if current_doc.is_empty() {
        match lang {
            "en" => "[empty]",
            "zh" => "[空]",
            _ => "[vide]",
        }
    } else {
        current_doc
    };

    let system = match lang {
        "en" => "\
            === ROLE ===\n\
            You are a professional document editor. You write with the precision of a technical specification, \
            the objectivity of an academic publication, and the clarity of an executive brief.\n\n\
            === RULES ===\n\
            - Write impersonal, third-person, factual prose only\n\
            - Every sentence must stand alone as published content\n\
            - NEVER include: speaker names, \"I\", \"we\", \"as discussed\", \"it was mentioned\", conversational fillers\n\
            - NEVER reproduce discussion text — transform ideas into formal document content\n\
            - Output ONLY the updated document — no preamble, no commentary".to_string(),

            "zh" => "\
            === 角色 ===\n\
            你是专业文档编辑。你的写作具有技术规范的精确性、学术出版物的客观性和执行摘要的清晰度。\n\n\
            === 规则 ===\n\
            - 仅使用非人称、第三人称、事实性的文字\n\
            - 每句话都必须能独立作为发表内容\n\
            - 绝不包含：发言者姓名、\"我\"、\"我们\"、\"如讨论所述\"、\"有人提到\"、口语化表达\n\
            - 绝不复制讨论文本——将想法转化为正式文档内容\n\
            - 仅输出更新后的文档——无前言、无评论".to_string(),

            _ => "\
            === RÔLE ===\n\
            Tu es un éditeur de document professionnel. Tu rédiges avec la précision d'une spécification technique, \
            l'objectivité d'une publication académique et la clarté d'une note de synthèse.\n\n\
            === RÈGLES ===\n\
            - Rédige uniquement en prose impersonnelle, factuelle, à la troisième personne\n\
            - Chaque phrase doit se suffire à elle-même comme contenu publiable\n\
            - N'inclus JAMAIS : noms d'intervenants, « je », « nous », « comme discuté », « il a été mentionné », formules conversationnelles\n\
            - Ne reproduis JAMAIS le texte de la discussion — transforme les idées en contenu formel\n\
            - Produis UNIQUEMENT le document mis à jour — pas de préambule, pas de commentaire".to_string(),
        };

    let lang_instruction = match lang {
        "en" => "Write the document in English.",
        "zh" => "用中文撰写文档。",
        _ => "Rédige le document en français.",
    };

    let user = match lang {
        "en" => format!(
            "=== TOPIC ===\n\
            {topic}\n\n\
            === IDEAS FROM DISCUSSION ===\n\
            The following contains IDEAS and PROPOSALS expressed during a discussion.\n\
            Extract ONLY the substantive content — ignore conversational language.\n\
            ---\n{discussion_text}\n---\n\n\
            === CURRENT DOCUMENT (.{format}) ===\n\
            {format_instruction}\n\
            ---\n{doc_display}\n---\n\n\
            === YOUR TASK ===\n\
            Update the document above by integrating the relevant ideas.\n\
            {mode_instruction}\n\
            Output the COMPLETE updated document.\n\
            {lang_instruction}\n\n\
            REMEMBER: Write impersonal, formal prose. No speaker names, no discussion references, no \"I\" or \"we\"."
        ),
        "zh" => format!(
            "=== 主题 ===\n\
            {topic}\n\n\
            === 讨论中的想法 ===\n\
            以下内容包含讨论中表达的想法和提案。\n\
            仅提取实质性内容——忽略口语化表达。\n\
            ---\n{discussion_text}\n---\n\n\
            === 当前文档 (.{format}) ===\n\
            {format_instruction}\n\
            ---\n{doc_display}\n---\n\n\
            === 你的任务 ===\n\
            通过整合相关想法更新上述文档。\n\
            {mode_instruction}\n\
            输出完整的更新文档。\n\
            {lang_instruction}\n\n\
            记住：使用非人称、正式文字。不含发言者姓名、不引用讨论、不使用\"我\"或\"我们\"。"
        ),
        _ => format!(
            "=== SUJET ===\n\
            {topic}\n\n\
            === IDÉES ISSUES DE LA DISCUSSION ===\n\
            Ce qui suit contient des IDÉES et PROPOSITIONS exprimées lors d'une discussion.\n\
            Extrais UNIQUEMENT le contenu substantiel — ignore le langage conversationnel.\n\
            ---\n{discussion_text}\n---\n\n\
            === DOCUMENT ACTUEL (.{format}) ===\n\
            {format_instruction}\n\
            ---\n{doc_display}\n---\n\n\
            === TA TÂCHE ===\n\
            Mets à jour le document ci-dessus en intégrant les idées pertinentes.\n\
            {mode_instruction}\n\
            Produis le document COMPLET mis à jour.\n\
            {lang_instruction}\n\n\
            RAPPEL : Rédige en prose impersonnelle et formelle. Pas de noms d'intervenants, pas de références à la discussion, pas de « je » ni « nous »."
        ),
    };

    (system, user)
}

/// Build read-only document context for synthesis prompt (IArbitre reads but doesn't modify).
pub fn build_document_context_readonly(
    content: &str,
    format: &str,
    lang: &str,
    mode: &DiscussionMode,
    ) -> String {
    let doc_display = if content.is_empty() {
        match lang {
            "en" => "[empty]",
            "zh" => "[空]",
            _ => "[vide]",
        }
    } else {
        content
    };

    let format_instruction = document_format_instruction(format, lang);
    let mode_instruction = mode_document_instruction(mode, lang);

    match lang {
        "en" => format!(
            "[SHARED DOCUMENT (.{format})]\n\
            {mode_instruction}\n\
            Format: .{format}. {format_instruction}\n\n\
            --- DOCUMENT ---\n\
            {doc_display}\n\
            --- END ---"
        ),
        "zh" => format!(
            "[共享文档 (.{format})]\n\
            {mode_instruction}\n\
            格式：.{format}。{format_instruction}\n\n\
            --- 文档 ---\n\
            {doc_display}\n\
            --- 结束 ---"
        ),
        _ => format!(
            "[DOCUMENT PARTAGÉ (.{format})]\n\
            {mode_instruction}\n\
            Format : .{format}. {format_instruction}\n\n\
            --- DOCUMENT ---\n\
            {doc_display}\n\
            --- FIN ---"
        ),
    }
}

/// Build mode-aware intervention instruction via compositional PE-optimized templates.
/// Dispatches to `mode_context_instruction` which composes mode-specific data into shared templates.
/// All 8 modes (including Debate) use the same architecture.
#[allow(clippy::too_many_arguments)]
fn build_mode_aware_instruction(
    mode: &DiscussionMode,
    lang: &str,
    user_name: &str,
    end_awareness: &str,
    is_opening: bool,
    current_turn: u32,
    user_has_spoken: bool,
    is_first_of_turn: bool,
    story_started: bool,
    focus: Option<&Focus>,
    ) -> String {
    let context = if *mode == DiscussionMode::CollaborativeFiction {
        // Relay writing: either write the opening or continue from the anchor —
        // the debate templates ("present YOUR OWN position") do not apply.
        if story_started {
            mode_prompts::InterventionContext::FictionContinue
        } else {
            mode_prompts::InterventionContext::FictionOpening
        }
    } else if is_opening {
        mode_prompts::InterventionContext::Opening
    } else if current_turn == 1 {
        mode_prompts::InterventionContext::Turn1
    } else if user_has_spoken {
        mode_prompts::InterventionContext::UserSpoke
    } else if is_first_of_turn {
        mode_prompts::InterventionContext::FirstOfTurn
    } else {
        mode_prompts::InterventionContext::General
    };
    // Focus only makes sense once there is something to respond to
    let focus = if current_turn >= 2 { focus } else { None };
    mode_prompts::mode_context_instruction(mode, lang, context, user_name, end_awareness, focus)
}

/// Build a prompt for extracting arguments and theses from recent exchanges.
pub fn build_argument_extraction_prompt(
    recent_context: &str,
    existing_arguments_context: &str,
    topic: &str,
    lang: &str,
    ) -> String {
    let lang_name = match lang {
        "en" => "English",
        "zh" => "Chinese",
        _ => "French",
    };

    match lang {
        "en" => format!(
            "Topic: {topic}\n\n\
            {existing_section}\
            Recent exchanges:\n{recent_context}\n\n\
            Analyze these exchanges and extract:\n\
            - New theses (main positions or claims) not already listed above\n\
            - Arguments supporting, countering, or providing evidence for theses or for existing arguments\n\
            Ignore speakers who only greet, moderate, or ask questions without taking a position.\n\
            Reuse existing thesis labels when a speaker refers to an already-identified thesis.\n\n\
            CRITICAL FORMATTING RULES:\n\
            - Write ALL labels in {lang_name}.\n\
            - Thesis labels: ONE concise claim capturing the position (max 15 words). Distill the essence, do not describe.\n\
            GOOD: \"AI creates more jobs than it destroys\"\n\
            BAD: \"Artificial intelligence, while transforming existing professions, has an overall positive impact on job creation because it generates new sectors of activity\"\n\
            - Argument text: ONE concise sentence capturing the key idea (max 25 words). Synthesize, do not quote.\n\
            GOOD: \"The tech sector created 3M jobs in 5 years, largely offsetting automated positions\"\n\
            BAD: \"As the AI Expert explained, data shows that the technology sector has experienced considerable growth with the creation of three million jobs over the past five years\"\n\
            - NEVER quote verbatim from the discussion. NEVER truncate mid-sentence. NEVER use identifiers like 'Thesis_X_Y'.\n\
            - NEVER use numbers, indices, or IDs to reference theses. Always use the FULL thesis label text.\n\
            - When an argument directly responds to a specific existing argument (not the thesis itself), set targets_argument to that argument's text. Otherwise set it to null.\n\n\
            Respond ONLY with JSON in this format:\n\
            {{\"extractions\": [\n\
            {{\"speaker\": \"Name\", \"new_theses\": [\"short thesis label\"], \"arguments\": [\n\
            {{\"text\": \"argument text\", \"type\": \"support|counter|evidence\",\n\
            \"for_thesis\": \"thesis label or null\", \"against_thesis\": \"thesis label or null\",\n\
            \"targets_argument\": \"text of the argument it responds to, or null\"}}\n\
            ]}}\n\
            ]}}",
            existing_section = if existing_arguments_context.is_empty() {
                String::new()
            } else {
                format!("Already identified theses and arguments:\n{existing_arguments_context}\n\n")
            },
    ),
    "zh" => format!(
        "主题: {topic}\n\n\
            {existing_section}\
            最近的对话:\n{recent_context}\n\n\
            分析这些对话并提取:\n\
            - 新论点（尚未在上面列出的主要立场或主张）\n\
            - 支持、反驳或为论点或现有论据提供证据的论据\n\
            忽略只打招呼、主持或提问而不表态的发言者。\n\
            当发言者提到已识别的论点时，重用现有的论点标签。\n\n\
            关键格式规则：\n\
            - 所有标签必须用中文。\n\
            - 论点标签：一句简洁的主张，提炼立场本质（最多15个词）。要提炼，不要描述。\n\
            好：\"人工智能创造的就业多于其消灭的\"\n\
            差：\"人工智能虽然正在改变现有职业，但由于它创造了新的活动领域，因此对就业创造总体上产生了积极影响\"\n\
            - 论据文本：一句简洁的句子捕捉核心观点（最多25个词）。要综合，不要引用。\n\
            好：\"科技行业5年内创造了300万个就业岗位，大幅抵消了自动化减少的职位\"\n\
            差：\"正如人工智能专家所解释的，数据显示科技行业在过去五年中经历了相当大的增长，创造了三百万个就业机会\"\n\
            - 绝不逐字引用讨论内容。绝不截断句子。绝不使用'Thesis_X_Y'等标识符。\n\
            - 绝不使用数字、索引或ID来引用论点。始终使用论点的完整标签文本。\n\
            - 当一个论据直接回应一个已存在的特定论据（而非论点本身）时，在targets_argument中填写该论据的文本。否则设为null。\n\n\
            仅用以下JSON格式回复:\n\
            {{\"extractions\": [\n\
            {{\"speaker\": \"姓名\", \"new_theses\": [\"简短论点标签\"], \"arguments\": [\n\
            {{\"text\": \"论据文本\", \"type\": \"support|counter|evidence\",\n\
            \"for_thesis\": \"论点标签或null\", \"against_thesis\": \"论点标签或null\",\n\
            \"targets_argument\": \"所回应的论据文本，或null\"}}\n\
            ]}}\n\
            ]}}",
            existing_section = if existing_arguments_context.is_empty() {
                String::new()
            } else {
                format!("已识别的论点和论据:\n{existing_arguments_context}\n\n")
            },
    ),
    _ => format!(
        "Sujet : {topic}\n\n\
            {existing_section}\
            Échanges récents :\n{recent_context}\n\n\
            Analyse ces échanges et extrais :\n\
            - Les nouvelles thèses (positions ou affirmations principales) non encore listées ci-dessus\n\
            - Les arguments soutenant, contrant ou apportant des preuves pour des thèses ou des arguments existants\n\
            Ignore les intervenants qui ne font que saluer, modérer ou poser des questions sans prendre position.\n\
            Réutilise les labels de thèses existantes quand un intervenant fait référence à une thèse déjà identifiée.\n\n\
            RÈGLES DE FORMAT CRITIQUES :\n\
            - Écris TOUS les labels en français.\n\
            - Labels de thèse : UNE proposition concise capturant la position (max 15 mots). Distille l'essence, ne décris pas.\n\
            BON : \"L'IA crée plus d'emplois qu'elle n'en détruit\"\n\
            MAUVAIS : \"L'intelligence artificielle, bien qu'elle transforme les métiers existants, a un impact globalement positif sur la création d'emplois car elle génère de nouveaux secteurs d'activité\"\n\
            - Texte d'argument : UNE phrase concise capturant l'idée-clé (max 25 mots). Synthétise, ne cite pas.\n\
            BON : \"Le secteur tech a créé 3M d'emplois en 5 ans, compensant largement les postes automatisés\"\n\
            MAUVAIS : \"Comme l'a expliqué l'Expert IA, les données montrent que le secteur technologique a connu une croissance considérable avec la création de trois millions d'emplois au cours des cinq dernières années\"\n\
            - JAMAIS de citation verbatim de la discussion. JAMAIS de phrase tronquée. JAMAIS d'identifiants comme 'Thesis_X_Y'.\n\
            - JAMAIS de numéro, indice ou ID pour référencer une thèse. Toujours utiliser le TEXTE COMPLET du label de la thèse.\n\
            - Quand un argument répond directement à un argument existant spécifique (pas la thèse elle-même), place dans targets_argument le texte de cet argument. Sinon met null.\n\n\
            Réponds UNIQUEMENT avec du JSON dans ce format :\n\
            {{\"extractions\": [\n\
            {{\"speaker\": \"Nom\", \"new_theses\": [\"label court de thèse\"], \"arguments\": [\n\
            {{\"text\": \"texte de l'argument\", \"type\": \"support|counter|evidence\",\n\
            \"for_thesis\": \"label thèse ou null\", \"against_thesis\": \"label thèse ou null\",\n\
            \"targets_argument\": \"texte de l'argument auquel il répond, ou null\"}}\n\
            ]}}\n\
            ]}}",
            existing_section = if existing_arguments_context.is_empty() {
                String::new()
            } else {
                format!("Thèses et arguments déjà identifiés :\n{existing_arguments_context}\n\n")
            },
    ),
}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(name: &str, role: SpeakerRole, content: &str) -> Message {
        Message {
            id: name.to_string(),
            discussion_id: "d".to_string(),
            turn_number: 2,
            speaker_id: name.to_lowercase(),
            speaker_name: name.to_string(),
            role,
            content: content.to_string(),
            inner_thought: None,
            thought_kind: Default::default(),
            reactions: vec![],
            is_ban_notification: false,
            kind: Default::default(),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_current_turn_block_puts_focus_last_and_shrinks_others() {
        let a = msg("Alpha", SpeakerRole::Gladiateur, "a");
        let u = msg("Léo", SpeakerRole::User, "u");
        let b = msg("Beta", SpeakerRole::Gladiateur, "b");
        let c = msg("Gamma", SpeakerRole::Gladiateur, "c");
        let refs = vec![&a, &u, &b, &c];

        let focus = Focus::Speaker("Beta".to_string());
        let ordered = order_current_turn_messages(&refs, Some(&focus), 1000);
        let names: Vec<&str> = ordered
            .iter()
            .map(|(m, _)| m.speaker_name.as_str())
            .collect();
        assert_eq!(names, vec!["Alpha", "Léo", "Gamma", "Beta"]);
        let budget = |n: &str| ordered.iter().find(|(m, _)| m.speaker_name == n).unwrap().1;
        assert_eq!(budget("Beta"), 1000);
        assert_eq!(budget("Léo"), 1000, "the user keeps the full budget");
        assert_eq!(
            budget("Alpha"),
            1000 / constants::FOCUS_OTHER_MESSAGE_DIVISOR
        );

        // Topic focus, no focus, or a focus who did not speak this turn: untouched
        for f in [
            None,
            Some(&Focus::Topic),
            Some(&Focus::Speaker("Delta".to_string())),
        ] {
            let same = order_current_turn_messages(&refs, f, 1000);
            let names: Vec<&str> = same.iter().map(|(m, _)| m.speaker_name.as_str()).collect();
            assert_eq!(names, vec!["Alpha", "Léo", "Beta", "Gamma"]);
            assert!(same.iter().all(|(_, n)| *n == 1000));
        }
    }

    fn open_loop(kind: OpenLoopKind, text: &str) -> OpenLoop {
        OpenLoop::new(kind, text, "Le Philosophe", 2)
    }

    #[test]
    fn intention_prompt_lists_targets_loops_and_the_json_contract() {
        let names = vec!["Le Philosophe".to_string(), "La Juriste".to_string()];
        let q = open_loop(OpenLoopKind::Question, "Et le coût social ?");
        let c = open_loop(OpenLoopKind::Commitment, "je citerai une étude");
        let loops = vec![&q, &c];
        let focus = Focus::Speaker("La Juriste".to_string());
        for (lang, target_word, header, priority) in [
            (
                "fr",
                "le mot sujet",
                "[Fils ouverts qui t'attendent]",
                "Priorité : adresse-toi à La Juriste.",
            ),
            (
                "en",
                "the word topic",
                "[Open loops waiting for you]",
                "Priority: address La Juriste.",
            ),
            (
                "zh",
                "一词",
                "[等待你处理的未决事项]",
                "优先：向La Juriste发言。",
            ),
        ] {
            let p = build_intention_prompt(
                "A: bonjour",
                &EmotionalProfile::default(),
                lang,
                true,
                false,
                2,
                Some(4),
                None,
                &DiscussionMode::Debate,
                &TokenBudget::default(),
                &names,
                Some(&focus),
                &loops,
                None,
            );
            assert!(p.contains(target_word), "{lang}: {p}");
            assert!(p.contains("Le Philosophe, La Juriste"), "{lang}");
            assert!(p.contains(priority), "{lang}: focus named as priority");
            assert!(p.contains(header), "{lang}");
            assert!(
                p.contains("1. ")
                    && p.contains("Et le coût social ?")
                    && p.contains("2. ")
                    && p.contains("je citerai une étude"),
                "{lang}"
            );
            assert!(
                p.contains("\"answers\"") && p.contains("\"thought\""),
                "{lang}"
            );
            for goal in IntentionGoal::ALL {
                assert!(p.contains(goal.label(lang)), "{lang}: {goal:?}");
            }
        }
        // No peers, no loops, first speaker: blocks absent, opening header
        let p = build_intention_prompt(
            "",
            &EmotionalProfile::default(),
            "fr",
            false,
            false,
            1,
            None,
            None,
            &DiscussionMode::Debate,
            &TokenBudget::default(),
            &[],
            None,
            &[],
            None,
        );
        assert!(!p.contains("[Participants que tu peux viser]") && !p.contains("[Fils ouverts"));
        assert!(p.contains("Tu es le premier à prendre la parole"));
        assert!(p.contains("[Ceci est ta préparation PRIVÉE"));
    }

    #[test]
    fn intention_block_is_bounded_and_open_loops_follow_the_budget() {
        let intention = Intention {
            target: Some("La Juriste".to_string()),
            goal: IntentionGoal::Contest,
            angle: "x".repeat(constants::INTENTION_ANGLE_MAX_CHARS),
            concession: Some("y".repeat(constants::INTENTION_FIELD_MAX_CHARS)),
            question: Some("z".repeat(constants::INTENTION_FIELD_MAX_CHARS)),
            answers: None,
            thought: "t".to_string(),
        };
        let block = build_intention_block(&intention, "fr");
        assert!(
            block.starts_with(
                "[Ton intention]\nCible : La Juriste — Objectif : contester — Angle : x"
            ),
            "{block}"
        );
        assert!(block.trim_end().ends_with("Tiens ce contrat : adresse-toi à ta cible en la nommant (pas forcément dès les premiers mots) et poursuis cet objectif."));
        assert!(
            block.len() <= constants::INTENTION_BLOCK_MAX_CHARS + 2,
            "{}",
            block.len()
        );
        let topic = Intention {
            target: None,
            ..Intention::default()
        };
        assert!(
            build_intention_block(&topic, "en").contains("Target : the topic — Goal : relaunch")
        );

        let q = open_loop(OpenLoopKind::Question, "Et le coût social ?");
        assert!(
            build_open_loops_block(&[&q], "fr", 0).is_empty(),
            "no budget, no block"
        );
        assert!(build_open_loops_block(&[], "fr", 900).is_empty());
        let block = build_open_loops_block(&[&q], "fr", 900);
        assert!(
            block.contains("[Fils ouverts — on attend une réponse de toi]")
                && block
                    .contains("1. Question de Le Philosophe (tour 2) : « Et le coût social ? »")
                && block.contains("tiens tes engagements"),
            "{block}"
        );
        let tight = build_open_loops_block(&[&q], "en", 12);
        assert!(
            tight.contains("1. Question ") && !tight.contains("social"),
            "{tight}"
        );
    }

    #[test]
    fn positions_carry_their_trajectory_in_prompts_and_synthesis() {
        let moved = ParticipantPosition {
            participant_name: "A".to_string(),
            stance: "ouvert".to_string(),
            initial_stance: Some("prudent".to_string()),
            shift: Some("s'est ouvert".to_string()),
            would_change_if: Some("des chiffres".to_string()),
        };
        let line = format_position_line("A", &moved, "fr", usize::MAX);
        assert_eq!(
            line,
            "- A : ouvert (a évolué : s'est ouvert) — changerait d'avis si : des chiffres"
        );
        assert_eq!(format_position_line("A", &moved, "fr", 12), "- A : ouvert");
        let still = ParticipantPosition {
            participant_name: "B".to_string(),
            stance: "critique".to_string(),
            initial_stance: Some("critique".to_string()),
            shift: None,
            would_change_if: None,
        };
        assert_eq!(
            format_position_line("B", &still, "en", usize::MAX),
            "- B : critique"
        );

        let mut memory = ParticipantMemory::default();
        memory.positional_map.insert("B".to_string(), still);
        assert!(
            build_positions_evolution_block(&memory, "fr").is_empty(),
            "nobody moved"
        );
        memory.positional_map.insert("A".to_string(), moved);
        let block = build_positions_evolution_block(&memory, "fr");
        assert!(
            block.contains("[Évolution des positions]\n- A : prudent → ouvert (s'est ouvert)\n")
                && block.contains("## Évolution des positions"),
            "{block}"
        );
        assert!(!block.contains("- B"));
        let synth = build_synthesis_prompt(
            "t",
            &memory,
            "en",
            &[],
            &DiscussionMode::Debate,
            None,
            None,
            &TokenBudget::default(),
            &[],
            None,
        );
        assert!(
            synth.contains("## Evolution of positions")
                && synth.contains("- A : ouvert (has evolved: s'est ouvert)"),
            "{synth}"
        );
    }

    #[test]
    fn turn_analyst_prompt_keeps_both_parts_and_asks_for_one_object() {
        for (lang, part1, merge) in [
            ("fr", "=== PARTIE 1 — MÉMOIRE ===", "FUSION"),
            ("en", "=== PART 1 — MEMORY ===", "MERGE"),
            ("zh", "=== 第一部分——记忆 ===", "合并"),
        ] {
            let p = build_turn_analyst_prompt("MEMOIRE-PROMPT", "EMOTION-PROMPT", lang);
            assert!(
                p.contains(part1)
                    && p.contains("MEMOIRE-PROMPT")
                    && p.contains("EMOTION-PROMPT")
                    && p.contains(merge),
                "{lang}: {p}"
            );
            assert!(
                p.contains("\"emotions\"") && p.contains(constants::EMOTION_STAGNATION_JSON_KEY),
                "{lang}"
            );
            assert!(p.find("MEMOIRE-PROMPT").unwrap() < p.find("EMOTION-PROMPT").unwrap());
        }
    }

    #[test]
    fn sources_block_lists_recent_unique_links_and_stays_bounded() {
        let src = |i: u32, kind: SourceKind| SourceRecord {
            kind,
            turn: i,
            speaker_name: format!("S{i}"),
            title: format!("Titre {i}"),
            url: format!("https://ex.org/{i}"),
        };
        assert!(build_sources_block(&[], "fr", 1500).is_empty());
        let mut list: Vec<SourceRecord> = (1..=30).map(|i| src(i, SourceKind::Web)).collect();
        list.push(src(30, SourceKind::Wiki)); // same url as the last one → deduplicated
        let block = build_sources_block(&list, "fr", 1500);
        assert!(block.contains("[Sources utilisées pendant la discussion]"));
        assert!(block.contains("## Sources"));
        assert!(
            block.contains("https://ex.org/30") && !block.contains("https://ex.org/5 "),
            "most recent entries kept"
        );
        assert_eq!(
            block.matches("https://ex.org/30").count(),
            1,
            "deduplicated by url"
        );
        assert!(block.len() <= 1500 + 300, "bounded (instruction excluded)");
        let tiny = build_sources_block(&list, "en", 80);
        assert!(
            tiny.contains("## Sources") && !tiny.contains("https://"),
            "no room for links → instruction only"
        );
        assert!(build_sources_block(&list, "zh", 0).is_empty());
    }

    #[test]
    fn test_summarize_emotional_state_neutral() {
        let emo = EmotionalProfile::default();
        let result = summarize_emotional_state(&emo, "fr");
        // Default emotions: engagement=50, accord=50, confiance=50, frustration=10, curiosite=50, enthousiasme=50
        // frustration=10 is far from 50, so it should pick up a calm/serene variant
        let has_calm =
            result.contains("serein") || result.contains("détendu") || result.contains("posé");
        assert!(has_calm, "Expected a calm phrase variant, got: {result}");
        assert!(
            result.ends_with('.'),
            "Expected sentence ending with '.', got: {result}"
        );
    }

    #[test]
    fn test_summarize_emotional_state_frustrated() {
        let emo = EmotionalProfile {
            frustration: 90,
            ..Default::default()
        };
        let result = summarize_emotional_state(&emo, "en");
        let has_frustrated =
            result.contains("tense") || result.contains("frustrated") || result.contains("edge");
        assert!(
            has_frustrated,
            "Expected a frustrated phrase variant, got: {result}"
        );
        assert!(
            result.ends_with('.'),
            "Expected sentence ending with '.', got: {result}"
        );
    }

    #[test]
    fn test_summarize_emotional_state_multiple() {
        let emo = EmotionalProfile {
            frustration: 90,
            engagement: 10,
            ..Default::default()
        };
        let result = summarize_emotional_state(&emo, "fr");
        let has_frustrated = result.contains("tendu")
            || result.contains("irrité")
            || result.contains("exaspération");
        let has_detached =
            result.contains("détaché") || result.contains("retrait") || result.contains("distant");
        assert!(
            has_frustrated,
            "Expected a frustrated phrase variant, got: {result}"
        );
        assert!(
            has_detached,
            "Expected a detached phrase variant, got: {result}"
        );
        assert!(
            result.ends_with('.'),
            "Expected sentence ending with '.', got: {result}"
        );
    }

    #[test]
    fn test_summarize_emotional_state_variety() {
        // Different emotion values should produce different starters/phrases
        let emo1 = EmotionalProfile {
            frustration: 80,
            engagement: 75,
            ..Default::default()
        };
        let emo2 = EmotionalProfile {
            frustration: 80,
            engagement: 80,
            ..Default::default()
        };
        let r1 = summarize_emotional_state(&emo1, "fr");
        let r2 = summarize_emotional_state(&emo2, "fr");
        // Both should be valid sentences but may differ
        assert!(r1.ends_with('.'));
        assert!(r2.ends_with('.'));
    }

    #[test]
    fn test_parse_ocean_values() {
        let prompt = "<psychology>\nOCEAN: O=8 C=9 E=4 A=4 N=3\nPosture: ADULTE\n</psychology>";
        let values = parse_ocean_values(prompt);
        assert_eq!(values, Some([8, 9, 4, 4, 3]));
    }

    #[test]
    fn test_parse_ocean_values_no_ocean() {
        let prompt = "No OCEAN values here";
        assert!(parse_ocean_values(prompt).is_none());
    }

    #[test]
    fn test_build_ocean_directives_extreme() {
        let prompt = "<psychology>\nOCEAN: O=9 C=2 E=9 A=2 N=9\n</psychology>";
        let directives = build_ocean_directives(prompt, "en");
        assert!(
            directives.contains("Openness"),
            "Missing Openness directive: {directives}"
        );
        assert!(
            directives.contains("Conscientiousness"),
            "Missing Conscientiousness directive: {directives}"
        );
        assert!(
            directives.contains("Extraversion"),
            "Missing Extraversion directive: {directives}"
        );
        assert!(
            directives.contains("Agreeableness"),
            "Missing Agreeableness directive: {directives}"
        );
        assert!(
            directives.contains("Neuroticism"),
            "Missing Neuroticism directive: {directives}"
        );
    }

    #[test]
    fn test_build_ocean_directives_neutral() {
        let prompt = "<psychology>\nOCEAN: O=5 C=5 E=5 A=5 N=5\n</psychology>";
        let directives = build_ocean_directives(prompt, "en");
        assert!(
            directives.is_empty(),
            "Expected empty for neutral OCEAN, got: {directives}"
        );
    }

    #[test]
    fn test_build_datetime_context_fr() {
        let result = build_datetime_context("fr");
        assert!(result.starts_with("[Date et heure actuelles] "));
        // Must use +XX:XX format, NOT timezone name like "Romance Standard Time"
        assert!(
            result.contains('+') || result.contains('-'),
            "Expected timezone offset (+/-) in: {result}"
        );
        // Should NOT contain alphabetic timezone names
        assert!(
            !result.contains("Standard") && !result.contains("Daylight"),
            "Should use %:z not %Z: {result}"
        );
    }

    #[test]
    fn test_build_datetime_context_en() {
        let result = build_datetime_context("en");
        assert!(result.starts_with("[Current date and time] "));
    }

    #[test]
    fn test_build_datetime_context_zh() {
        let result = build_datetime_context("zh");
        assert!(result.starts_with("[当前日期和时间] "));
    }

    #[test]
    fn test_default_search_directive_all_languages() {
        let fr = default_search_directive("fr");
        assert!(fr.contains("recherche internet"));

        let en = default_search_directive("en");
        assert!(en.contains("internet search"));

        let zh = default_search_directive("zh");
        assert!(zh.contains("互联网搜索"));

        // Unknown language falls back to French
        let other = default_search_directive("de");
        assert_eq!(other, fr);
    }

    #[test]
    fn test_build_search_results_context_basic() {
        let results = vec![(
            "test query".to_string(),
            TavilySearchResponse {
                answer: Some("A test answer".to_string()),
                results: vec![crate::tavily::TavilyResult {
                    title: "Title".to_string(),
                    url: "https://example.com/page".to_string(),
                    content: "Some content".to_string(),
                    score: 0.9,
                }],
            },
        )];
        let ctx = build_search_results_context(&results, "fr");
        assert!(ctx.contains("[Résultats internet"));
        assert!(ctx.contains("test query"));
        assert!(ctx.contains("A test answer"));
        assert!(ctx.contains("example.com"));
    }

    #[test]
    fn test_build_search_results_context_truncation() {
        // Create results with lots of content to trigger constants::SEARCH_MAX_CONTEXT_LEN limit
        let long_content = "x".repeat(500);
        let results: Vec<(String, TavilySearchResponse)> = (0..10)
            .map(|i| {
                (
                    format!("query {i}"),
                    TavilySearchResponse {
                        answer: Some(long_content.clone()),
                        results: vec![crate::tavily::TavilyResult {
                            title: format!("Title {i}"),
                            url: format!("https://example{i}.com/page"),
                            content: long_content.clone(),
                            score: 0.5,
                        }],
                    },
                )
            })
            .collect();
        let ctx = build_search_results_context(&results, "en");
        assert!(
            ctx.len() <= constants::SEARCH_MAX_CONTEXT_LEN,
            "Output should be truncated to {} chars, got {}",
            constants::SEARCH_MAX_CONTEXT_LEN,
            ctx.len()
        );
    }

    #[test]
    fn test_build_search_results_context_empty() {
        let results: Vec<(String, TavilySearchResponse)> = vec![];
        let ctx = build_search_results_context(&results, "en");
        assert!(ctx.contains("[Internet results"));
    }

    // ── Wikipedia prompt tests ──

    #[test]
    fn test_default_wiki_directive_all_languages() {
        let fr = default_wiki_directive("fr");
        assert!(fr.contains("Wikipédia"));

        let en = default_wiki_directive("en");
        assert!(en.contains("Wikipedia"));

        let zh = default_wiki_directive("zh");
        assert!(zh.contains("维基百科"));

        let other = default_wiki_directive("de");
        assert_eq!(other, fr);
    }

    #[test]
    fn test_build_wiki_results_context_basic() {
        use crate::wikipedia::{WikiPage, WikiQuery, WikiSearchResponse};
        let results = vec![(
            "intelligence artificielle".to_string(),
            WikiSearchResponse {
                query: Some(WikiQuery {
                    pages: vec![
                        WikiPage {
                            title: "Intelligence artificielle".to_string(),
                            pageid: 1,
                            index: 1,
                            extract:
                                "L'intelligence artificielle est un domaine de l'informatique."
                                    .to_string(),
                        },
                        WikiPage {
                            title: "Apprentissage automatique".to_string(),
                            pageid: 2,
                            index: 2,
                            extract: "L'apprentissage automatique est une branche de l'IA."
                                .to_string(),
                        },
                    ],
                }),
            },
        )];
        let ctx = build_wiki_results_context(&results, "fr");
        assert!(ctx.contains("[Résultats Wikipédia"));
        assert!(ctx.contains("Intelligence artificielle"));
        assert!(ctx.contains("Apprentissage automatique"));
    }

    #[test]
    fn test_build_wiki_results_context_sorts_by_index() {
        use crate::wikipedia::{WikiPage, WikiQuery, WikiSearchResponse};
        let results = vec![(
            "test".to_string(),
            WikiSearchResponse {
                query: Some(WikiQuery {
                    pages: vec![
                        WikiPage {
                            title: "B".to_string(),
                            pageid: 2,
                            index: 3,
                            extract: "Second".to_string(),
                        },
                        WikiPage {
                            title: "A".to_string(),
                            pageid: 1,
                            index: 1,
                            extract: "First".to_string(),
                        },
                    ],
                }),
            },
        )];
        let ctx = build_wiki_results_context(&results, "en");
        let pos_a = ctx.find("\"A\"").unwrap();
        let pos_b = ctx.find("\"B\"").unwrap();
        assert!(pos_a < pos_b, "A (index=1) should come before B (index=3)");
    }

    #[test]
    fn test_build_wiki_results_context_truncation() {
        use crate::wikipedia::{WikiPage, WikiQuery, WikiSearchResponse};
        let long_extract = "x".repeat(500);
        let results: Vec<(String, WikiSearchResponse)> = (0..10)
            .map(|i| {
                (
                    format!("query {i}"),
                    WikiSearchResponse {
                        query: Some(WikiQuery {
                            pages: vec![WikiPage {
                                title: format!("Title {i}"),
                                pageid: i,
                                index: 1,
                                extract: long_extract.clone(),
                            }],
                        }),
                    },
                )
            })
            .collect();
        let ctx = build_wiki_results_context(&results, "fr");
        assert!(
            ctx.len() <= constants::SEARCH_MAX_CONTEXT_LEN,
            "Output should be truncated to {} chars, got {}",
            constants::SEARCH_MAX_CONTEXT_LEN,
            ctx.len()
        );
    }

    #[test]
    fn test_build_reaction_prompt_language_instruction() {
        let interventions = vec![
            ("Alice".to_string(), "Some argument".to_string()),
            ("Bob".to_string(), "Counter argument".to_string()),
        ];
        let en = build_reaction_prompt(
            &interventions,
            "en",
            &DiscussionMode::Debate,
            ReactionScope::PreviousTurn,
            None,
            true,
        );
        assert!(
            en.contains("English"),
            "EN prompt should contain 'English': {en}"
        );
        assert!(en.contains("previous turn") && en.contains("\"quote\""));

        let fr = build_reaction_prompt(
            &interventions,
            "fr",
            &DiscussionMode::Debate,
            ReactionScope::LastIntervention,
            None,
            true,
        );
        assert!(
            fr.contains("français"),
            "FR prompt should contain 'français': {fr}"
        );
        assert!(fr.contains("vient de se terminer") && !fr.contains("tour précédent"));

        let zh = build_reaction_prompt(
            &interventions,
            "zh",
            &DiscussionMode::Debate,
            ReactionScope::PreviousTurn,
            None,
            true,
        );
        assert!(zh.contains("中文"), "ZH prompt should contain '中文': {zh}");
    }

    #[test]
    fn test_build_reaction_prompt_mode_meanings_vocabulary_and_propensity() {
        let interventions = vec![("Alice".to_string(), "Some idea".to_string())];
        // Ideation should use brainstorming vocabulary, not debate vocabulary
        let ideation = build_reaction_prompt(
            &interventions,
            "en",
            &DiscussionMode::Ideation,
            ReactionScope::PreviousTurn,
            None,
            true,
        );
        assert!(
            ideation.contains("promising idea"),
            "Ideation should contain 'promising idea', got: {ideation}"
        );
        assert!(
            !ideation.contains("agree or"),
            "Ideation should NOT contain debate vocab 'agree or', got: {ideation}"
        );

        // Debate should use debate vocabulary and list every colour
        let debate = build_reaction_prompt(
            &interventions,
            "en",
            &DiscussionMode::Debate,
            ReactionScope::PreviousTurn,
            None,
            true,
        );
        assert!(
            debate.contains("agree or"),
            "Debate should contain 'agree or', got: {debate}"
        );
        for kind in ["insightful", "question", "offTopic", "laugh", "none"] {
            assert!(
                debate.contains(&format!("\"{kind}\"")),
                "{kind} missing: {debate}"
            );
        }
        // Fiction: no disagreement colours
        let fiction = build_reaction_prompt(
            &interventions,
            "fr",
            &DiscussionMode::CollaborativeFiction,
            ReactionScope::LastIntervention,
            None,
            true,
        );
        assert!(
            fiction.contains("\"laugh\"")
                && !fiction.contains("\"dislike\"")
                && !fiction.contains("\"offTopic\"")
        );
        // Propensity line
        let shy = ReactionPropensity::from_ocean(Some([5, 5, 2, 9, 5])).unwrap();
        let with = build_reaction_prompt(
            &interventions,
            "fr",
            &DiscussionMode::Debate,
            ReactionScope::LastIntervention,
            Some(&shy),
            true,
        );
        assert!(with.contains("Tu réagis rarement") && with.contains("indulgent"));
    }

    /// v1.20.4 — sincere reactions: the example never anchors on "insightful"
    /// and rotates with the content, the sincerity rules are stated in the
    /// three languages, and fiction (no dislike) says disagreement with a question.
    #[test]
    fn reaction_prompt_example_rotates_and_sincerity_rules_are_stated() {
        let two = |content: &str| vec![("Alice".to_string(), content.to_string()), ("Bob".to_string(), "b".to_string())];
        let fr = build_reaction_prompt(&two("Some argument"), "fr", &DiscussionMode::Debate, ReactionScope::PreviousTurn, None, true);
        assert!(fr.contains("Règles de sincérité") && fr.contains("\"insightful\" se mérite") && fr.contains("\"dislike\" ou \"question\", jamais un compliment nuancé"), "{fr}");
        // v1.20.5 — quantitative anchors: models calibrate on numbers, not on "rare"
        assert!(fr.contains("environ une intervention sur deux") && fr.contains("au plus une intervention sur cinq"), "{fr}");
        assert!(fr.contains("Format attendu : [{\"speaker\":\"Alice\",\"reacts\":false,\"reaction\":\"none\""), "not reacting is the first example: {fr}");
        assert!(fr.contains("Décide d'abord si l'intervention mérite une réaction") && !fr.contains("crédit \"insightful\" est épuisé"), "{fr}");
        let spent = build_reaction_prompt(&two("a"), "fr", &DiscussionMode::Debate, ReactionScope::LastIntervention, None, false);
        assert!(spent.contains("ton crédit \"insightful\" est épuisé pour l'instant"), "{spent}");
        assert!(build_reaction_prompt(&two("a"), "en", &DiscussionMode::Debate, ReactionScope::LastIntervention, None, false).contains("credit is spent"));
        let example_kinds: std::collections::HashSet<String> = (0..12)
            .map(|n| build_reaction_prompt(&two(&"x".repeat(n)), "fr", &DiscussionMode::Debate, ReactionScope::PreviousTurn, None, true))
            .map(|p| p.split("\"speaker\":\"Bob\",\"reacts\":true,\"reaction\":\"").nth(1).unwrap().split('"').next().unwrap().to_string())
            .collect();
        assert!(!example_kinds.contains("insightful"), "{example_kinds:?}");
        assert!(example_kinds.len() >= 4, "rotates with the content: {example_kinds:?}");
        let en = build_reaction_prompt(&two("a"), "en", &DiscussionMode::Debate, ReactionScope::LastIntervention, None, true);
        assert!(en.contains("Sincerity rules") && en.contains("must be earned") && en.contains("first person"), "{en}");
        let zh = build_reaction_prompt(&two("a"), "zh", &DiscussionMode::Debate, ReactionScope::LastIntervention, None, true);
        assert!(zh.contains("真诚规则") && zh.contains("靠实力赢得"), "{zh}");
        // Fiction: disagreement is a question, and the example never shows a forbidden colour
        for n in 0..8 {
            let fiction = build_reaction_prompt(&two(&"y".repeat(n)), "fr", &DiscussionMode::CollaborativeFiction, ReactionScope::LastIntervention, None, true);
            assert!(fiction.contains("dis-le : \"question\", jamais") && !fiction.contains("\"dislike\"") && !fiction.contains("\"offTopic\""), "{fiction}");
        }
        // The insightful meaning itself says it is rare
        assert!(fr.contains("- \"insightful\": un point fort qui change ta façon de voir — rare"), "{fr}");
        // A single intervention (immediate rounds): "none" is the example every other time, never "insightful"
        let one = |content: &str| vec![("Alice".to_string(), content.to_string())];
        let singles: Vec<String> = (0..12).map(|n| build_reaction_prompt(&one(&"z".repeat(n)), "fr", &DiscussionMode::Debate, ReactionScope::LastIntervention, None, true)).collect();
        assert_eq!(singles.iter().filter(|p| p.contains("\"reacts\":false,\"reaction\":\"none\",\"justification\":\"\"")).count(), 6, "every other example says none");
        assert!(singles.iter().all(|p| !p.contains("\"reaction\":\"insightful\"")));
        assert!(fr.contains("choisis UNE réaction — ou \"none\""));
    }

    #[test]
    fn test_current_turn_reactions_shown_under_messages() {
        use crate::models::message::Reaction;
        let mut a = msg("Alpha", SpeakerRole::Gladiateur, "a");
        let react = |name: &str, kind: ReactionType, j: Option<&str>| Reaction {
            from_speaker_id: name.to_lowercase(),
            from_speaker_name: name.into(),
            reaction_type: kind,
            target_message_id: "Alpha".into(),
            justification: j.map(String::from),
            quote: None,
        };
        a.reactions = vec![
            react("Beta", ReactionType::Insightful, Some("solide")),
            react("Gamma", ReactionType::Dislike, None),
            react("Delta", ReactionType::Laugh, Some("x")),
            react("Léo", ReactionType::Like, None),
        ];
        let line = format_message_reactions(&a, "fr");
        assert!(
            line.starts_with(
                "   [réactions: 💡 Beta (« solide »), 👎 Gamma, 😂 Delta (« x »), +1]"
            ),
            "{line}"
        );
        assert!(format_message_reactions(&msg("B", SpeakerRole::Gladiateur, "b"), "fr").is_empty());
    }

    // ── Document update prompt tests ──

    #[test]
    fn test_build_document_update_prompt_anti_contamination() {
        // Verify PE anti-contamination rules are present in system prompts for all languages
        let (sys_en, _) = build_document_update_prompt(
            "",
            "md",
            "some discussion",
            &DiscussionMode::CoConstruction,
            "en",
            "Test topic",
        );
        assert!(
            sys_en.contains("impersonal"),
            "EN system should contain 'impersonal': {sys_en}"
        );
        assert!(
            sys_en.contains("NEVER"),
            "EN system should contain 'NEVER': {sys_en}"
        );

        let (sys_fr, _) = build_document_update_prompt(
            "",
            "md",
            "discussion",
            &DiscussionMode::CoConstruction,
            "fr",
            "Sujet test",
        );
        assert!(
            sys_fr.contains("impersonnelle"),
            "FR system should contain 'impersonnelle': {sys_fr}"
        );
        assert!(
            sys_fr.contains("JAMAIS"),
            "FR system should contain 'JAMAIS': {sys_fr}"
        );

        let (sys_zh, _) = build_document_update_prompt(
            "",
            "md",
            "讨论",
            &DiscussionMode::CoConstruction,
            "zh",
            "测试主题",
        );
        assert!(
            sys_zh.contains("非人称"),
            "ZH system should contain '非人称': {sys_zh}"
        );
        assert!(
            sys_zh.contains("绝不"),
            "ZH system should contain '绝不': {sys_zh}"
        );
    }

    #[test]
    fn test_build_document_update_prompt_context_isolation() {
        // Verify PE context isolation (IDEAS framing) and REMEMBER repetition in user prompts
        let (_, usr_en) = build_document_update_prompt(
            "existing content",
            "md",
            "Alice said hello",
            &DiscussionMode::CoConstruction,
            "en",
            "Test",
        );
        assert!(
            usr_en.contains("IDEAS"),
            "EN user should contain 'IDEAS': {usr_en}"
        );
        assert!(
            usr_en.contains("REMEMBER"),
            "EN user should contain 'REMEMBER': {usr_en}"
        );
        assert!(
            usr_en.contains("Test"),
            "EN user should contain topic 'Test': {usr_en}"
        );

        let (_, usr_fr) = build_document_update_prompt(
            "contenu",
            "md",
            "Alice a dit bonjour",
            &DiscussionMode::CoConstruction,
            "fr",
            "Sujet",
        );
        assert!(
            usr_fr.contains("IDÉES"),
            "FR user should contain 'IDÉES': {usr_fr}"
        );
        assert!(
            usr_fr.contains("RAPPEL"),
            "FR user should contain 'RAPPEL': {usr_fr}"
        );

        let (_, usr_zh) = build_document_update_prompt(
            "内容",
            "md",
            "讨论内容",
            &DiscussionMode::CoConstruction,
            "zh",
            "主题",
        );
        assert!(
            usr_zh.contains("想法"),
            "ZH user should contain '想法': {usr_zh}"
        );
        assert!(
            usr_zh.contains("记住"),
            "ZH user should contain '记住': {usr_zh}"
        );
    }

    #[test]
    fn test_build_full_document_block_none() {
        let result = build_full_document_block(None, 5000, "fr");
        assert!(result.is_empty());
    }

    #[test]
    fn test_build_full_document_block_empty_string() {
        let result = build_full_document_block(Some(""), 5000, "fr");
        assert!(result.is_empty());
    }

    #[test]
    fn test_build_full_document_block_french() {
        let result = build_full_document_block(Some("Contenu du document"), 5000, "fr");
        assert!(result.contains("DOCUMENT DE RÉFÉRENCE"));
        assert!(result.contains("--- DOCUMENT ---"));
        assert!(result.contains("Contenu du document"));
        assert!(result.contains("--- FIN ---"));
    }

    #[test]
    fn test_build_full_document_block_english() {
        let result = build_full_document_block(Some("Document content"), 5000, "en");
        assert!(result.contains("REFERENCE DOCUMENT"));
        assert!(result.contains("Document content"));
    }

    #[test]
    fn test_build_full_document_block_chinese() {
        let result = build_full_document_block(Some("文档内容"), 5000, "zh");
        assert!(result.contains("参考文档"));
        assert!(result.contains("文档内容"));
    }

    // ── Hidden agendas and casting (v1.19) ──────────────────────────────

    fn max_agenda() -> Agenda {
        let field =
            |c: char| std::iter::repeat_n(c, constants::AGENDA_FIELD_MAX_CHARS).collect::<String>();
        Agenda {
            objective: field('o'),
            red_line: field('r'),
            victory: field('v'),
        }
    }

    #[test]
    fn agenda_block_fits_the_reserved_overhead_and_keeps_its_rule_in_every_language() {
        let bound = constants::AGENDA_MAX_CHARS + constants::AGENDA_BLOCK_OVERHEAD_CHARS;
        for mode in [DiscussionMode::Debate, DiscussionMode::CollaborativeFiction] {
            for lang in ["fr", "en", "zh"] {
                let block = build_agenda_block(&max_agenda(), &mode, lang);
                assert!(
                    block.chars().count() <= bound,
                    "{lang} {mode:?}: {} chars",
                    block.chars().count()
                );
                // Nothing was cut: the closing rule survived, and the header names the secret
                let last = block.lines().last().unwrap();
                assert!(last.len() > 30, "{lang} {mode:?}: rule cut → {last}");
                assert!(block.starts_with('['));
            }
        }
        // Empty fields are skipped, the author's wording is used in fiction
        let partial = Agenda {
            objective: "faire admettre le coût".into(),
            red_line: String::new(),
            victory: String::new(),
        };
        let block = build_agenda_block(&partial, &DiscussionMode::Debate, "fr");
        assert!(
            block.contains("Objectif : faire admettre le coût") && !block.contains("Ligne rouge")
        );
        assert!(
            build_agenda_block(&partial, &DiscussionMode::CollaborativeFiction, "fr")
                .contains("agenda d'auteur")
        );
    }

    #[test]
    fn intervention_system_prompt_carries_the_agenda_after_the_persona_only_when_given() {
        let memory = ParticipantMemory::default();
        let build = |agenda: Option<&Agenda>| {
            build_intervention_prompt(
                "<persona>Le Scientifique</persona>",
                "t",
                &memory,
                &[],
                None,
                &EmotionalProfile::default(),
                "fr",
                "Léo",
                false,
                2,
                Some(4),
                None,
                &["Le Philosophe".to_string()],
                None,
                &DiscussionMode::Debate,
                None,
                &TokenBudget::default(),
                None,
                None,
                &[],
                None,
                agenda,
                None,
            )
        };
        let agenda = Agenda {
            objective: "faire admettre le coût".into(),
            red_line: "jamais le remplacement total".into(),
            victory: "le Philosophe cite mes chiffres".into(),
        };
        let (system, user) = build(Some(&agenda));
        let persona = system.find("</persona>").unwrap();
        let block = system
            .find("[Ton agenda secret — ne le révèle jamais explicitement]")
            .unwrap();
        let preamble = system.find("Tu es un participant").unwrap();
        assert!(
            persona < block && block < preamble,
            "agenda must sit between the persona and the rules"
        );
        assert!(system.contains("Ligne rouge : jamais le remplacement total"));
        assert!(
            !user.contains("agenda secret"),
            "the agenda never leaves the system prompt"
        );
        let (system, _) = build(None);
        assert!(!system.contains("agenda"));
    }

    #[test]
    fn intention_prompt_reminds_the_objective_when_an_agenda_exists() {
        let agenda = Agenda {
            objective: "faire admettre le coût".into(),
            red_line: String::new(),
            victory: String::new(),
        };
        let with = build_intention_prompt(
            "A: b",
            &EmotionalProfile::default(),
            "en",
            true,
            false,
            2,
            None,
            None,
            &DiscussionMode::Debate,
            &TokenBudget::default(),
            &[],
            None,
            &[],
            Some(&agenda),
        );
        assert!(with.contains("[Your secret agenda] faire admettre le coût"));
        let without = build_intention_prompt(
            "A: b",
            &EmotionalProfile::default(),
            "en",
            true,
            false,
            2,
            None,
            None,
            &DiscussionMode::Debate,
            &TokenBudget::default(),
            &[],
            None,
            &[],
            None,
        );
        assert!(!without.contains("secret agenda"));
    }

    #[test]
    fn agenda_prompt_asks_for_json_and_takes_the_author_voice_in_fiction() {
        let others = vec!["Le Philosophe".to_string()];
        for lang in ["fr", "en", "zh"] {
            let debate = build_agenda_prompt("sujet", &others, &DiscussionMode::Debate, lang);
            assert!(
                debate.contains("Le Philosophe")
                    && debate.contains("JSON")
                    && debate.contains(&constants::AGENDA_FIELD_MAX_CHARS.to_string()),
                "{lang}"
            );
            let fiction =
                build_agenda_prompt("sujet", &[], &DiscussionMode::CollaborativeFiction, lang);
            assert_ne!(debate, fiction, "{lang}: the story gets an author's agenda");
        }
        assert!(
            build_agenda_prompt("sujet", &[], &DiscussionMode::CollaborativeFiction, "fr")
                .contains("agenda d'auteur")
        );
        assert!(
            !build_agenda_prompt("sujet", &[], &DiscussionMode::Debate, "fr")
                .contains("Autres participants")
        );
    }

    #[test]
    fn synthesis_prompt_lists_the_agendas_and_asks_for_their_verdict() {
        let memory = ParticipantMemory::default();
        let reveal = |name: &str| AgendaReveal {
            speaker_id: name.to_lowercase(),
            speaker_name: name.to_string(),
            agenda: Agenda {
                objective: format!("objectif de {name}"),
                red_line: "ligne".into(),
                victory: String::new(),
            },
            achieved: None,
        };
        let agendas = vec![reveal("Alpha"), reveal("Beta")];
        let synth = build_synthesis_prompt(
            "t",
            &memory,
            "fr",
            &[],
            &DiscussionMode::Debate,
            None,
            None,
            &TokenBudget::default(),
            &agendas,
            None,
        );
        assert!(synth.contains("[Agendas secrets"));
        assert!(synth.contains("- Alpha : objectif « objectif de Alpha » ; ligne rouge « ligne »"));
        assert!(!synth.contains("victoire « »"), "empty fields are skipped");
        assert!(synth.contains("## Agendas"));
        let none = build_synthesis_prompt(
            "t",
            &memory,
            "fr",
            &[],
            &DiscussionMode::Debate,
            None,
            None,
            &TokenBudget::default(),
            &[],
            None,
        );
        assert!(!none.contains("Agendas"));
    }

    #[test]
    fn outcome_blocks_report_verdicts_agreements_and_votes_in_three_languages() {
        use crate::models::outcome::{PartyDecision, VerdictVote};
        let verdict = ModeOutcome::Verdict {
            votes: vec![VerdictVote {
                voter_id: "j1".into(),
                voter_name: "La Juriste".into(),
                choice: VERDICT_PROSECUTION.into(),
                reason: "les preuves".into(),
            }],
            winner: Some(VERDICT_PROSECUTION.into()),
            by_arbitre: false,
        };
        let fr = build_outcome_synthesis_block(Some(&verdict), "fr");
        assert!(
            fr.contains("[Verdict]")
                && fr.contains("- La Juriste : l'accusation — les preuves")
                && fr.contains("Verdict rendu : l'accusation.")
                && fr.contains("## Verdict")
        );
        let hung = ModeOutcome::Verdict {
            votes: vec![],
            winner: None,
            by_arbitre: true,
        };
        assert!(
            build_outcome_synthesis_block(Some(&hung), "en").contains("returned by the moderator")
                && build_outcome_synthesis_block(Some(&hung), "en").contains("Hung jury")
        );
        let deal = ModeOutcome::Agreement {
            parties: vec![PartyDecision {
                party_id: "p".into(),
                party_name: "Alpha".into(),
                accepts: false,
                reason: "trop cher".into(),
            }],
            reached: false,
        };
        let zh = build_outcome_synthesis_block(Some(&deal), "zh");
        assert!(
            zh.contains("[协议]")
                && zh.contains("- Alpha : 拒绝 — trop cher")
                && zh.contains("未达成协议")
        );
        let swing = ModeOutcome::AudienceSwing {
            before: Some(VOTE_FOR.into()),
            after: Some(VOTE_AGAINST.into()),
            winner: Some(VOTE_AGAINST.into()),
        };
        let en = build_outcome_synthesis_block(Some(&swing), "en");
        assert!(en.contains("Before the debate: for the motion. After: against the motion. Winner by displacement: against the motion."));
        let no_move = ModeOutcome::AudienceSwing {
            before: None,
            after: None,
            winner: None,
        };
        assert!(build_outcome_synthesis_block(Some(&no_move), "fr")
            .contains("Avant le débat : pas de vote. Après : pas de vote. Aucun déplacement"));
        assert!(build_outcome_synthesis_block(None, "fr").is_empty());
        // Prompts of the calls
        let memory = ParticipantMemory::default();
        for lang in ["fr", "en", "zh"] {
            assert!(build_verdict_prompt("t", &memory, lang, false).contains("JSON"));
            assert_ne!(
                build_verdict_prompt("t", &memory, lang, false),
                build_verdict_prompt("t", &memory, lang, true)
            );
            assert!(build_agreement_prompt("t", &memory, lang).contains("accepts"));
            let (system, user) = build_dispatches_prompt("t", 4, lang);
            assert!(system.contains("JSON") && user.contains("dispatches") && user.contains('4'));
        }
    }

    #[test]
    fn memories_block_stays_within_its_reserve_and_recap_prompt_lists_the_context() {
        use crate::models::persona_memory::PersonaRecap;
        let memory = |topic: &str| PersonaMemory {
            id: "m".into(),
            profile_id: "scientist".into(),
            discussion_id: "d".into(),
            topic: topic.into(),
            created_at: "2026-09-16T10:00:00Z".into(),
            recap: PersonaRecap {
                positions: vec!["p".repeat(200)],
                best_lines: vec![],
                allies: vec!["La Juriste".into()],
                rivals: vec![],
                lesson: "l".repeat(200),
            },
        };
        for lang in ["fr", "en", "zh"] {
            let block = build_memories_block(&[memory("A"), memory("B"), memory("C")], lang);
            assert!(
                block.chars().count() <= constants::PERSONA_MEMORY_MAX_CHARS,
                "{lang}: {}",
                block.chars().count()
            );
            assert!(
                block.starts_with('[') && block.contains("« A » (2026-09-16)"),
                "{block}"
            );
            assert_eq!(block.lines().count(), 5, "header + 3 memories + footer");
        }
        assert!(build_memories_block(&[], "fr").is_empty());
        let own = vec!["Les données montrent une transformation.".to_string()];
        let allies = vec!["La Juriste".to_string()];
        let input = RecapInput {
            speaker_name: "Le Scientifique",
            topic: "Sujet",
            summary: "Résumé",
            own_messages: &own,
            allies: &allies,
            rivals: &[],
        };
        let p = build_recap_prompt(&input, "fr");
        assert!(
            p.contains("mémoire de Le Scientifique")
                && p.contains("- « Les données montrent une transformation. »")
                && p.contains("Tes alliés : La Juriste")
                && !p.contains("Tes rivaux")
        );
        assert!(
            p.contains("\"lesson\"") && p.contains(&constants::RECAP_LIST_MAX_ITEMS.to_string())
        );
        assert!(
            build_recap_prompt(&input, "en").contains("[End of the discussion")
                && build_recap_prompt(&input, "zh").contains("讨论结束")
        );
    }

    #[test]
    fn casting_prompt_bounds_the_catalogue_without_cutting_a_line() {
        let long = "p".repeat(constants::CASTING_PERSONALITY_MAX_CHARS + 50);
        let glads: Vec<CastingCandidate<'_>> = (0..6)
            .map(|i| CastingCandidate {
                id: if i == 0 { "g0" } else { "gx" },
                name: "Nom",
                personality: &long,
            })
            .collect();
        let arbs = [CastingCandidate {
            id: "arb",
            name: "Le Modérateur",
            personality: "impartial",
        }];
        // 600 chars: a quarter for the moderators, ~134 chars per gladiateur line → four lines fit, not five
        let (system, user) = build_casting_prompt(
            "sujet",
            &DiscussionMode::Debate,
            "fr",
            3,
            &glads,
            &arbs,
            600,
        );
        assert!(system.contains("JSON"));
        assert!(user.contains("- arb — Le Modérateur : impartial"));
        // Each line ≤ id + name + bounded personality; the total stays under the bound, no half line
        let lines: Vec<&str> = user.lines().filter(|l| l.starts_with("- g")).collect();
        assert_eq!(lines.len(), 4, "{user}");
        assert!(lines
            .iter()
            .all(|l| l.chars().count() <= constants::CASTING_PERSONALITY_MAX_CHARS + 20));
        assert!(user.contains("exactement 3 GladIAteurs"));
        // Zero bound: catalogue empty but the prompt still well-formed
        let (_, user) =
            build_casting_prompt("sujet", &DiscussionMode::Debate, "en", 2, &glads, &arbs, 0);
        assert!(!user.contains("- g0") && user.contains("\"gladiateurs\""));
    }

    #[test]
    fn test_build_full_document_block_truncation() {
        let long_doc = "A".repeat(10_000);
        let result = build_full_document_block(Some(&long_doc), 100, "en");
        // The document text should be truncated to ~100 chars
        assert!(result.contains("REFERENCE DOCUMENT"));
        // The result should NOT contain all 10K chars (instruction + header + delimiters ≈ 700 chars)
        assert!(
            result.len() < 1000,
            "Block should be truncated, got {} chars",
            result.len()
        );
    }
}
