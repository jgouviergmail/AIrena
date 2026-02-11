use crate::engine::emotion_engine::{HIGH_THRESHOLD, LOW_THRESHOLD};
use crate::models::emotion::EmotionalProfile;
use crate::models::memory::ParticipantMemory;
use crate::models::message::{Message, SpeakerRole};
use crate::tavily::TavilySearchResponse;
use crate::wikipedia::WikiSearchResponse;

use super::truncate_str as truncate;

/// Maximum character length for web search context injected into prompts.
const MAX_SEARCH_CONTEXT_LEN: usize = 2000;

/// Build the introduction prompt for the IArbitre
pub fn build_introduction_prompt(
    topic: &str,
    participant_names: &[String],
    discussion_language: &str,
    web_search_results: Option<&str>,
) -> String {
    let participants = participant_names.join(", ");
    let datetime = build_datetime_context(discussion_language);
    let web_block = web_search_results
        .map(|r| {
            let instruction = match discussion_language {
                "en" => "\n⚡ Only use results that are relevant to the debate topic. Ignore off-topic results. \
                         Weave key facts or current data naturally into your introduction to frame the topic. Do NOT list them raw.",
                "zh" => "\n⚡ 只使用与辩论主题相关的结果。忽略偏题的结果。\
                         将关键事实或当前数据自然地融入你的介绍中来构建主题框架。不要原样列举。",
                _ => "\n⚡ N'utilise que les résultats pertinents par rapport au sujet du débat. Ignore les résultats hors-sujet. \
                         Intègre naturellement des faits clés ou des données actuelles dans ton introduction pour cadrer le sujet. Ne les liste PAS tels quels.",
            };
            format!("\n\n{}{}", r, instruction)
        })
        .unwrap_or_default();
    match discussion_language {
        "en" => format!(
            "{}\n\nYou are the moderator of a debate. The topic is: \"{}\"\n\
             The participants are: {}{}\n\n\
             Introduce the topic in a BROAD and NEUTRAL manner (2-3 sentences), covering the key dimensions \
             and perspectives of the subject without narrowing it to a single angle or your personal bias. \
             Then invite the first participant to speak.\n\
             IMPORTANT RULE: Clearly remind participants that this first round is for presenting \
             initial opinions only — each speaker must share their own position on the topic. \
             Debating, challenging, or responding to others' arguments is NOT allowed until the second round.\n\
             Respond in English.",
            datetime, topic, participants, web_block
        ),
        "zh" => format!(
            "{}\n\n你是一场辩论的主持人。主题是：\"{}\"\n\
             参与者有：{}{}\n\n\
             以广泛且中立的方式简要介绍主题（2-3句话），涵盖该主题的主要方面和视角，\
             不要将其缩小为单一角度或个人偏见。然后邀请第一位参与者发言。\n\
             重要规则：明确提醒参与者，第一轮仅用于表达初始观点——每位发言者必须分享自己对主题的立场。\
             在第二轮之前，不得辩论、质疑或回应他人的论点。\n\
             请用中文回答。",
            datetime, topic, participants, web_block
        ),
        _ => format!(
            "{}\n\nTu es le modérateur d'un débat. Le sujet est : \"{}\"\n\
             Les participants sont : {}{}\n\n\
             Présente le sujet de manière LARGE et NEUTRE (2-3 phrases), en couvrant les dimensions \
             et perspectives clés du sujet sans le réduire à un seul angle ou à ton biais personnel. \
             Puis invite le premier participant à prendre la parole.\n\
             RÈGLE IMPORTANTE : Rappelle clairement aux participants que ce premier tour est réservé \
             à l'expression des opinions initiales — chaque intervenant doit présenter sa propre position sur le sujet. \
             Il est INTERDIT de débattre, contester ou répondre aux arguments des autres avant le deuxième tour.\n\
             Réponds en français.",
            datetime, topic, participants, web_block
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
             One reaction per participant only — do NOT react twice to the same speaker.\n\
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
             每个参与者只能有一个反应——不要对同一发言者反应两次。\n\
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
             Une seule réaction par participant — ne réagis PAS deux fois au même intervenant.\n\
             Format attendu : {}\n\n\
             Réponds UNIQUEMENT avec le tableau JSON.",
            list, example
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
                "en" => "\n⚡ First assess: are these results actually relevant to the debate and the current exchange? \
                         Discard anything off-topic. Then identify which specific facts you can weave into your argument.",
                "zh" => "\n⚡ 首先评估：这些结果是否真的与辩论和当前交流相关？丢弃任何偏题的内容。\
                         然后找出哪些具体事实可以融入你的论证。",
                _ => "\n⚡ Évalue d'abord : ces résultats sont-ils vraiment pertinents pour le débat et l'échange en cours ? \
                         Écarte tout ce qui est hors-sujet. Puis identifie quels faits précis tu peux intégrer dans ton argumentation.",
            };
            format!("\n\n{}{}", r, instruction)
        })
        .unwrap_or_default();

    let end_thought = build_end_awareness_thought(current_turn, max_turns, discussion_language);

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

    if has_prior_context {
        match discussion_language {
            "en" => format!(
                "{}{}\
                 [This text is your PRIVATE reflection, invisible to other participants.]\n\n\
                 Reflect briefly (2-4 sentences, stay in character):\n\
                 1. Which argument from the exchanges above struck you most and why?\n\
                 2. What angle will you take in your intervention?\n\
                 3. Is there a weak point in your position you need to anticipate?{}{}{}",
                context_block, preamble, end_thought, emotion_suffix, web_block
            ),
            "zh" => format!(
                "{}{}\
                 [这是你的私人反思，其他参与者看不到。]\n\n\
                 简要思考（2-4句话，保持角色）：\n\
                 1. 上面的交流中哪个论点最让你印象深刻，为什么？\n\
                 2. 你的发言将采取什么角度？\n\
                 3. 你的立场是否有需要预见的弱点？{}{}{}",
                context_block, preamble, end_thought, emotion_suffix, web_block
            ),
            _ => format!(
                "{}{}\
                 [Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]\n\n\
                 Réfléchis brièvement (2-4 phrases, reste dans ton personnage) :\n\
                 1. Quel argument des échanges ci-dessus t'a le plus marqué et pourquoi ?\n\
                 2. Quel angle vas-tu prendre dans ton intervention ?\n\
                 3. Y a-t-il un point faible dans ta position que tu dois anticiper ?{}{}{}",
                context_block, preamble, end_thought, emotion_suffix, web_block
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
                 3. What key argument will you lead with?{}{}",
                context_block, preamble, emotion_suffix, web_block
            ),
            "zh" => format!(
                "{}{}\
                 [这是你的私人反思，其他参与者看不到。]\n\n\
                 你是第一个就此话题发言的人。用2-4句话思考：\n\
                 1. 你对这个话题的初始立场是什么？\n\
                 2. 你将以什么角度开启辩论？\n\
                 3. 你将以什么关键论点开始？{}{}",
                context_block, preamble, emotion_suffix, web_block
            ),
            _ => format!(
                "{}{}\
                 [Ce texte est ta réflexion PRIVÉE, invisible des autres participants.]\n\n\
                 Tu es le premier à prendre la parole sur ce sujet. Réfléchis en 2-4 phrases :\n\
                 1. Quelle est ta position initiale sur ce sujet ?\n\
                 2. Quel angle vas-tu prendre pour ouvrir le débat ?\n\
                 3. Quel argument clé vas-tu avancer en premier ?{}{}",
                context_block, preamble, emotion_suffix, web_block
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
    web_search_results: Option<&str>,
    participant_names: &[String],
    dynamic_directive: Option<&str>,
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
Be unpredictable. Sometimes be brief and punchy. Sometimes develop an idea at length. React genuinely to what others say.\n\
CRITICAL: NEVER refer to yourself in the third person. You speak in first person (\"I\", \"me\", \"my\"). Never quote or comment on yourself as if you were someone else.\n\
Your verbal tics are OCCASIONAL punctuations, not crutches. Use them at most once per intervention, never at the beginning of a sentence.\n",
        "zh" => "\
你是辩论参与者——始终保持角色。永远不要打破角色或称自己为AI。\n\
自然而即兴地发言，像真正激烈辩论中的真人一样。变化你的句子长度和结构。\n\
不要每次都以「我认为」或「作为某角色」开头——变换你的开场方式。\n\
避免公式化模式：不要系统地列举要点，不要总是先同意再反对，不要重复相同的修辞结构。\n\
要不可预测。有时简短有力，有时深入展开一个想法。真诚地回应别人说的话。\n\
关键：永远不要用第三人称提到自己。你用第一人称（「我」、「我的」）说话。永远不要像谈论别人一样引用或评论自己。\n\
你的口头禅是偶尔的点缀，不是拐杖。每次发言最多使用一次，绝不放在句首。\n",
        _ => "\
Tu es un participant au débat — reste pleinement dans ton personnage en permanence. Ne sors jamais du rôle et ne te présente jamais comme une IA.\n\
Parle naturellement et spontanément, comme une vraie personne dans un débat animé. Varie la longueur et la structure de tes phrases.\n\
Ne commence JAMAIS systématiquement par \"Je pense que...\" ou \"En tant que [rôle]...\" — varie tes accroches.\n\
Évite les patterns répétitifs : ne liste pas systématiquement des points, ne fais pas toujours accord-puis-désaccord, ne répète pas les mêmes structures rhétoriques.\n\
Sois imprévisible. Parfois sois bref et percutant. Parfois développe une idée en profondeur. Réagis sincèrement à ce que disent les autres.\n\
CRITIQUE : Ne te réfère JAMAIS à toi-même à la troisième personne. Tu parles à la première personne (\"je\", \"moi\", \"mon\"). Ne te cite pas et ne te commente pas comme si tu étais quelqu'un d'autre.\n\
Tes tics verbaux sont des ponctuations OCCASIONNELLES, pas des béquilles. Utilise-les au maximum 1 fois par intervention, jamais en début de phrase.\n",
    };

    // Build system message
    let system = format!("{}\n\n{}\n{}", system_prompt, preamble, lang_instruction);

    // Build user message with memory context
    let mut user_msg = String::new();

    // Date/time context
    user_msg.push_str(&build_datetime_context(discussion_language));
    user_msg.push_str("\n\n");

    // Topic (always present — critical for turn 1 when memory is empty)
    let topic_label = match discussion_language {
        "en" => "Debate topic",
        "zh" => "辩论主题",
        _ => "Sujet du débat",
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
            "en" => "⚡ CRITICAL — Be SELECTIVE with these results: first verify they are relevant to the debate topic \
                     and the current exchange. If a result is off-topic or incorrect, IGNORE it completely. \
                     For relevant results: weave specific facts, data, or references naturally into YOUR OWN reasoning \
                     to support or challenge arguments. Do NOT restate or list them — integrate them as a knowledgeable \
                     debater would cite a source mid-argument.",
            "zh" => "⚡ 关键——对这些结果要有选择性：首先验证它们是否与辩论主题和当前交流相关。\
                     如果结果偏题或不正确，完全忽略它。\
                     对于相关结果：将具体事实、数据或参考资料自然地融入你自己的推理中，\
                     以支持或质疑论点。不要重述或列举——像一个博学的辩论者在论证中引用资料一样整合它们。",
            _ => "⚡ CRITIQUE — Sois SÉLECTIF avec ces résultats : vérifie d'abord qu'ils sont pertinents par rapport \
                     au sujet du débat et à l'échange en cours. Si un résultat est hors-sujet ou incorrect, IGNORE-le complètement. \
                     Pour les résultats pertinents : intègre des faits, données ou références précises naturellement \
                     dans TON PROPRE raisonnement pour appuyer ou contester des arguments. \
                     Ne les recopie PAS et ne les liste PAS — cite-les comme un débatteur cultivé le ferait en pleine argumentation.",
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

    // Current turn messages — separate IArbitre directives for emphasis
    if !current_turn_messages.is_empty() {
        let label = match discussion_language {
            "en" => "Current turn",
            "zh" => "本轮",
            _ => "Tour en cours",
        };
        // Collect IArbitre directives separately
        let (arbitre_msgs, other_msgs): (Vec<_>, Vec<_>) = current_turn_messages
            .iter()
            .partition(|m| m.role == SpeakerRole::Arbitre);

        if !other_msgs.is_empty() {
            user_msg.push_str(&format!("[{}]\n", label));
            for msg in &other_msgs {
                user_msg.push_str(&format!(
                    "{}: {}\n",
                    msg.speaker_name,
                    truncate(&msg.content, 300)
                ));
            }
            user_msg.push('\n');
        }

        // IArbitre moderation directives — emphasized section
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
                    truncate(&msg.content, 500)
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
    } else if is_opening {
        match discussion_language {
            "en" => format!(
                "This is the OPENING ROUND — present your initial opinion ONLY. \
                 Jump straight into your position — don't introduce yourself or state your role. \
                 Open with a strong, memorable statement that sets the tone. \
                 Do NOT debate, challenge, or respond to others — this will begin in the next round. \
                 Keep it to one focused paragraph. \
                 Do NOT address or speak to {} who is only an observer.{}",
                user_name, end_awareness
            ),
            "zh" => format!(
                "这是开场轮——仅表达你的初始观点。\
                 直接切入你的立场——不要自我介绍或说明你的角色。\
                 以一个有力、令人难忘的声明开场来定下基调。\
                 不要辩论、质疑或回应他人——这将在下一轮开始。\
                 保持一段集中的论述。\
                 不要对{}说话，此人只是观察者。{}",
                user_name, end_awareness
            ),
            _ => format!(
                "C'est le TOUR D'OUVERTURE — présente uniquement ton opinion initiale. \
                 Entre directement dans le vif — ne te présente pas et ne décris pas ton rôle. \
                 Ouvre avec une affirmation forte et marquante qui donne le ton. \
                 Ne débats PAS, ne conteste PAS, ne réponds PAS aux autres — cela commencera au prochain tour. \
                 Reste sur un paragraphe concentré. \
                 Ne t'adresse PAS à {} qui n'est qu'un observateur.{}",
                user_name, end_awareness
            ),
        }
    } else if current_turn == 1 {
        // Other speakers on Turn 1 — present their own opinion, don't debate yet
        match discussion_language {
            "en" => format!(
                "This is the OPENING ROUND — present your initial opinion ONLY. \
                 Share YOUR OWN position on the topic with a strong, distinctive angle. \
                 Do NOT debate, challenge, or respond to what previous speakers said — \
                 the debate phase begins next round. Focus on what YOU think. \
                 Keep it to one focused paragraph. \
                 Do NOT address or speak to {} who is only an observer.{}",
                user_name, end_awareness
            ),
            "zh" => format!(
                "这是开场轮——仅表达你的初始观点。\
                 以独特的角度分享你对主题的立场。\
                 不要辩论、质疑或回应之前发言者说的话——\
                 辩论阶段从下一轮开始。专注于你自己的想法。\
                 保持一段集中的论述。\
                 不要对{}说话，此人只是观察者。{}",
                user_name, end_awareness
            ),
            _ => format!(
                "C'est le TOUR D'OUVERTURE — présente uniquement ton opinion initiale. \
                 Partage TA PROPRE position sur le sujet avec un angle fort et distinctif. \
                 Ne débats PAS, ne conteste PAS, ne réponds PAS à ce que les précédents intervenants ont dit — \
                 la phase de débat commence au prochain tour. Concentre-toi sur ce que TU penses. \
                 Reste sur un paragraphe concentré. \
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
             IMPORTANT: Write ALL text values (\"comment\" and \"ban_reason\") in English.\n\
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
             重要：所有文本值（\"comment\"和\"ban_reason\"）必须用中文书写。\n\
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
             IMPORTANT : Rédige TOUTES les valeurs texte (\"comment\" et \"ban_reason\") en français.\n\
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
    web_search_results: Option<&str>,
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
                "en" => "\n⚡ Only reference results that are relevant to the debate. Ignore off-topic results. \
                         Weave pertinent facts naturally into your synthesis to ground it in concrete data.",
                "zh" => "\n⚡ 只引用与辩论相关的结果。忽略偏题的结果。\
                         将相关事实自然地融入你的总结中，以具体数据来支撑。",
                _ => "\n⚡ Ne fais référence qu'aux résultats pertinents pour le débat. Ignore les résultats hors-sujet. \
                         Intègre les faits pertinents naturellement dans ta synthèse pour l'ancrer dans des données concrètes.",
            };
            format!("\n\n{}{}", r, instruction)
        })
        .unwrap_or_default();

    match discussion_language {
        "en" => format!(
            "{}\n\nThe debate on \"{}\" is now over.\n\n\
             Discussion summary:\n{}\n\n\
             Final positions:\n{}{}\n\n\
             As moderator, produce a structured synthesis:\n\
             1. Main points of agreement\n\
             2. Key disagreements\n\
             3. Most notable arguments\n\
             4. Overall conclusion\n\n\
             Be balanced and thorough (8-15 sentences).",
            datetime, topic, memory.contextual_summary, positions, web_block
        ),
        "zh" => format!(
            "{}\n\n关于\"{}\"的辩论现在结束了。\n\n\
             讨论摘要：\n{}\n\n\
             最终立场：\n{}{}\n\n\
             作为主持人，请做出结构化总结：\n\
             1. 主要共识点\n\
             2. 关键分歧\n\
             3. 最值得注意的论点\n\
             4. 整体结论\n\n\
             请公正全面（8-15句话）。",
            datetime, topic, memory.contextual_summary, positions, web_block
        ),
        _ => format!(
            "{}\n\nLe débat sur \"{}\" est maintenant terminé.\n\n\
             Résumé de la discussion :\n{}\n\n\
             Positions finales :\n{}{}\n\n\
             En tant que modérateur, produis une synthèse structurée :\n\
             1. Points d'accord principaux\n\
             2. Désaccords majeurs\n\
             3. Arguments les plus marquants\n\
             4. Conclusion générale\n\n\
             Sois équilibré et exhaustif (8-15 phrases).",
            datetime, topic, memory.contextual_summary, positions, web_block
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

    if emotions.frustration > HIGH_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your frustration level is critical.".to_string(),
            "zh" => "⚠ 你的挫败感已达临界水平。".to_string(),
            _ => "⚠ Ton niveau de frustration est critique.".to_string(),
        });
    }
    if emotions.engagement < LOW_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your engagement is at rock bottom.".to_string(),
            "zh" => "⚠ 你的投入度已降至最低。".to_string(),
            _ => "⚠ Ton engagement est au plus bas.".to_string(),
        });
    }
    if emotions.confiance > HIGH_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your confidence is at its peak.".to_string(),
            "zh" => "⚠ 你的信心已达巅峰。".to_string(),
            _ => "⚠ Ta confiance est à son maximum.".to_string(),
        });
    }
    if emotions.confiance < LOW_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your confidence is at rock bottom.".to_string(),
            "zh" => "⚠ 你的信心已降至最低。".to_string(),
            _ => "⚠ Ta confiance est au plus bas.".to_string(),
        });
    }
    if emotions.curiosite > HIGH_THRESHOLD {
        instructions.push(match lang {
            "en" => "⚠ Your curiosity is at its peak.".to_string(),
            "zh" => "⚠ 你的好奇心已达巅峰。".to_string(),
            _ => "⚠ Ta curiosité est à son comble.".to_string(),
        });
    }
    if emotions.enthousiasme > HIGH_THRESHOLD {
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
             Debate topic: \"{}\"\n\
             Recent context: {}\n\
             Remaining searches: {}{}{}\n\n\
             Based on YOUR unique expertise, do you need specific factual information to strengthen YOUR argument?\n\
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
             辩论主题：\"{}\"\n\
             近期背景：{}\n\
             剩余搜索次数：{}{}{}\n\n\
             基于你独特的专业知识，你需要具体的事实信息来加强你的论点吗？\n\
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
             Sujet du débat : \"{}\"\n\
             Contexte récent : {}\n\
             Recherches restantes : {}{}{}\n\n\
             En fonction de TON expertise unique, as-tu besoin d'informations factuelles spécifiques pour renforcer TON argument ?\n\
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
                output.push_str(&format!("{}: {}\n", summary_label, truncate(answer, 500)));
            }
        }

        if !response.results.is_empty() {
            output.push_str(&format!("{}:\n", sources_label));
            for (i, result) in response.results.iter().take(5).enumerate() {
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
                    truncate(&result.title, 100),
                    domain,
                    truncate(&result.content, 300)
                ));
            }
        }
        output.push('\n');

        // Hard limit on total output
        if output.len() > MAX_SEARCH_CONTEXT_LEN {
            let boundary = output.floor_char_boundary(MAX_SEARCH_CONTEXT_LEN);
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
        "en" => "Use Wikipedia to find encyclopedic definitions, historical context, scientific concepts, and established facts relevant to the debate.",
        "zh" => "使用维基百科查找与辩论相关的百科定义、历史背景、科学概念和既定事实。",
        _ => "Utilise Wikipédia pour trouver des définitions encyclopédiques, du contexte historique, des concepts scientifiques et des faits établis pertinents au débat.",
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
             Debate topic: \"{}\"\n\
             Recent context: {}\n\
             Remaining searches: {}{}{}{}\n\n\
             Based on YOUR unique expertise, choose a Wikipedia article that would help YOU bring an ORIGINAL perspective to this debate.\n\
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
             辩论主题：\"{}\"\n\
             近期背景：{}\n\
             剩余搜索次数：{}{}{}{}\n\n\
             基于你独特的专业知识，选择一篇能帮助你为这场辩论带来原创视角的维基百科文章。\n\
             使用维基百科上显示的确切文章标题（例如：「美利坚合众国宪法第二十二条修正案」，而不是「修正案22」）。\n\
             重要：用中文撰写文章标题。\n\
             仅用以下JSON格式回复：\n\
             {{\"needs_search\": true, \"queries\": [\"确切文章标题\"]}}\n\
             或 {{\"needs_search\": false, \"queries\": []}} 如果你已有足够信息。",
            datetime, search_directive, topic, recent_context, searches_remaining, past_block, others_block, web_block
        ),
        _ => format!(
            "{}\n\nTu as accès à Wikipédia. {}\n\
             Sujet du débat : \"{}\"\n\
             Contexte récent : {}\n\
             Recherches restantes : {}{}{}{}\n\n\
             En fonction de TON expertise unique, choisis un article Wikipédia qui t'aiderait à apporter un angle ORIGINAL à ce débat.\n\
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

            for (i, page) in pages.iter().take(3).enumerate() {
                if page.extract.is_empty() {
                    continue;
                }
                output.push_str(&format!(
                    "{}. \"{}\" : {}\n",
                    i + 1,
                    truncate(&page.title, 100),
                    truncate(&page.extract, 400)
                ));
            }
        }
        output.push('\n');

        if output.len() > MAX_SEARCH_CONTEXT_LEN {
            let boundary = output.floor_char_boundary(MAX_SEARCH_CONTEXT_LEN);
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
            ("frustration", fv) if fv >= 70 => match lang {
                "en" => pick(&["tense and irritated", "frustrated by the exchanges", "visibly on edge"], v),
                "zh" => pick(&["紧张且烦躁", "对交流感到不满", "明显焦躁不安"], v),
                _ => pick(&["tendu et agacé", "irrité par les échanges", "au bord de l'exaspération"], v),
            },
            ("frustration", fv) if fv <= 20 => match lang {
                "en" => pick(&["calm and serene", "relaxed and at ease", "perfectly composed"], v),
                "zh" => pick(&["平静而从容", "放松自在", "泰然自若"], v),
                _ => pick(&["calme et serein", "détendu et apaisé", "parfaitement posé"], v),
            },
            ("enthousiasme", ev) if ev >= 70 => match lang {
                "en" => pick(&["enthusiastic about the discussion", "fired up by the debate", "brimming with energy"], v),
                "zh" => pick(&["对讨论充满热情", "被辩论所激发", "精力充沛"], v),
                _ => pick(&["enthousiasmé par les échanges", "porté par l'élan du débat", "galvanisé par la discussion"], v),
            },
            ("enthousiasme", ev) if ev <= 30 => match lang {
                "en" => pick(&["lacking enthusiasm", "somewhat indifferent", "showing little energy"], v),
                "zh" => pick(&["缺乏热情", "显得漠不关心", "了无生气"], v),
                _ => pick(&["peu enthousiaste", "assez indifférent", "sans entrain particulier"], v),
            },
            ("engagement", ev) if ev >= 70 => match lang {
                "en" => pick(&["deeply invested in the debate", "fully engaged", "absorbed in the discussion"], v),
                "zh" => pick(&["深入参与辩论", "全身心投入", "沉浸在讨论中"], v),
                _ => pick(&["très investi dans le débat", "pleinement engagé", "absorbé par la discussion"], v),
            },
            ("engagement", ev) if ev <= 30 => match lang {
                "en" => pick(&["detached from the discussion", "somewhat disengaged", "losing interest"], v),
                "zh" => pick(&["对讨论超然", "有些心不在焉", "渐失兴趣"], v),
                _ => pick(&["détaché de la discussion", "en retrait du débat", "de plus en plus distant"], v),
            },
            ("curiosite", cv) if cv >= 70 => match lang {
                "en" => pick(&["very curious about the arguments", "intrigued by the ideas", "eager to explore further"], v),
                "zh" => pick(&["对论点非常好奇", "被各种观点所吸引", "渴望深入探究"], v),
                _ => pick(&["très curieux des arguments avancés", "intrigué par les idées échangées", "avide de comprendre"], v),
            },
            ("curiosite", cv) if cv <= 30 => match lang {
                "en" => pick(&["showing little curiosity", "unimpressed by the arguments", "not particularly intrigued"], v),
                "zh" => pick(&["缺乏好奇心", "对论点不以为然", "兴趣索然"], v),
                _ => pick(&["peu curieux", "pas vraiment intrigué", "indifférent aux arguments"], v),
            },
            ("confiance", cv) if cv >= 70 => match lang {
                "en" => pick(&["confident in their position", "assertive and self-assured", "unwavering in conviction"], v),
                "zh" => pick(&["对自己的立场充满信心", "态度坚定而自信", "立场坚定不移"], v),
                _ => pick(&["confiant dans sa position", "assuré et déterminé", "sûr de son fait"], v),
            },
            ("confiance", cv) if cv <= 30 => match lang {
                "en" => pick(&["hesitant and uncertain", "second-guessing their position", "lacking conviction"], v),
                "zh" => pick(&["犹豫不决", "在质疑自己的立场", "缺乏信念"], v),
                _ => pick(&["hésitant et incertain", "en proie au doute", "peu sûr de lui"], v),
            },
            ("accord", av) if av >= 70 => match lang {
                "en" => pick(&["in agreement with the others", "finding common ground", "largely aligned with the group"], v),
                "zh" => pick(&["与他人意见一致", "找到了共识", "基本认同大家的观点"], v),
                _ => pick(&["en accord avec les autres", "dans un esprit de consensus", "aligné avec le groupe"], v),
            },
            ("accord", av) if av <= 30 => match lang {
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
        // Create results with lots of content to trigger MAX_SEARCH_CONTEXT_LEN limit
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
            ctx.len() <= MAX_SEARCH_CONTEXT_LEN,
            "Output should be truncated to {} chars, got {}",
            MAX_SEARCH_CONTEXT_LEN, ctx.len()
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
            ctx.len() <= MAX_SEARCH_CONTEXT_LEN,
            "Output should be truncated to {} chars, got {}",
            MAX_SEARCH_CONTEXT_LEN, ctx.len()
        );
    }
}
