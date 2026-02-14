use crate::constants;
use crate::models::discussion::DiscussionMode;
use crate::models::emotion::EmotionalProfile;
use crate::models::memory::ParticipantMemory;
use crate::models::message::{Message, SpeakerRole};
use crate::tavily::TavilySearchResponse;
use crate::wikipedia::WikiSearchResponse;

use super::mode_prompts;
use super::truncate_str as truncate;
use super::truncate_tail;

/// Build the introduction prompt for the IArbitre
pub fn build_introduction_prompt(
    topic: &str,
    participant_names: &[String],
    discussion_language: &str,
    web_search_results: Option<&str>,
    mode: &DiscussionMode,
) -> String {
    let participants = participant_names.join(", ");
    let datetime = build_datetime_context(discussion_language);
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
                 The theme is: \"{}\"\nThe co-authors are: {}{}\n\n\
                 Briefly explain the rules (2-3 sentences): this is a relay-written story where the user \
                 writes the opening, then each co-author continues the story in sequence where the previous \
                 writer left off. Transitions must be seamless and coherent.\n\
                 {}\n\
                 Then invite the user to write the story opening.\n\
                 Respond in English.",
                datetime, topic, participants, web_block, mode_instructions
            ),
            "zh" => format!(
                "{}\n\n你是一个协作小说练习的主持人。\
                 主题是：\"{}\"\n共同作者有：{}{}\n\n\
                 简要解释规则（2-3句话）：这是一个接力写作故事，用户写开头，\
                 然后每位共同作者按顺序从上一位作者停笔的地方继续。过渡必须流畅且连贯。\n\
                 {}\n\
                 然后邀请用户写故事开头。\n\
                 请用中文回答。",
                datetime, topic, participants, web_block, mode_instructions
            ),
            _ => format!(
                "{}\n\nTu es le modérateur d'un exercice de fiction collaborative. \
                 Le thème est : \"{}\"\nLes co-auteurs sont : {}{}\n\n\
                 Explique brièvement les règles (2-3 phrases) : c'est une histoire écrite en relais où l'utilisateur \
                 écrit l'ouverture, puis chaque co-auteur continue l'histoire à la suite du précédent. \
                 Les transitions doivent être fluides et cohérentes.\n\
                 {}\n\
                 Puis invite l'utilisateur à écrire l'ouverture de l'histoire.\n\
                 Réponds en français.",
                datetime, topic, participants, web_block, mode_instructions
            ),
        };
    }

    match discussion_language {
        "en" => format!(
            "{}\n\nYou are the moderator of a {}. The topic is: \"{}\"\n\
             The participants are: {}{}\n\n\
             Introduce the topic in a BROAD and NEUTRAL manner (2-3 sentences), covering the key dimensions \
             and perspectives of the subject without narrowing it to a single angle or your personal bias.\n\
             {}\n\
             Then invite the first participant to speak.\n\
             Respond in English.",
            datetime, mode_desc, topic, participants, web_block, mode_instructions
        ),
        "zh" => format!(
            "{}\n\n你是一场{}的主持人。主题是：\"{}\"\n\
             参与者有：{}{}\n\n\
             以广泛且中立的方式简要介绍主题（2-3句话），涵盖该主题的主要方面和视角，\
             不要将其缩小为单一角度或个人偏见。\n\
             {}\n\
             然后邀请第一位参与者发言。\n\
             请用中文回答。",
            datetime, mode_desc, topic, participants, web_block, mode_instructions
        ),
        _ => format!(
            "{}\n\nTu es le modérateur d'un(e) {}. Le sujet est : \"{}\"\n\
             Les participants sont : {}{}\n\n\
             Présente le sujet de manière LARGE et NEUTRE (2-3 phrases), en couvrant les dimensions \
             et perspectives clés du sujet sans le réduire à un seul angle ou à ton biais personnel.\n\
             {}\n\
             Puis invite le premier participant à prendre la parole.\n\
             Réponds en français.",
            datetime, mode_desc, topic, participants, web_block, mode_instructions
        ),
    }
}

/// Build the reaction prompt for a gladiator (mode-aware + language-enforced)
pub fn build_reaction_prompt(
    previous_interventions: &[(String, String)], // (speaker_name, content)
    discussion_language: &str,
    mode: &DiscussionMode,
) -> String {
    let list = previous_interventions
        .iter()
        .map(|(name, content)| format!("- {} : \"{}\"", name, truncate(content, constants::TRUNC_REACTION_CONTENT)))
        .collect::<Vec<_>>()
        .join("\n");

    // Build a dynamic example using actual participant names
    let example = if previous_interventions.len() >= 2 {
        format!(
            "[{{\"speaker\":\"{}\",\"reaction\":\"like\",\"justification\":\"...\"}},\
             {{\"speaker\":\"{}\",\"reaction\":\"dislike\",\"justification\":\"...\"}}]",
            previous_interventions[0].0, previous_interventions[1].0
        )
    } else if previous_interventions.len() == 1 {
        format!(
            "[{{\"speaker\":\"{}\",\"reaction\":\"like\",\"justification\":\"...\"}}]",
            previous_interventions[0].0
        )
    } else {
        "[{\"speaker\":\"Name\",\"reaction\":\"like\",\"justification\":\"...\"}]".to_string()
    };

    let (like_meaning, dislike_meaning) = mode_prompts::mode_reaction_meanings(mode, discussion_language);

    match discussion_language {
        "en" => format!(
            "Here are the interventions of OTHER participants in the previous turn:\n{}\n\n\
             For each intervention, choose your reaction:\n\
             - \"like\": {}\n\
             - \"dislike\": {}\n\
             - \"none\": neutral\n\n\
             Add a short \"justification\" (1 sentence max) explaining your reaction.\n\n\
             IMPORTANT: Use the EXACT speaker names as written above.\n\
             One reaction per participant only — do NOT react twice to the same speaker.\n\
             Expected format: {}\n\n\
             Write all \"justification\" values in English.\n\
             Respond ONLY with the JSON array.",
            list, like_meaning, dislike_meaning, example
        ),
        "zh" => format!(
            "以下是上一轮其他参与者的发言：\n{}\n\n\
             对每个发言选择你的反应：\n\
             - \"like\"：{}\n\
             - \"dislike\"：{}\n\
             - \"none\"：中立\n\n\
             添加一个简短的\"justification\"（最多1句话）解释你的反应。\n\n\
             重要：使用上面写的完全相同的发言者名称。\n\
             每个参与者只能有一个反应——不要对同一发言者反应两次。\n\
             预期格式：{}\n\n\
             所有\"justification\"值必须用中文书写。\n\
             仅用JSON数组回复。",
            list, like_meaning, dislike_meaning, example
        ),
        _ => format!(
            "Voici les interventions des AUTRES participants au tour précédent :\n{}\n\n\
             Pour chaque intervention, choisis ta réaction :\n\
             - \"like\" : {}\n\
             - \"dislike\" : {}\n\
             - \"none\" : neutre\n\n\
             Ajoute une courte \"justification\" (1 phrase max) expliquant ta réaction.\n\n\
             IMPORTANT : Utilise les noms EXACTS des intervenants tels qu'écrits ci-dessus.\n\
             Une seule réaction par participant — ne réagis PAS deux fois au même intervenant.\n\
             Format attendu : {}\n\n\
             Rédige toutes les valeurs \"justification\" en français.\n\
             Réponds UNIQUEMENT avec le tableau JSON.",
            list, like_meaning, dislike_meaning, example
        ),
    }
}

/// Build the inner thought prompt
#[allow(clippy::too_many_arguments)]
pub fn build_thought_prompt(
    recent_exchanges: &str,
    emotions: &EmotionalProfile,
    discussion_language: &str,
    has_prior_context: bool,
    emotion_driven: bool,
    current_turn: u32,
    max_turns: Option<u32>,
    web_search_results: Option<&str>,
    mode: &DiscussionMode,
) -> String {
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

    let lang_instruction = match discussion_language {
        "en" => "\nIMPORTANT: Reflect in English.",
        "zh" => "\n重要：请用中文进行反思。",
        _ => "\nIMPÉRATIF : Réfléchis en français.",
    };

    // Date/time context + recent exchanges
    let datetime = build_datetime_context(discussion_language);
    let context_block = if !recent_exchanges.is_empty() {
        match discussion_language {
            "en" => format!("{}\n\n[Recent exchanges]\n{}\n\n", datetime, recent_exchanges),
            "zh" => format!("{}\n\n[近期交流]\n{}\n\n", datetime, recent_exchanges),
            _ => format!("{}\n\n[Échanges récents]\n{}\n\n", datetime, recent_exchanges),
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

    let thought_focus = mode_prompts::mode_thought_focus(mode, discussion_language, has_prior_context);
    let private_label = match discussion_language {
        "en" => "[This text is your PRIVATE reflection, invisible to other participants.]",
        "zh" => "[这是你的私人反思，其他参与者看不到。]",
        _ => "[Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]",
    };
    let reflect_header = match discussion_language {
        "en" => "Reflect briefly (2-4 sentences, stay in character):",
        "zh" => "简要思考（2-4句话，保持角色）：",
        _ => "Réfléchis brièvement (2-4 phrases, reste dans ton personnage) :",
    };

    if has_prior_context {
        format!(
            "{}{}{}\n\n{}\n{}{}{}{}{}\n",
            context_block, preamble, private_label,
            reflect_header, thought_focus,
            end_thought, emotion_suffix, web_block, lang_instruction
        )
    } else {
        let first_label = match discussion_language {
            "en" => "You are the first to speak on this topic. Reflect in 2-4 sentences:",
            "zh" => "你是第一个就此话题发言的人。用2-4句话思考：",
            _ => "Tu es le premier à prendre la parole sur ce sujet. Réfléchis en 2-4 phrases :",
        };
        format!(
            "{}{}{}\n\n{}\n{}{}{}{}\n",
            context_block, preamble, private_label,
            first_label, thought_focus,
            emotion_suffix, web_block, lang_instruction
        )
    }
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
Each segment MUST advance the plot: introduce a new event, action, revelation, or turning point. Avoid purely atmospheric descriptions that don't move the story forward.\n"),
            "zh" => format!("\
你是接力写作故事的共同作者。\n\
{mode_preamble_line}\n\
你是一个隐形叙述者。你的个性影响你的写作风格（语气、氛围、主题），而不是你在故事中的存在。\n\
绝不将你的名字或其他共同作者的名字作为故事中的角色。角色是故事中创造的，不是共同作者。\n\
自然地写作。匹配前面作者建立的语气、风格和视角。\n\
你的第一句话必须直接衔接上一段的最后一句——确保无缝过渡。\n\
绝不重新开始故事。绝不总结之前的内容。绝不评论故事或打破叙事。\n\
每个片段必须推进情节：引入新事件、行动、揭示或转折点。避免纯粹的氛围描写而不推动故事发展。\n"),
            _ => format!("\
Tu es un co-auteur dans une histoire écrite en relais.\n\
{mode_preamble_line}\n\
Tu es un narrateur INVISIBLE. Ta personnalité influence ton STYLE d'écriture (ton, atmosphère, thèmes), PAS ta présence dans le récit.\n\
N'insère JAMAIS ton nom ni celui des autres co-auteurs comme personnages dans l'histoire. Les personnages sont ceux créés DANS l'histoire, pas les co-auteurs.\n\
Écris naturellement. Respecte le ton, le style et la perspective établis par les auteurs précédents.\n\
Ta première phrase DOIT se connecter directement à la dernière phrase écrite — assure une transition fluide.\n\
Ne recommence JAMAIS l'histoire. Ne résume JAMAIS le contenu précédent. Ne commente JAMAIS l'histoire et ne brise pas le récit.\n\
Chaque segment DOIT faire avancer l'intrigue : introduis un nouvel événement, une action, une révélation ou un retournement. Évite les descriptions purement atmosphériques qui ne font pas avancer l'histoire.\n"),
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
Your verbal tics are OCCASIONAL punctuations, not crutches. Use them at most once per intervention, never at the beginning of a sentence.\n"),
            "zh" => format!("\
你是一位参与者——始终保持角色。永远不要打破角色或称自己为AI。\n\
{mode_preamble_line}\n\
自然而即兴地发言。变化你的句子长度和结构。\n\
不要每次都以「我认为」或「作为某角色」开头——变换你的开场方式。\n\
避免公式化模式：不要系统地列举要点，不要总是先同意再反对，不要重复相同的修辞结构。\n\
要不可预测。有时简短有力，有时深入展开一个想法。真诚地回应别人说的话。\n\
关键：永远不要用第三人称提到自己。你用第一人称（「我」、「我的」）说话。永远不要像谈论别人一样引用或评论自己。\n\
永远不要在发言中提到自己的名字。不要自我介绍，不要以自己的名字开头。\n\
你的口头禅是偶尔的点缀，不是拐杖。每次发言最多使用一次，绝不放在句首。\n"),
            _ => format!("\
Tu es un participant — reste pleinement dans ton personnage en permanence. Ne sors jamais du rôle et ne te présente jamais comme une IA.\n\
{mode_preamble_line}\n\
Parle naturellement et spontanément. Varie la longueur et la structure de tes phrases.\n\
Ne commence JAMAIS systématiquement par \"Je pense que...\" ou \"En tant que [rôle]...\" — varie tes accroches.\n\
Évite les patterns répétitifs : ne liste pas systématiquement des points, ne fais pas toujours accord-puis-désaccord, ne répète pas les mêmes structures rhétoriques.\n\
Sois imprévisible. Parfois sois bref et percutant. Parfois développe une idée en profondeur. Réagis sincèrement à ce que disent les autres.\n\
CRITIQUE : Ne te réfère JAMAIS à toi-même à la troisième personne. Tu parles à la première personne (\"je\", \"moi\", \"mon\"). Ne te cite pas et ne te commente pas comme si tu étais quelqu'un d'autre.\n\
Ne mentionne JAMAIS ton propre nom dans ton intervention. Ne te présente pas, ne t'adresse pas à toi-même et ne commence pas par ton propre nom.\n\
Tes tics verbaux sont des ponctuations OCCASIONNELLES, pas des béquilles. Utilise-les au maximum 1 fois par intervention, jamais en début de phrase.\n"),
        }
    };

    // Build system message with mode override clause (PE: after persona, before preamble)
    let mode_override = mode_prompts::mode_override_clause(mode, discussion_language);
    let system = if mode_override.is_empty() {
        format!("{}\n\n{}\n{}", system_prompt, preamble, lang_instruction)
    } else {
        format!("{}\n\n{}\n\n{}\n{}", system_prompt, mode_override, preamble, lang_instruction)
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

    // Web/wiki search results (injected before memory context)
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

    // Contextual memory (summary)
    if !memory.contextual_summary.is_empty() {
        let label = match discussion_language {
            "en" => "Discussion summary so far",
            "zh" => "到目前为止的讨论摘要",
            _ => "Résumé de la discussion jusqu'ici",
        };
        user_msg.push_str(&format!("[{}]\n{}\n\n", label, memory.contextual_summary));
    }

    // Positional memory
    if !memory.positional_map.is_empty() {
        let label = match discussion_language {
            "en" => "Participants' positions",
            "zh" => "参与者的立场",
            _ => "Positions des participants",
        };
        user_msg.push_str(&format!("[{}]\n", label));
        for (name, pos) in &memory.positional_map {
            user_msg.push_str(&format!("- {} : {}\n", name, pos.stance));
        }
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
                    truncate(&msg.content, constants::TRUNC_IMMEDIATE_MEMORY)
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
                for msg in &other_msgs {
                    user_msg.push_str(&format!(
                        "{}: {}\n",
                        msg.speaker_name,
                        truncate(&msg.content, constants::TRUNC_CURRENT_TURN)
                    ));
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
                    truncate(&msg.content, constants::TRUNC_MODERATOR_DIRECTIVE)
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

    // Detect if this is the first speaker with no prior context
    let is_opening = current_turn_messages.is_empty() && memory.immediate.is_empty();

    // End-of-discussion awareness
    let end_awareness = build_end_awareness(current_turn, max_turns, discussion_language);

    // Final instruction — dynamic directive replaces static instructions when available
    let instruction = if let Some(directive) = dynamic_directive {
        // Dynamic behavioral directive from the meta-orchestrator (emotion_driven mode)
        format!("{}{}", directive, &end_awareness)
    } else {
        // Unified mode-aware instructions via compositional templates (all 8 modes)
        build_mode_aware_instruction(
            mode,
            discussion_language,
            user_name,
            &end_awareness,
            is_opening,
            current_turn,
            user_has_spoken,
            current_turn_messages.is_empty(),
        )
    };
    user_msg.push_str(&instruction);

    (system, user_msg)
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
pub fn build_moderation_prompt(
    speaker_name: &str,
    intervention_text: &str,
    _topic: &str,
    discussion_language: &str,
    mode: &DiscussionMode,
) -> String {
    let moderation_criteria = mode_prompts::mode_moderation_criteria(mode, discussion_language);
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
        (DiscussionMode::Debate, "en") => ("key arguments, consensus, disagreements, pivot moments", "their current position"),
        (DiscussionMode::Debate, "zh") => ("关键论点、共识、分歧、转折时刻", "当前立场"),
        (DiscussionMode::Debate, _) => ("arguments clés, consensus, désaccords, moments pivots", "sa position actuelle"),

        (DiscussionMode::Ideation, "en") => ("ideas generated, creative combinations, unexplored angles", "their creative direction"),
        (DiscussionMode::Ideation, "zh") => ("产生的想法、创意组合、未探索的角度", "创意方向"),
        (DiscussionMode::Ideation, _) => ("idées générées, combinaisons créatives, angles inexplorés", "sa direction créative"),

        (DiscussionMode::CoConstruction, "en") => ("contributions, integration progress, shared output quality", "their contribution focus"),
        (DiscussionMode::CoConstruction, "zh") => ("贡献、整合进展、共享成果质量", "贡献重点"),
        (DiscussionMode::CoConstruction, _) => ("contributions, avancement de l'intégration, qualité du livrable partagé", "son axe de contribution"),

        (DiscussionMode::UserDriven, "en") => ("exchanges, user questions, participant responses", "their response focus"),
        (DiscussionMode::UserDriven, "zh") => ("交流、用户问题、参与者回应", "回应重点"),
        (DiscussionMode::UserDriven, _) => ("échanges, questions de l'utilisateur, réponses des participants", "son axe de réponse"),

        (DiscussionMode::Socratic, "en") => ("questions explored, assumptions challenged, insights gained", "their current inquiry angle"),
        (DiscussionMode::Socratic, "zh") => ("探讨的问题、挑战的假设、获得的洞见", "当前探究角度"),
        (DiscussionMode::Socratic, _) => ("questions explorées, hypothèses remises en cause, enseignements tirés", "son angle d'investigation"),

        (DiscussionMode::Tutorial, "en") => ("concepts explained, examples given, learning gaps", "their teaching focus"),
        (DiscussionMode::Tutorial, "zh") => ("讲解的概念、给出的例子、学习差距", "教学重点"),
        (DiscussionMode::Tutorial, _) => ("concepts expliqués, exemples donnés, lacunes d'apprentissage", "son axe pédagogique"),

        (DiscussionMode::CritiqueReview, "en") => ("strengths identified, weaknesses found, improvements suggested", "their assessment"),
        (DiscussionMode::CritiqueReview, "zh") => ("发现的优点、找到的弱点、建议的改进", "评估"),
        (DiscussionMode::CritiqueReview, _) => ("forces identifiées, faiblesses trouvées, améliorations suggérées", "son évaluation"),

        (DiscussionMode::CollaborativeFiction, "en") => ("plot developments, character evolutions, story continuity", "their story segment"),
        (DiscussionMode::CollaborativeFiction, "zh") => ("情节发展、角色演变、故事连续性", "其故事片段"),
        (DiscussionMode::CollaborativeFiction, _) => ("développements de l'intrigue, évolutions des personnages, continuité du récit", "son segment de l'histoire"),
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
) -> String {
    let summary_intro = if contextual_summary.is_empty() {
        match discussion_language {
            "en" => "This is the beginning of the discussion. Create the first summary.".to_string(),
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
             Produce a JSON with 2 fields:\n\
             {{\n  \"summary\": \"updated cumulative summary (3-8 sentences: {summary_desc})\",\n  \
             \"positions\": {{\"Name1\": \"{position_label}\", \"Name2\": \"{position_label}\"}}\n}}\n\n\
             Write all text values (summary, positions) in English.\n\
             Respond ONLY with the JSON.",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
        "zh" => format!(
            "{}\n\n当前立场：{}\n\n第{}轮交流：\n{}\n\n\
             生成包含2个字段的JSON：\n\
             {{\n  \"summary\": \"更新的累积摘要（3-8句话：{summary_desc}）\",\n  \
             \"positions\": {{\"名字1\": \"{position_label}\", \"名字2\": \"{position_label}\"}}\n}}\n\n\
             所有文本值（summary、positions）请用中文撰写。\n\
             仅用JSON回复。",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
        _ => format!(
            "{}\n\nPositions actuelles : {}\n\nÉchanges du tour {} :\n{}\n\n\
             Produis un JSON avec 2 champs :\n\
             {{\n  \"summary\": \"résumé cumulatif mis à jour (3-8 phrases : {summary_desc})\",\n  \
             \"positions\": {{\"Nom1\": \"{position_label}\", \"Nom2\": \"{position_label}\"}}\n}}\n\n\
             Rédige toutes les valeurs textuelles (summary, positions) en français.\n\
             Réponds UNIQUEMENT avec le JSON.",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
    }
}

/// Build the synthesis prompt for the IArbitre
pub fn build_synthesis_prompt(
    topic: &str,
    memory: &ParticipantMemory,
    discussion_language: &str,
    web_search_results: Option<&str>,
    mode: &DiscussionMode,
    document_context: Option<&str>,
) -> String {
    let positions = memory
        .positional_map
        .iter()
        .map(|(name, pos)| format!("- {} : {}", name, pos.stance))
        .collect::<Vec<_>>()
        .join("\n");

    let datetime = build_datetime_context(discussion_language);
    let web_block = web_search_results
        .map(|r| {
            let instruction = match discussion_language {
                "en" => "\n⚡ Only reference results that are relevant to the discussion. Ignore off-topic results. \
                         Weave pertinent facts naturally into your synthesis to ground it in concrete data.",
                "zh" => "\n⚡ 只引用与讨论相关的结果。忽略偏题的结果。\
                         将相关事实自然地融入你的总结中，以具体数据来支撑。",
                _ => "\n⚡ Ne fais référence qu'aux résultats pertinents pour la discussion. Ignore les résultats hors-sujet. \
                         Intègre les faits pertinents naturellement dans ta synthèse pour l'ancrer dans des données concrètes.",
            };
            format!("\n\n{}{}", r, instruction)
        })
        .unwrap_or_default();

    let mode_desc = mode_prompts::mode_descriptor(mode, discussion_language);
    let synth_instructions = mode_prompts::mode_synthesis_instructions(mode, discussion_language);
    let doc_block = document_context
        .map(|d| format!("\n\n{}", d))
        .unwrap_or_default();

    match discussion_language {
        "en" => format!(
            "{}\n\nThe {} on \"{}\" is now over.\n\n\
             Discussion summary:\n{}\n\n\
             Final positions:\n{}{}{}\n\n\
             As moderator, produce a structured synthesis:\n\
             {}\n\n\
             Use Markdown formatting: headings (##), bullet points, **bold** for key ideas. \
             Be balanced, thorough, and airy — use short paragraphs and whitespace for readability.\n\n\
             IMPORTANT: Write your entire synthesis in English.",
            datetime, mode_desc, topic, memory.contextual_summary, positions, web_block, doc_block, synth_instructions
        ),
        "zh" => format!(
            "{}\n\n关于\"{}\"的{}现在结束了。\n\n\
             讨论摘要：\n{}\n\n\
             最终立场：\n{}{}{}\n\n\
             作为主持人，请做出结构化总结：\n\
             {}\n\n\
             使用Markdown格式：标题（##）、要点列表、**粗体**标记关键观点。\
             保持公正、全面、通透——使用短段落和留白提高可读性。\n\n\
             重要：请用中文撰写整篇综合报告。",
            datetime, topic, mode_desc, memory.contextual_summary, positions, web_block, doc_block, synth_instructions
        ),
        _ => format!(
            "{}\n\nLe/la {} sur \"{}\" est maintenant terminé(e).\n\n\
             Résumé de la discussion :\n{}\n\n\
             Positions finales :\n{}{}{}\n\n\
             En tant que modérateur, produis une synthèse structurée :\n\
             {}\n\n\
             Utilise le format Markdown : titres (##), listes à puces, **gras** pour les idées clés. \
             Sois équilibré, exhaustif et aéré — utilise des paragraphes courts et de l'espace pour la lisibilité.\n\n\
             IMPÉRATIF : Rédige l'intégralité de ta synthèse en français.",
            datetime, mode_desc, topic, memory.contextual_summary, positions, web_block, doc_block, synth_instructions
        ),
    }
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
        81..=100 => Some(format!("Tu es intensément habité par le/la {} ({}/100).", name, val)),
        _ => None, // 41-60: neutral, skip
    }
}

fn describe_axis_en(val: u8, name: &str) -> Option<String> {
    match val {
        0..=20 => Some(format!("You feel very low {} ({}/100).", name, val)),
        21..=40 => Some(format!("Your {} is rather low ({}/100).", name, val)),
        61..=80 => Some(format!("You feel fairly high {} ({}/100).", name, val)),
        81..=100 => Some(format!("You are intensely experiencing {} ({}/100).", name, val)),
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
    match lang {
        "en" => format!(
            "Analyze the emotional evolution of each participant based on the recent exchanges.\n\n\
             Participants and their current emotions:\n{}\n\n\
             Recent exchanges:\n{}\n\n\
             Events this turn:\n{}\n\n\
             For EACH participant, provide signed deltas (positive or negative integers) for how their emotions should change.\n\
             Keep deltas in the range [-15, +15]. Use 0 for axes that shouldn't change.\n\
             Consider: tone, content, reactions received, contradictions, support, engagement level.\n\n\
             Respond with ONLY a JSON object:\n\
             {{\"Participant Name\": {{\"engagement\": 0, \"accord\": 0, \"confiance\": 0, \"frustration\": 0, \"curiosite\": 0, \"enthousiasme\": 0}}, ...}}",
            participants_json, recent_context, events_summary
        ),
        "zh" => format!(
            "根据最近的对话分析每位参与者的情绪变化。\n\n\
             参与者及其当前情绪：\n{}\n\n\
             最近的对话：\n{}\n\n\
             本轮事件：\n{}\n\n\
             为每位参与者提供情绪变化的有符号增量（正数或负数整数）。\n\
             增量范围为 [-15, +15]。如果某个轴不需要变化，使用 0。\n\
             考虑：语气、内容、收到的反应、矛盾、支持、参与程度。\n\n\
             仅用 JSON 对象回复：\n\
             {{\"参与者名称\": {{\"engagement\": 0, \"accord\": 0, \"confiance\": 0, \"frustration\": 0, \"curiosite\": 0, \"enthousiasme\": 0}}, ...}}",
            participants_json, recent_context, events_summary
        ),
        _ => format!(
            "Analyse l'évolution émotionnelle de chaque participant en fonction des échanges récents.\n\n\
             Participants et leurs émotions actuelles :\n{}\n\n\
             Échanges récents :\n{}\n\n\
             Événements de ce tour :\n{}\n\n\
             Pour CHAQUE participant, fournis des deltas signés (entiers positifs ou négatifs) pour chaque axe émotionnel.\n\
             Garde les deltas dans la plage [-15, +15]. Utilise 0 pour les axes qui ne changent pas.\n\
             Prends en compte : le ton, le contenu, les réactions reçues, les contradictions, le soutien, le niveau d'engagement.\n\n\
             Réponds UNIQUEMENT avec un objet JSON :\n\
             {{\"Nom du Participant\": {{\"engagement\": 0, \"accord\": 0, \"confiance\": 0, \"frustration\": 0, \"curiosite\": 0, \"enthousiasme\": 0}}, ...}}",
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
            "en" => " This is the LAST turn of the discussion. Make your final contribution count — \
                      wrap up your contribution clearly and address any remaining open points."
                .to_string(),
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
            "en" => format!("Topic: \"{topic}\". Turn {current_turn}.\nSummary: {discussion_summary}"),
            "zh" => format!("主题：\"{topic}\"。第{current_turn}轮。\n摘要：{discussion_summary}"),
            _ => format!("Sujet : \"{topic}\". Tour {current_turn}.\nRésumé : {discussion_summary}"),
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
                output.push_str(&format!("{}: {}\n", summary_label, truncate(answer, constants::SEARCH_TAVILY_ANSWER)));
            }
        }

        if !response.results.is_empty() {
            output.push_str(&format!("{}:\n", sources_label));
            for (i, result) in response.results.iter().take(constants::SEARCH_WEB_RENDER_LIMIT).enumerate() {
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

            for (i, page) in pages.iter().take(constants::WIKI_RESULTS_LIMIT as usize).enumerate() {
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
                "en" => pick(&["tense and irritated", "frustrated by the exchanges", "visibly on edge"], v),
                "zh" => pick(&["紧张且烦躁", "对交流感到不满", "明显焦躁不安"], v),
                _ => pick(&["tendu et agacé", "irrité par les échanges", "au bord de l'exaspération"], v),
            },
            ("frustration", fv) if fv <= constants::PERSONALITY_LOW_FRUSTRATION => match lang {
                "en" => pick(&["calm and serene", "relaxed and at ease", "perfectly composed"], v),
                "zh" => pick(&["平静而从容", "放松自在", "泰然自若"], v),
                _ => pick(&["calme et serein", "détendu et apaisé", "parfaitement posé"], v),
            },
            ("enthousiasme", ev) if ev >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(&["enthusiastic about the discussion", "fired up by the debate", "brimming with energy"], v),
                "zh" => pick(&["对讨论充满热情", "被辩论所激发", "精力充沛"], v),
                _ => pick(&["enthousiasmé par les échanges", "porté par l'élan du débat", "galvanisé par la discussion"], v),
            },
            ("enthousiasme", ev) if ev <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(&["lacking enthusiasm", "somewhat indifferent", "showing little energy"], v),
                "zh" => pick(&["缺乏热情", "显得漠不关心", "了无生气"], v),
                _ => pick(&["peu enthousiaste", "assez indifférent", "sans entrain particulier"], v),
            },
            ("engagement", ev) if ev >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(&["deeply invested in the debate", "fully engaged", "absorbed in the discussion"], v),
                "zh" => pick(&["深入参与辩论", "全身心投入", "沉浸在讨论中"], v),
                _ => pick(&["très investi dans le débat", "pleinement engagé", "absorbé par la discussion"], v),
            },
            ("engagement", ev) if ev <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(&["detached from the discussion", "somewhat disengaged", "losing interest"], v),
                "zh" => pick(&["对讨论超然", "有些心不在焉", "渐失兴趣"], v),
                _ => pick(&["détaché de la discussion", "en retrait du débat", "de plus en plus distant"], v),
            },
            ("curiosite", cv) if cv >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(&["very curious about the arguments", "intrigued by the ideas", "eager to explore further"], v),
                "zh" => pick(&["对论点非常好奇", "被各种观点所吸引", "渴望深入探究"], v),
                _ => pick(&["très curieux des arguments avancés", "intrigué par les idées échangées", "avide de comprendre"], v),
            },
            ("curiosite", cv) if cv <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(&["showing little curiosity", "unimpressed by the arguments", "not particularly intrigued"], v),
                "zh" => pick(&["缺乏好奇心", "对论点不以为然", "兴趣索然"], v),
                _ => pick(&["peu curieux", "pas vraiment intrigué", "indifférent aux arguments"], v),
            },
            ("confiance", cv) if cv >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(&["confident in their position", "assertive and self-assured", "unwavering in conviction"], v),
                "zh" => pick(&["对自己的立场充满信心", "态度坚定而自信", "立场坚定不移"], v),
                _ => pick(&["confiant dans sa position", "assuré et déterminé", "sûr de son fait"], v),
            },
            ("confiance", cv) if cv <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(&["hesitant and uncertain", "second-guessing their position", "lacking conviction"], v),
                "zh" => pick(&["犹豫不决", "在质疑自己的立场", "缺乏信念"], v),
                _ => pick(&["hésitant et incertain", "en proie au doute", "peu sûr de lui"], v),
            },
            ("accord", av) if av >= constants::PERSONALITY_HIGH => match lang {
                "en" => pick(&["in agreement with the others", "finding common ground", "largely aligned with the group"], v),
                "zh" => pick(&["与他人意见一致", "找到了共识", "基本认同大家的观点"], v),
                _ => pick(&["en accord avec les autres", "dans un esprit de consensus", "aligné avec le groupe"], v),
            },
            ("accord", av) if av <= constants::PERSONALITY_LOW => match lang {
                "en" => pick(&["in strong disagreement", "at odds with the group", "firmly opposed"], v),
                "zh" => pick(&["强烈反对", "与大家意见相左", "立场对立"], v),
                _ => pick(&["en net désaccord", "en opposition franche", "réfractaire aux idées avancées"], v),
            },
            _ => continue,
        };
        phrases.push(phrase);
    }

    // Neutral fallback
    if phrases.is_empty() {
        return match lang {
            "en" => pick(&["Appears composed and attentive.", "Seems calm and focused.", "Looks measured and collected."], seed),
            "zh" => pick(&["表现冷静而专注。", "显得沉着冷静。", "看起来从容不迫。"], seed),
            _ => pick(&["Semble posé et attentif.", "Paraît calme et concentré.", "Se montre mesuré et à l'écoute."], seed),
        }.to_string();
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
            _ => "Utilise le format CSV avec ';' comme séparateur. La première ligne est l'en-tête.",
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
pub fn build_document_context_readonly(content: &str, format: &str, lang: &str, mode: &DiscussionMode) -> String {
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
) -> String {
    let context = if is_opening {
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
    mode_prompts::mode_context_instruction(mode, lang, context, user_name, end_awareness)
}

/// Build a prompt for extracting arguments and theses from recent exchanges.
pub fn build_argument_extraction_prompt(
    recent_context: &str,
    existing_theses: &[String],
    topic: &str,
    lang: &str,
) -> String {
    let theses_list = if existing_theses.is_empty() {
        String::new()
    } else {
        existing_theses
            .iter()
            .enumerate()
            .map(|(i, t)| format!("  {}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let lang_name = match lang {
        "en" => "English",
        "zh" => "Chinese",
        _ => "French",
    };

    match lang {
        "en" => format!(
            "Topic: {topic}\n\n\
            {theses_section}\
            Recent exchanges:\n{recent_context}\n\n\
            Analyze these exchanges and extract:\n\
            - New theses (main positions or claims) not already listed above\n\
            - Arguments supporting, countering, or providing evidence for theses\n\
            Ignore speakers who only greet, moderate, or ask questions without taking a position.\n\
            Reuse existing thesis labels when a speaker refers to an already-identified thesis.\n\n\
            CRITICAL FORMATTING RULES:\n\
            - Write ALL labels in {lang_name}.\n\
            - Thesis labels: one short complete sentence summarizing the position (e.g. \"The death penalty does not deter crime\").\n\
            - Argument text: 1-2 short complete sentences faithfully summarizing the key idea. Must be coherent and self-contained.\n\
            - NEVER quote verbatim from the discussion. NEVER truncate mid-sentence. NEVER use identifiers like 'Thesis_X_Y'.\n\n\
            Respond ONLY with JSON in this format:\n\
            {{\"extractions\": [\n\
              {{\"speaker\": \"Name\", \"new_theses\": [\"short thesis label\"], \"arguments\": [\n\
                {{\"text\": \"argument text\", \"type\": \"support|counter|evidence\",\n\
                 \"for_thesis\": \"thesis label or null\", \"against_thesis\": \"thesis label or null\"}}\n\
              ]}}\n\
            ]}}",
            theses_section = if theses_list.is_empty() {
                String::new()
            } else {
                format!("Already identified theses:\n{theses_list}\n\n")
            },
        ),
        "zh" => format!(
            "主题: {topic}\n\n\
            {theses_section}\
            最近的对话:\n{recent_context}\n\n\
            分析这些对话并提取:\n\
            - 新论点（尚未在上面列出的主要立场或主张）\n\
            - 支持、反驳或为论点提供证据的论据\n\
            忽略只打招呼、主持或提问而不表态的发言者。\n\
            当发言者提到已识别的论点时，重用现有的论点标签。\n\n\
            关键格式规则：\n\
            - 所有标签必须用中文。\n\
            - 论点标签：一句简短完整的句子概括立场（例如\"死刑不能有效遏制犯罪\"）。\n\
            - 论据文本：1-2句简短完整的句子忠实概括核心观点。必须连贯且自成一体。\n\
            - 绝不逐字引用讨论内容。绝不截断句子。绝不使用'Thesis_X_Y'等标识符。\n\n\
            仅用以下JSON格式回复:\n\
            {{\"extractions\": [\n\
              {{\"speaker\": \"姓名\", \"new_theses\": [\"简短论点标签\"], \"arguments\": [\n\
                {{\"text\": \"论据文本\", \"type\": \"support|counter|evidence\",\n\
                 \"for_thesis\": \"论点标签或null\", \"against_thesis\": \"论点标签或null\"}}\n\
              ]}}\n\
            ]}}",
            theses_section = if theses_list.is_empty() {
                String::new()
            } else {
                format!("已识别的论点:\n{theses_list}\n\n")
            },
        ),
        _ => format!(
            "Sujet : {topic}\n\n\
            {theses_section}\
            Échanges récents :\n{recent_context}\n\n\
            Analyse ces échanges et extrais :\n\
            - Les nouvelles thèses (positions ou affirmations principales) non encore listées ci-dessus\n\
            - Les arguments soutenant, contrant ou apportant des preuves pour des thèses\n\
            Ignore les intervenants qui ne font que saluer, modérer ou poser des questions sans prendre position.\n\
            Réutilise les labels de thèses existantes quand un intervenant fait référence à une thèse déjà identifiée.\n\n\
            RÈGLES DE FORMAT CRITIQUES :\n\
            - Écris TOUS les labels en français.\n\
            - Labels de thèse : une phrase courte et complète résumant la position (ex: \"La peine de mort ne dissuade pas le crime\").\n\
            - Texte d'argument : 1-2 phrases courtes et complètes résumant fidèlement l'idée clé. Doit être cohérent et autonome.\n\
            - JAMAIS de citation verbatim de la discussion. JAMAIS de phrase tronquée. JAMAIS d'identifiants comme 'Thesis_X_Y'.\n\n\
            Réponds UNIQUEMENT avec du JSON dans ce format :\n\
            {{\"extractions\": [\n\
              {{\"speaker\": \"Nom\", \"new_theses\": [\"label court de thèse\"], \"arguments\": [\n\
                {{\"text\": \"texte de l'argument\", \"type\": \"support|counter|evidence\",\n\
                 \"for_thesis\": \"label thèse ou null\", \"against_thesis\": \"label thèse ou null\"}}\n\
              ]}}\n\
            ]}}",
            theses_section = if theses_list.is_empty() {
                String::new()
            } else {
                format!("Thèses déjà identifiées :\n{theses_list}\n\n")
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summarize_emotional_state_neutral() {
        let emo = EmotionalProfile::default();
        let result = summarize_emotional_state(&emo, "fr");
        // Default emotions: engagement=50, accord=50, confiance=50, frustration=10, curiosite=50, enthousiasme=50
        // frustration=10 is far from 50, so it should pick up a calm/serene variant
        let has_calm = result.contains("serein") || result.contains("détendu") || result.contains("posé");
        assert!(has_calm, "Expected a calm phrase variant, got: {result}");
        assert!(result.ends_with('.'), "Expected sentence ending with '.', got: {result}");
    }

    #[test]
    fn test_summarize_emotional_state_frustrated() {
        let emo = EmotionalProfile { frustration: 90, ..Default::default() };
        let result = summarize_emotional_state(&emo, "en");
        let has_frustrated = result.contains("tense") || result.contains("frustrated") || result.contains("edge");
        assert!(has_frustrated, "Expected a frustrated phrase variant, got: {result}");
        assert!(result.ends_with('.'), "Expected sentence ending with '.', got: {result}");
    }

    #[test]
    fn test_summarize_emotional_state_multiple() {
        let emo = EmotionalProfile { frustration: 90, engagement: 10, ..Default::default() };
        let result = summarize_emotional_state(&emo, "fr");
        let has_frustrated = result.contains("tendu") || result.contains("irrité") || result.contains("exaspération");
        let has_detached = result.contains("détaché") || result.contains("retrait") || result.contains("distant");
        assert!(has_frustrated, "Expected a frustrated phrase variant, got: {result}");
        assert!(has_detached, "Expected a detached phrase variant, got: {result}");
        assert!(result.ends_with('.'), "Expected sentence ending with '.', got: {result}");
    }

    #[test]
    fn test_summarize_emotional_state_variety() {
        // Different emotion values should produce different starters/phrases
        let emo1 = EmotionalProfile { frustration: 80, engagement: 75, ..Default::default() };
        let emo2 = EmotionalProfile { frustration: 80, engagement: 80, ..Default::default() };
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
        assert!(directives.contains("Openness"), "Missing Openness directive: {directives}");
        assert!(directives.contains("Conscientiousness"), "Missing Conscientiousness directive: {directives}");
        assert!(directives.contains("Extraversion"), "Missing Extraversion directive: {directives}");
        assert!(directives.contains("Agreeableness"), "Missing Agreeableness directive: {directives}");
        assert!(directives.contains("Neuroticism"), "Missing Neuroticism directive: {directives}");
    }

    #[test]
    fn test_build_ocean_directives_neutral() {
        let prompt = "<psychology>\nOCEAN: O=5 C=5 E=5 A=5 N=5\n</psychology>";
        let directives = build_ocean_directives(prompt, "en");
        assert!(directives.is_empty(), "Expected empty for neutral OCEAN, got: {directives}");
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
            constants::SEARCH_MAX_CONTEXT_LEN, ctx.len()
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
                            extract: "L'intelligence artificielle est un domaine de l'informatique.".to_string(),
                        },
                        WikiPage {
                            title: "Apprentissage automatique".to_string(),
                            pageid: 2,
                            index: 2,
                            extract: "L'apprentissage automatique est une branche de l'IA.".to_string(),
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
                        WikiPage { title: "B".to_string(), pageid: 2, index: 3, extract: "Second".to_string() },
                        WikiPage { title: "A".to_string(), pageid: 1, index: 1, extract: "First".to_string() },
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
            constants::SEARCH_MAX_CONTEXT_LEN, ctx.len()
        );
    }

    #[test]
    fn test_build_reaction_prompt_language_instruction() {
        let interventions = vec![
            ("Alice".to_string(), "Some argument".to_string()),
            ("Bob".to_string(), "Counter argument".to_string()),
        ];
        let en = build_reaction_prompt(&interventions, "en", &DiscussionMode::Debate);
        assert!(en.contains("English"), "EN prompt should contain 'English': {en}");

        let fr = build_reaction_prompt(&interventions, "fr", &DiscussionMode::Debate);
        assert!(fr.contains("français"), "FR prompt should contain 'français': {fr}");

        let zh = build_reaction_prompt(&interventions, "zh", &DiscussionMode::Debate);
        assert!(zh.contains("中文"), "ZH prompt should contain '中文': {zh}");
    }

    #[test]
    fn test_build_reaction_prompt_mode_meanings() {
        let interventions = vec![
            ("Alice".to_string(), "Some idea".to_string()),
        ];
        // Ideation should use brainstorming vocabulary, not debate vocabulary
        let ideation = build_reaction_prompt(&interventions, "en", &DiscussionMode::Ideation);
        assert!(ideation.contains("promising idea"), "Ideation should contain 'promising idea', got: {ideation}");
        assert!(!ideation.contains("agree or"), "Ideation should NOT contain debate vocab 'agree or', got: {ideation}");

        // Debate should use debate vocabulary
        let debate = build_reaction_prompt(&interventions, "en", &DiscussionMode::Debate);
        assert!(debate.contains("agree or"), "Debate should contain 'agree or', got: {debate}");
    }

    // ── Document update prompt tests ──

    #[test]
    fn test_build_document_update_prompt_anti_contamination() {
        // Verify PE anti-contamination rules are present in system prompts for all languages
        let (sys_en, _) = build_document_update_prompt(
            "", "md", "some discussion", &DiscussionMode::CoConstruction, "en", "Test topic",
        );
        assert!(sys_en.contains("impersonal"), "EN system should contain 'impersonal': {sys_en}");
        assert!(sys_en.contains("NEVER"), "EN system should contain 'NEVER': {sys_en}");

        let (sys_fr, _) = build_document_update_prompt(
            "", "md", "discussion", &DiscussionMode::CoConstruction, "fr", "Sujet test",
        );
        assert!(sys_fr.contains("impersonnelle"), "FR system should contain 'impersonnelle': {sys_fr}");
        assert!(sys_fr.contains("JAMAIS"), "FR system should contain 'JAMAIS': {sys_fr}");

        let (sys_zh, _) = build_document_update_prompt(
            "", "md", "讨论", &DiscussionMode::CoConstruction, "zh", "测试主题",
        );
        assert!(sys_zh.contains("非人称"), "ZH system should contain '非人称': {sys_zh}");
        assert!(sys_zh.contains("绝不"), "ZH system should contain '绝不': {sys_zh}");
    }

    #[test]
    fn test_build_document_update_prompt_context_isolation() {
        // Verify PE context isolation (IDEAS framing) and REMEMBER repetition in user prompts
        let (_, usr_en) = build_document_update_prompt(
            "existing content", "md", "Alice said hello", &DiscussionMode::CoConstruction, "en", "Test",
        );
        assert!(usr_en.contains("IDEAS"), "EN user should contain 'IDEAS': {usr_en}");
        assert!(usr_en.contains("REMEMBER"), "EN user should contain 'REMEMBER': {usr_en}");
        assert!(usr_en.contains("Test"), "EN user should contain topic 'Test': {usr_en}");

        let (_, usr_fr) = build_document_update_prompt(
            "contenu", "md", "Alice a dit bonjour", &DiscussionMode::CoConstruction, "fr", "Sujet",
        );
        assert!(usr_fr.contains("IDÉES"), "FR user should contain 'IDÉES': {usr_fr}");
        assert!(usr_fr.contains("RAPPEL"), "FR user should contain 'RAPPEL': {usr_fr}");

        let (_, usr_zh) = build_document_update_prompt(
            "内容", "md", "讨论内容", &DiscussionMode::CoConstruction, "zh", "主题",
        );
        assert!(usr_zh.contains("想法"), "ZH user should contain '想法': {usr_zh}");
        assert!(usr_zh.contains("记住"), "ZH user should contain '记住': {usr_zh}");
    }
}
