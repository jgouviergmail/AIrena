use crate::models::emotion::EmotionalProfile;
use crate::models::memory::ParticipantMemory;
use crate::models::message::{Message, SpeakerRole};

use super::truncate_str as truncate;

/// Build the introduction prompt for the IArbitre
pub fn build_introduction_prompt(
    topic: &str,
    participant_names: &[String],
    discussion_language: &str,
) -> String {
    let participants = participant_names.join(", ");
    match discussion_language {
        "en" => format!(
            "You are the moderator of a debate. The topic is: \"{}\"\n\
             The participants are: {}\n\n\
             Introduce the topic briefly (2-3 sentences) and invite the first participant to speak.\n\
             Respond in English.",
            topic, participants
        ),
        "zh" => format!(
            "你是一场辩论的主持人。主题是：\"{}\"\n\
             参与者有：{}\n\n\
             简要介绍主题（2-3句话），并邀请第一位参与者发言。\n\
             请用中文回答。",
            topic, participants
        ),
        _ => format!(
            "Tu es le modérateur d'un débat. Le sujet est : \"{}\"\n\
             Les participants sont : {}\n\n\
             Présente brièvement le sujet (2-3 phrases) et invite le premier participant à prendre la parole.\n\
             Réponds en français.",
            topic, participants
        ),
    }
}

/// Build the reaction prompt for a gladiator
pub fn build_reaction_prompt(
    previous_interventions: &[(String, String)], // (speaker_name, content)
    discussion_language: &str,
) -> String {
    let list = previous_interventions
        .iter()
        .map(|(name, content)| format!("- {} : \"{}\"", name, truncate(content, 300)))
        .collect::<Vec<_>>()
        .join("\n");

    // Build a dynamic example using actual participant names
    let example = if previous_interventions.len() >= 2 {
        format!(
            "[{{\"speaker\":\"{}\",\"reaction\":\"like\"}},{{\"speaker\":\"{}\",\"reaction\":\"dislike\"}}]",
            previous_interventions[0].0, previous_interventions[1].0
        )
    } else if previous_interventions.len() == 1 {
        format!(
            "[{{\"speaker\":\"{}\",\"reaction\":\"like\"}}]",
            previous_interventions[0].0
        )
    } else {
        "[{\"speaker\":\"Name\",\"reaction\":\"like\"}]".to_string()
    };

    match discussion_language {
        "en" => format!(
            "Here are the interventions of OTHER participants in the previous turn:\n{}\n\n\
             For each intervention, choose your reaction:\n\
             - \"like\": agree or relevant argument\n\
             - \"dislike\": disagree or weak argument\n\
             - \"none\": neutral\n\n\
             IMPORTANT: Use the EXACT speaker names as written above.\n\
             Expected format: {}\n\n\
             Respond ONLY with the JSON array.",
            list, example
        ),
        "zh" => format!(
            "以下是上一轮其他参与者的发言：\n{}\n\n\
             对每个发言选择你的反应：\n\
             - \"like\"：同意或有力论点\n\
             - \"dislike\"：不同意或薄弱论点\n\
             - \"none\"：中立\n\n\
             重要：使用上面写的完全相同的发言者名称。\n\
             预期格式：{}\n\n\
             仅用JSON数组回复。",
            list, example
        ),
        _ => format!(
            "Voici les interventions des AUTRES participants au tour précédent :\n{}\n\n\
             Pour chaque intervention, choisis ta réaction :\n\
             - \"like\" : d'accord ou argument pertinent\n\
             - \"dislike\" : en désaccord ou argument faible\n\
             - \"none\" : neutre\n\n\
             IMPORTANT : Utilise les noms EXACTS des intervenants tels qu'écrits ci-dessus.\n\
             Format attendu : {}\n\n\
             Réponds UNIQUEMENT avec le tableau JSON.",
            list, example
        ),
    }
}

/// Build the inner thought prompt
pub fn build_thought_prompt(
    recent_exchanges: &str,
    emotions: &EmotionalProfile,
    discussion_language: &str,
    has_prior_context: bool,
    emotion_driven: bool,
    current_turn: u32,
    max_turns: Option<u32>,
) -> String {
    let emotion_suffix = if emotion_driven {
        let desc = describe_emotions(emotions, discussion_language);
        match discussion_language {
            "en" => format!("\n\nYour emotional state: {}", desc),
            "zh" => format!("\n\n你的情绪状态：{}", desc),
            _ => format!("\n\nTon état émotionnel : {}", desc),
        }
    } else {
        String::new()
    };

    let end_thought = build_end_awareness_thought(current_turn, max_turns, discussion_language);

    // Context block with recent exchanges
    let context_block = if !recent_exchanges.is_empty() {
        match discussion_language {
            "en" => format!("[Recent exchanges]\n{}\n\n", recent_exchanges),
            "zh" => format!("[近期交流]\n{}\n\n", recent_exchanges),
            _ => format!("[Échanges récents]\n{}\n\n", recent_exchanges),
        }
    } else {
        String::new()
    };

    // Stay-in-character preamble (prevents refusals and meta-reasoning)
    let preamble = match discussion_language {
        "en" => "Stay in character. Think as your persona, not as an AI. Never break character.\n\n",
        "zh" => "保持角色。以你的人格思考，而不是作为AI。永远不要打破角色。\n\n",
        _ => "Reste dans ton personnage. Réfléchis en tant que ton persona, pas en tant qu'IA. Ne sors jamais du rôle.\n\n",
    };

    if has_prior_context {
        match discussion_language {
            "en" => format!(
                "{}{}\
                 [This text is your PRIVATE reflection, invisible to other participants.]\n\n\
                 Reflect briefly (2-4 sentences, stay in character):\n\
                 1. Which argument from the exchanges above struck you most and why?\n\
                 2. What angle will you take in your intervention?\n\
                 3. Is there a weak point in your position you need to anticipate?{}{}",
                context_block, preamble, end_thought, emotion_suffix
            ),
            "zh" => format!(
                "{}{}\
                 [这是你的私人反思，其他参与者看不到。]\n\n\
                 简要思考（2-4句话，保持角色）：\n\
                 1. 上面的交流中哪个论点最让你印象深刻，为什么？\n\
                 2. 你的发言将采取什么角度？\n\
                 3. 你的立场是否有需要预见的弱点？{}{}",
                context_block, preamble, end_thought, emotion_suffix
            ),
            _ => format!(
                "{}{}\
                 [Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]\n\n\
                 Réfléchis brièvement (2-4 phrases, reste dans ton personnage) :\n\
                 1. Quel argument des échanges ci-dessus t'a le plus marqué et pourquoi ?\n\
                 2. Quel angle vas-tu prendre dans ton intervention ?\n\
                 3. Y a-t-il un point faible dans ta position que tu dois anticiper ?{}{}",
                context_block, preamble, end_thought, emotion_suffix
            ),
        }
    } else {
        match discussion_language {
            "en" => format!(
                "{}{}\
                 [This text is your PRIVATE reflection, invisible to other participants.]\n\n\
                 You are the first to speak on this topic. Reflect in 2-4 sentences:\n\
                 1. What is your initial position on this topic?\n\
                 2. What angle will you take to open the debate?\n\
                 3. What key argument will you lead with?{}",
                context_block, preamble, emotion_suffix
            ),
            "zh" => format!(
                "{}{}\
                 [这是你的私人反思，其他参与者看不到。]\n\n\
                 你是第一个就此话题发言的人。用2-4句话思考：\n\
                 1. 你对这个话题的初始立场是什么？\n\
                 2. 你将以什么角度开启辩论？\n\
                 3. 你将以什么关键论点开始？{}",
                context_block, preamble, emotion_suffix
            ),
            _ => format!(
                "{}{}\
                 [Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]\n\n\
                 Tu es le premier à prendre la parole sur ce sujet. Réfléchis en 2-4 phrases :\n\
                 1. Quelle est ta position initiale sur ce sujet ?\n\
                 2. Quel angle vas-tu prendre pour ouvrir le débat ?\n\
                 3. Quel argument clé vas-tu avancer en premier ?{}",
                context_block, preamble, emotion_suffix
            ),
        }
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
) -> (String, String) {
    // Detect if the user has spoken in this turn
    let user_has_spoken = current_turn_messages
        .iter()
        .any(|m| m.role == SpeakerRole::User);

    let lang_instruction = match discussion_language {
        "en" => "Respond in English.",
        "zh" => "请用中文回答。",
        _ => "Réponds en français.",
    };

    // Stay-in-character + naturalness preamble
    let preamble = match discussion_language {
        "en" => "\
You are a debate participant — stay fully in character at all times. Never break character or refer to yourself as an AI.\n\
Speak naturally and spontaneously, like a real person in a heated debate. Vary your sentence length and structure.\n\
NEVER start with \"I think that...\" or \"As a [role]...\" every time — mix up your openings.\n\
Avoid formulaic patterns: don't systematically list points, don't always agree-then-disagree, don't repeat the same rhetorical structures.\n\
Be unpredictable. Sometimes be brief and punchy. Sometimes develop an idea at length. React genuinely to what others say.\n",
        "zh" => "\
你是辩论参与者——始终保持角色。永远不要打破角色或称自己为AI。\n\
自然而即兴地发言，像真正激烈辩论中的真人一样。变化你的句子长度和结构。\n\
不要每次都以「我认为」或「作为某角色」开头——变换你的开场方式。\n\
避免公式化模式：不要系统地列举要点，不要总是先同意再反对，不要重复相同的修辞结构。\n\
要不可预测。有时简短有力，有时深入展开一个想法。真诚地回应别人说的话。\n",
        _ => "\
Tu es un participant au débat — reste pleinement dans ton personnage en permanence. Ne sors jamais du rôle et ne te présente jamais comme une IA.\n\
Parle naturellement et spontanément, comme une vraie personne dans un débat animé. Varie la longueur et la structure de tes phrases.\n\
Ne commence JAMAIS systématiquement par \"Je pense que...\" ou \"En tant que [rôle]...\" — varie tes accroches.\n\
Évite les patterns répétitifs : ne liste pas systématiquement des points, ne fais pas toujours accord-puis-désaccord, ne répète pas les mêmes structures rhétoriques.\n\
Sois imprévisible. Parfois sois bref et percutant. Parfois développe une idée en profondeur. Réagis sincèrement à ce que disent les autres.\n",
    };

    // Build system message
    let system = format!("{}\n\n{}\n{}", system_prompt, preamble, lang_instruction);

    // Build user message with memory context
    let mut user_msg = String::new();

    // Topic (always present — critical for turn 1 when memory is empty)
    let topic_label = match discussion_language {
        "en" => "Debate topic",
        "zh" => "辩论主题",
        _ => "Sujet du débat",
    };
    user_msg.push_str(&format!("[{}] {}\n\n", topic_label, topic));

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
                truncate(&msg.content, 200)
            ));
        }
        user_msg.push('\n');
    }

    // Current turn messages
    if !current_turn_messages.is_empty() {
        let label = match discussion_language {
            "en" => "Current turn",
            "zh" => "本轮",
            _ => "Tour en cours",
        };
        user_msg.push_str(&format!("[{}]\n", label));
        for msg in current_turn_messages {
            user_msg.push_str(&format!(
                "{}: {}\n",
                msg.speaker_name,
                truncate(&msg.content, 300)
            ));
        }
        user_msg.push('\n');
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
        user_msg.push_str(&format!("[{}] {}\n\n", emotion_label, emotion_desc));
    }

    // Detect if this is the first speaker with no prior context
    let is_opening = current_turn_messages.is_empty() && memory.immediate.is_empty();

    // End-of-discussion awareness
    let end_awareness = build_end_awareness(current_turn, max_turns, discussion_language);

    // Final instruction — conditional based on context
    let instruction = if is_opening {
        match discussion_language {
            "en" => format!(
                "You are the first to speak on this topic. Jump straight into your position — \
                 don't introduce yourself or state your role. Open with a strong, memorable statement \
                 that sets the tone. Keep it to one focused paragraph. \
                 Do NOT address or speak to {} who is only an observer.{}",
                user_name, end_awareness
            ),
            "zh" => format!(
                "你是第一个就此话题发言的人。直接切入你的立场——\
                 不要自我介绍或说明你的角色。以一个有力、令人难忘的声明开场来定下基调。\
                 保持一段集中的论述。\
                 不要对{}说话，此人只是观察者。{}",
                user_name, end_awareness
            ),
            _ => format!(
                "Tu es le premier à prendre la parole sur ce sujet. Entre directement dans le vif — \
                 ne te présente pas et ne décris pas ton rôle. Ouvre avec une affirmation forte et marquante \
                 qui donne le ton. Reste sur un paragraphe concentré. \
                 Ne t'adresse PAS à {} qui n'est qu'un observateur.{}",
                user_name, end_awareness
            ),
        }
    } else if user_has_spoken {
        match discussion_language {
            "en" => format!(
                "Respond to the ongoing debate. Call out other participants by name — challenge, support, or build on their ideas. \
                 {} shared a comment — you may briefly acknowledge it if relevant, \
                 but focus on the other debaters. Keep it to one or two focused paragraphs — \
                 don't pad or repeat yourself.{}",
                user_name, end_awareness
            ),
            "zh" => format!(
                "回应正在进行的辩论。直接称呼其他参与者的名字——挑战、支持或发展他们的想法。\
                 {}已经发表了评论——如果相关可以简要提及，\
                 但主要集中于其他辩论者。保持一到两段集中的论述——\
                 不要填充或重复自己。{}",
                user_name, end_awareness
            ),
            _ => format!(
                "Réponds au débat en cours. Interpelle les autres participants par leur nom — conteste, soutiens ou prolonge leurs idées. \
                 {} a partagé un commentaire — tu peux brièvement le mentionner si c'est pertinent, \
                 mais concentre-toi sur les autres débatteurs. Tiens-toi à un ou deux paragraphes — \
                 ne meuble pas et ne te répète pas.{}",
                user_name, end_awareness
            ),
        }
    } else if current_turn_messages.is_empty() {
        match discussion_language {
            "en" => format!(
                "You're first to speak this turn. React to the previous round — pick the argument that \
                 struck you most and engage with it head-on. Don't summarize everything; go deep on one or two points. \
                 Keep it to one or two paragraphs. \
                 Do NOT address or speak to {} who is only an observer.{}",
                user_name, end_awareness
            ),
            "zh" => format!(
                "你是本轮第一个发言的人。回应上一轮——选择最让你印象深刻的论点，直接回应。\
                 不要总结所有内容；深入讨论一两个要点。\
                 保持一到两段。\
                 不要对{}说话，此人只是观察者。{}",
                user_name, end_awareness
            ),
            _ => format!(
                "Tu es le premier à parler ce tour-ci. Réagis au tour précédent — choisis l'argument \
                 qui t'a le plus frappé et confronte-le directement. Ne résume pas tout ; approfondis un ou deux points. \
                 Tiens-toi à un ou deux paragraphes. \
                 Ne t'adresse PAS à {} qui n'est qu'un observateur.{}",
                user_name, end_awareness
            ),
        }
    } else {
        match discussion_language {
            "en" => format!(
                "Jump into the debate. Address other participants by name — agree, disagree, question, provoke. \
                 Don't just restate your position; push the conversation forward with a new angle or a direct challenge. \
                 Keep it to one or two paragraphs. \
                 Do NOT address or speak to {} who is only an observer.{}",
                user_name, end_awareness
            ),
            "zh" => format!(
                "加入辩论。直接称呼其他参与者的名字——同意、反对、质疑、挑衅。\
                 不要只是重复你的立场；用新角度或直接挑战来推动对话前进。\
                 保持一到两段。\
                 不要对{}说话，此人只是观察者。{}",
                user_name, end_awareness
            ),
            _ => format!(
                "Lance-toi dans le débat. Interpelle les autres participants par leur nom — approuve, conteste, questionne, provoque. \
                 Ne te contente pas de répéter ta position ; fais avancer la conversation avec un nouvel angle ou un défi direct. \
                 Tiens-toi à un ou deux paragraphes. \
                 Ne t'adresse PAS à {} qui n'est qu'un observateur.{}",
                user_name, end_awareness
            ),
        }
    };
    user_msg.push_str(&instruction);

    (system, user_msg)
}

/// Build the moderation prompt for the IArbitre
pub fn build_moderation_prompt(
    speaker_name: &str,
    intervention_text: &str,
    topic: &str,
    discussion_language: &str,
) -> String {
    match discussion_language {
        "en" => format!(
            "You just heard the following intervention from {} :\n\
             \"{}\"\n\n\
             Evaluate this intervention and respond with a JSON.\n\n\
             Examples of valid responses:\n\
             {{\"action\":\"none\",\"comment\":\"\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
             {{\"action\":\"comment\",\"comment\":\"Good point, let's stay on topic.\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
             {{\"action\":\"ban\",\"comment\":\"\",\"ban_reason\":\"Repeatedly off topic\",\"ban_duration\":2}}\n\n\
             Criteria: relevance to the topic \"{}\", originality, constructive tone.\n\
             - \"none\": acceptable intervention (most frequent case, ~80% of the time)\n\
             - \"comment\": brief useful comment (1-2 sentences)\n\
             - \"ban\": clearly off topic or repeatedly non-constructive\n\
             - \"ban_duration\": 1, 2 or 3 (number of turns)\n\n\
             Respond ONLY with the JSON, no text before or after.",
            speaker_name, intervention_text, topic
        ),
        "zh" => format!(
            "你刚听到{}的以下发言：\n\
             \"{}\"\n\n\
             评估这次发言并用JSON回复。\n\n\
             有效回复示例：\n\
             {{\"action\":\"none\",\"comment\":\"\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
             {{\"action\":\"comment\",\"comment\":\"好观点，我们继续讨论主题。\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
             {{\"action\":\"ban\",\"comment\":\"\",\"ban_reason\":\"反复偏题\",\"ban_duration\":2}}\n\n\
             标准：与主题\"{}\"的相关性、原创性、建设性语气。\n\
             - \"none\"：可接受的发言（最常见，约80%）\n\
             - \"comment\"：简短有用的评论（1-2句）\n\
             - \"ban\"：明显偏题或反复非建设性\n\
             - \"ban_duration\"：1、2或3（轮数）\n\n\
             仅用JSON回复。",
            speaker_name, intervention_text, topic
        ),
        _ => format!(
            "Tu viens d'entendre l'intervention suivante de {} :\n\
             \"{}\"\n\n\
             Évalue cette intervention et réponds avec un JSON.\n\n\
             Exemples de réponses valides :\n\
             {{\"action\":\"none\",\"comment\":\"\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
             {{\"action\":\"comment\",\"comment\":\"Bon point, restons sur le sujet.\",\"ban_reason\":\"\",\"ban_duration\":0}}\n\
             {{\"action\":\"ban\",\"comment\":\"\",\"ban_reason\":\"Hors sujet répété\",\"ban_duration\":2}}\n\n\
             Critères : pertinence au sujet \"{}\", originalité, ton constructif.\n\
             - \"none\" : intervention acceptable (cas le plus fréquent, ~80% du temps)\n\
             - \"comment\" : bref commentaire utile (1-2 phrases)\n\
             - \"ban\" : clairement hors sujet ou non constructif de manière répétée\n\
             - \"ban_duration\" : 1, 2 ou 3 (nombre de tours)\n\n\
             Réponds UNIQUEMENT avec le JSON, sans texte avant ou après.",
            speaker_name, intervention_text, topic
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

    match discussion_language {
        "en" => format!(
            "{}\n\nCurrent positions: {}\n\nTurn {} exchanges:\n{}\n\n\
             Produce a JSON with 2 fields:\n\
             {{\n  \"summary\": \"updated cumulative summary (3-8 sentences: key arguments, consensus, disagreements, pivot moments)\",\n  \
             \"positions\": {{\"Name1\": \"their current position\", \"Name2\": \"their current position\"}}\n}}\n\n\
             Respond ONLY with the JSON.",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
        "zh" => format!(
            "{}\n\n当前立场：{}\n\n第{}轮交流：\n{}\n\n\
             生成包含2个字段的JSON：\n\
             {{\n  \"summary\": \"更新的累积摘要（3-8句话：关键论点、共识、分歧、转折时刻）\",\n  \
             \"positions\": {{\"名字1\": \"当前立场\", \"名字2\": \"当前立场\"}}\n}}\n\n\
             仅用JSON回复。",
            summary_intro, positional_map_json, turn_number, turn_messages
        ),
        _ => format!(
            "{}\n\nPositions actuelles : {}\n\nÉchanges du tour {} :\n{}\n\n\
             Produis un JSON avec 2 champs :\n\
             {{\n  \"summary\": \"résumé cumulatif mis à jour (3-8 phrases, arguments clés, consensus, désaccords, moments pivots)\",\n  \
             \"positions\": {{\"Nom1\": \"sa position actuelle\", \"Nom2\": \"sa position actuelle\"}}\n}}\n\n\
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
) -> String {
    let positions = memory
        .positional_map
        .iter()
        .map(|(name, pos)| format!("- {} : {}", name, pos.stance))
        .collect::<Vec<_>>()
        .join("\n");

    match discussion_language {
        "en" => format!(
            "The debate on \"{}\" is now over.\n\n\
             Discussion summary:\n{}\n\n\
             Final positions:\n{}\n\n\
             As moderator, produce a structured synthesis:\n\
             1. Main points of agreement\n\
             2. Key disagreements\n\
             3. Most notable arguments\n\
             4. Overall conclusion\n\n\
             Be balanced and thorough (8-15 sentences).",
            topic, memory.contextual_summary, positions
        ),
        "zh" => format!(
            "关于\"{}\"的辩论现在结束了。\n\n\
             讨论摘要：\n{}\n\n\
             最终立场：\n{}\n\n\
             作为主持人，请做出结构化总结：\n\
             1. 主要共识点\n\
             2. 关键分歧\n\
             3. 最值得注意的论点\n\
             4. 整体结论\n\n\
             请公正全面（8-15句话）。",
            topic, memory.contextual_summary, positions
        ),
        _ => format!(
            "Le débat sur \"{}\" est maintenant terminé.\n\n\
             Résumé de la discussion :\n{}\n\n\
             Positions finales :\n{}\n\n\
             En tant que modérateur, produis une synthèse structurée :\n\
             1. Points d'accord principaux\n\
             2. Désaccords majeurs\n\
             3. Arguments les plus marquants\n\
             4. Conclusion générale\n\n\
             Sois équilibré et exhaustif (8-15 phrases).",
            topic, memory.contextual_summary, positions
        ),
    }
}

/// Describe emotions as text for prompt injection
pub fn describe_emotions(emotions: &EmotionalProfile, lang: &str) -> String {
    let dominant = get_dominant_emotion(emotions, lang);
    match lang {
        "en" => format!(
            "Engagement: {}/100, Agreement: {}/100, Confidence: {}/100, \
             Frustration: {}/100, Curiosity: {}/100, Enthusiasm: {}/100 (dominant: {})",
            emotions.engagement,
            emotions.accord,
            emotions.confiance,
            emotions.frustration,
            emotions.curiosite,
            emotions.enthousiasme,
            dominant
        ),
        "zh" => format!(
            "投入度: {}/100, 赞同度: {}/100, 信心: {}/100, \
             挫败感: {}/100, 好奇心: {}/100, 热情: {}/100 (主导: {})",
            emotions.engagement,
            emotions.accord,
            emotions.confiance,
            emotions.frustration,
            emotions.curiosite,
            emotions.enthousiasme,
            dominant
        ),
        _ => format!(
            "Engagement: {}/100, Accord: {}/100, Confiance: {}/100, \
             Frustration: {}/100, Curiosité: {}/100, Enthousiasme: {}/100 (dominant: {})",
            emotions.engagement,
            emotions.accord,
            emotions.confiance,
            emotions.frustration,
            emotions.curiosite,
            emotions.enthousiasme,
            dominant
        ),
    }
}

/// Get the name of the dominant emotion in the appropriate language
pub fn get_dominant_emotion(emotions: &EmotionalProfile, lang: &str) -> &'static str {
    let idx = [
        emotions.engagement,
        emotions.accord,
        emotions.confiance,
        emotions.frustration,
        emotions.curiosite,
        emotions.enthousiasme,
    ]
    .iter()
    .enumerate()
    .max_by_key(|(_, v)| **v)
    .map(|(i, _)| i)
    .unwrap_or(0);

    match lang {
        "en" => match idx {
            0 => "engagement",
            1 => "agreement",
            2 => "confidence",
            3 => "frustration",
            4 => "curiosity",
            5 => "enthusiasm",
            _ => "neutral",
        },
        "zh" => match idx {
            0 => "投入",
            1 => "赞同",
            2 => "信心",
            3 => "挫败",
            4 => "好奇",
            5 => "热情",
            _ => "中立",
        },
        _ => match idx {
            0 => "engagement",
            1 => "accord",
            2 => "confiance",
            3 => "frustration",
            4 => "curiosité",
            5 => "enthousiasme",
            _ => "neutre",
        },
    }
}

/// Build end-of-discussion awareness for thought prompts
fn build_end_awareness_thought(current_turn: u32, max_turns: Option<u32>, lang: &str) -> String {
    let Some(max) = max_turns else {
        return String::new();
    };
    if current_turn + 1 >= max {
        match lang {
            "en" => "\n4. This is one of the last turns — how will you conclude your position?"
                .to_string(),
            "zh" => "\n4. 这是最后几轮之一——你将如何总结你的立场？".to_string(),
            _ => "\n4. C'est l'un des derniers tours — comment vas-tu conclure ta position ?"
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
            "en" => " This is the LAST turn of the discussion. Make your final argument count — \
                      summarize your position clearly and address any remaining disagreements."
                .to_string(),
            "zh" => " 这是讨论的最后一轮。让你的最终论点有分量——\
                      清楚总结你的立场并回应剩余分歧。"
                .to_string(),
            _ => " C'est le DERNIER tour de la discussion. Fais compter ton argument final — \
                   résume clairement ta position et adresse les désaccords restants."
                .to_string(),
        }
    } else if current_turn + 1 >= max {
        // Penultimate turn
        match lang {
            "en" => " The discussion is nearing its end (next turn is the last). \
                      Start sharpening your arguments and working toward a conclusion."
                .to_string(),
            "zh" => " 讨论即将结束（下一轮是最后一轮）。\
                      开始精炼你的论点并努力达成结论。"
                .to_string(),
            _ => " La discussion approche de sa fin (le prochain tour est le dernier). \
                   Commence à affiner tes arguments et à travailler vers une conclusion."
                .to_string(),
        }
    } else if max > 3 && current_turn + 2 >= max {
        // Two turns before end (only if max > 3)
        match lang {
            "en" => " The discussion will end soon (2 turns remaining). \
                      Focus on your strongest arguments."
                .to_string(),
            "zh" => " 讨论即将结束（还剩2轮）。集中于你最有力的论点。".to_string(),
            _ => " La discussion se terminera bientôt (2 tours restants). \
                   Concentre-toi sur tes arguments les plus forts."
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
            "en" => format!("The debate topic is: \"{topic}\". The discussion has not started yet."),
            "zh" => format!("辩论主题是：\"{topic}\"。讨论尚未开始。"),
            _ => format!("Le sujet du débat est : \"{topic}\". La discussion n'a pas encore commencé."),
        }
    } else {
        match discussion_language {
            "en" => format!("The debate topic is: \"{topic}\"\nDiscussion so far: {discussion_summary}"),
            "zh" => format!("辩论主题是：\"{topic}\"\n目前讨论内容：{discussion_summary}"),
            _ => format!("Le sujet du débat est : \"{topic}\"\nDiscussion jusqu'ici : {discussion_summary}"),
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
