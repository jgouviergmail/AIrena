use crate::models::emotion::EmotionalProfile;
use crate::models::memory::ParticipantMemory;
use crate::models::message::{Message, SpeakerRole};

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

    match discussion_language {
        "en" => format!(
            "Here are the interventions of OTHER participants in the previous turn:\n{}\n\n\
             For each intervention, choose your reaction:\n\
             - \"like\": agree or relevant argument\n\
             - \"dislike\": disagree or weak argument\n\
             - \"none\": neutral\n\n\
             Example: [{{\"speaker\":\"Alice\",\"reaction\":\"like\"}},{{\"speaker\":\"Bob\",\"reaction\":\"none\"}}]\n\n\
             Respond ONLY with the JSON.",
            list
        ),
        "zh" => format!(
            "以下是上一轮其他参与者的发言：\n{}\n\n\
             对每个发言选择你的反应：\n\
             - \"like\"：同意或有力论点\n\
             - \"dislike\"：不同意或薄弱论点\n\
             - \"none\"：中立\n\n\
             示例：[{{\"speaker\":\"Alice\",\"reaction\":\"like\"}},{{\"speaker\":\"Bob\",\"reaction\":\"none\"}}]\n\n\
             仅用JSON回复。",
            list
        ),
        _ => format!(
            "Voici les interventions des AUTRES participants au tour précédent :\n{}\n\n\
             Pour chaque intervention, choisis ta réaction :\n\
             - \"like\" : d'accord ou argument pertinent\n\
             - \"dislike\" : en désaccord ou argument faible\n\
             - \"none\" : neutre\n\n\
             Exemple : [{{\"speaker\":\"Alice\",\"reaction\":\"like\"}},{{\"speaker\":\"Bob\",\"reaction\":\"none\"}}]\n\n\
             Réponds UNIQUEMENT avec le JSON.",
            list
        ),
    }
}

/// Build the inner thought prompt
pub fn build_thought_prompt(
    emotions: &EmotionalProfile,
    discussion_language: &str,
    has_prior_context: bool,
    emotion_driven: bool,
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

    if has_prior_context {
        match discussion_language {
            "en" => format!(
                "[This text is your PRIVATE reflection, invisible to other participants.]\n\n\
                 Reflect in 2-4 sentences:\n\
                 1. Which argument from the last turn struck you most and why?\n\
                 2. What angle will you take in your intervention?\n\
                 3. Is there a weak point in your position you need to anticipate?{}",
                emotion_suffix
            ),
            "zh" => format!(
                "[这是你的私人反思，其他参与者看不到。]\n\n\
                 用2-4句话思考：\n\
                 1. 上一轮哪个论点最让你印象深刻，为什么？\n\
                 2. 你的发言将采取什么角度？\n\
                 3. 你的立场是否有需要预见的弱点？{}",
                emotion_suffix
            ),
            _ => format!(
                "[Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]\n\n\
                 Réfléchis en 2-4 phrases :\n\
                 1. Quel argument du dernier tour t'a le plus marqué et pourquoi ?\n\
                 2. Quel angle vas-tu prendre dans ton intervention ?\n\
                 3. Y a-t-il un point faible dans ta position que tu dois anticiper ?{}",
                emotion_suffix
            ),
        }
    } else {
        match discussion_language {
            "en" => format!(
                "[This text is your PRIVATE reflection, invisible to other participants.]\n\n\
                 You are the first to speak on this topic. Reflect in 2-4 sentences:\n\
                 1. What is your initial position on this topic?\n\
                 2. What angle will you take to open the debate?\n\
                 3. What key argument will you lead with?{}",
                emotion_suffix
            ),
            "zh" => format!(
                "[这是你的私人反思，其他参与者看不到。]\n\n\
                 你是第一个就此话题发言的人。用2-4句话思考：\n\
                 1. 你对这个话题的初始立场是什么？\n\
                 2. 你将以什么角度开启辩论？\n\
                 3. 你将以什么关键论点开始？{}",
                emotion_suffix
            ),
            _ => format!(
                "[Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]\n\n\
                 Tu es le premier à prendre la parole sur ce sujet. Réfléchis en 2-4 phrases :\n\
                 1. Quelle est ta position initiale sur ce sujet ?\n\
                 2. Quel angle vas-tu prendre pour ouvrir le débat ?\n\
                 3. Quel argument clé vas-tu avancer en premier ?{}",
                emotion_suffix
            ),
        }
    }
}

/// Build the main intervention prompt for a gladiator
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

    // Build system message
    let system = format!("{}\n\n{}", system_prompt, lang_instruction);

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

    // Final instruction — conditional based on context
    let instruction = if is_opening {
        // First speaker, no prior arguments — ask to present initial position
        match discussion_language {
            "en" => format!(
                "You are the first to speak on this topic. Present your initial position clearly (3-6 sentences). \
                 State your main argument and set the tone for the debate. \
                 Do NOT address or speak to {} who is only an observer.",
                user_name
            ),
            "zh" => format!(
                "你是第一个就此话题发言的人。清楚地阐述你的初始立场（3-6句话）。\
                 陈述你的主要论点并为辩论定下基调。\
                 不要对{}说话，此人只是观察者。",
                user_name
            ),
            _ => format!(
                "Tu es le premier à prendre la parole sur ce sujet. Présente clairement ta position initiale (3-6 phrases). \
                 Expose ton argument principal et donne le ton du débat. \
                 Ne t'adresse PAS à {} qui n'est qu'un observateur.",
                user_name
            ),
        }
    } else if user_has_spoken {
        match discussion_language {
            "en" => format!(
                "Now give your intervention on the topic. Be concise (3-6 sentences). \
                 Address other participants by name and debate with them directly. \
                 {} has shared a comment — you may briefly acknowledge it if relevant, \
                 but focus primarily on debating with the other participants.",
                user_name
            ),
            "zh" => format!(
                "现在就主题发表你的看法。请简洁（3-6句话）。\
                 直接称呼其他参与者的名字并与他们辩论。\
                 {}已经发表了评论——如果相关可以简要提及，\
                 但主要集中于与其他参与者辩论。",
                user_name
            ),
            _ => format!(
                "Donne maintenant ton intervention sur le sujet. Sois concis (3-6 phrases). \
                 Adresse-toi aux autres participants par leur nom et débats directement avec eux. \
                 {} a partagé un commentaire — tu peux brièvement le mentionner si c'est pertinent, \
                 mais concentre-toi principalement sur le débat avec les autres participants.",
                user_name
            ),
        }
    } else if current_turn_messages.is_empty() {
        // First in this turn, but has memory from previous turns — debate normally
        match discussion_language {
            "en" => format!(
                "Now give your intervention on the topic. Be concise (3-6 sentences). \
                 You are the first to speak this turn. React to the previous turn's arguments \
                 and present your perspective. \
                 Do NOT address or speak to {} who is only an observer.",
                user_name
            ),
            "zh" => format!(
                "现在就主题发表你的看法。请简洁（3-6句话）。\
                 你是本轮第一个发言的人。回应上一轮的论点并阐述你的观点。\
                 不要对{}说话，此人只是观察者。",
                user_name
            ),
            _ => format!(
                "Donne maintenant ton intervention sur le sujet. Sois concis (3-6 phrases). \
                 Tu es le premier à parler ce tour-ci. Réagis aux arguments du tour précédent \
                 et expose ton point de vue. \
                 Ne t'adresse PAS à {} qui n'est qu'un observateur.",
                user_name
            ),
        }
    } else {
        // Normal case: other speakers have already spoken this turn
        match discussion_language {
            "en" => format!(
                "Now give your intervention on the topic. Be concise (3-6 sentences). \
                 Address other participants by name and debate with them directly. \
                 Do NOT address or speak to {} who is only an observer. \
                 Focus on responding to the other debaters' arguments.",
                user_name
            ),
            "zh" => format!(
                "现在就主题发表你的看法。请简洁（3-6句话）。\
                 直接称呼其他参与者的名字并与他们辩论。\
                 不要对{}说话，此人只是观察者。\
                 专注于回应其他辩论者的论点。",
                user_name
            ),
            _ => format!(
                "Donne maintenant ton intervention sur le sujet. Sois concis (3-6 phrases). \
                 Adresse-toi aux autres participants par leur nom et débats directement avec eux. \
                 Ne t'adresse PAS à {} qui n'est qu'un observateur. \
                 Concentre-toi sur les arguments des autres débatteurs.",
                user_name
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

fn truncate(s: &str, max_chars: usize) -> &str {
    &s[..s.floor_char_boundary(max_chars)]
}
