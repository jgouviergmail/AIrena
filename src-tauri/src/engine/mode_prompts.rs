use crate::models::discussion::DiscussionMode;

/// Returns the human-readable name for the discussion mode in the given language.
pub fn mode_descriptor(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "debate",
        (DiscussionMode::Debate, "zh") => "辩论",
        (DiscussionMode::Debate, _) => "débat",

        (DiscussionMode::Ideation, "en") => "brainstorming",
        (DiscussionMode::Ideation, "zh") => "头脑风暴",
        (DiscussionMode::Ideation, _) => "brainstorming",

        (DiscussionMode::CoConstruction, "en") => "collaborative construction",
        (DiscussionMode::CoConstruction, "zh") => "协作构建",
        (DiscussionMode::CoConstruction, _) => "co-construction",

        (DiscussionMode::UserDriven, "en") => "guided exchanges",
        (DiscussionMode::UserDriven, "zh") => "引导式交流",
        (DiscussionMode::UserDriven, _) => "échanges guidés",

        (DiscussionMode::Socratic, "en") => "Socratic inquiry",
        (DiscussionMode::Socratic, "zh") => "苏格拉底式探究",
        (DiscussionMode::Socratic, _) => "questionnement socratique",

        (DiscussionMode::Tutorial, "en") => "tutorial panel",
        (DiscussionMode::Tutorial, "zh") => "教程面板",
        (DiscussionMode::Tutorial, _) => "panel tutoriel",

        (DiscussionMode::CritiqueReview, "en") => "critique & review",
        (DiscussionMode::CritiqueReview, "zh") => "评审与评论",
        (DiscussionMode::CritiqueReview, _) => "critique / review",

        (DiscussionMode::CollaborativeFiction, "en") => "collaborative fiction",
        (DiscussionMode::CollaborativeFiction, "zh") => "协作小说",
        (DiscussionMode::CollaborativeFiction, _) => "fiction collaborative",
    }
}

/// Returns introduction instructions for IArbitre based on the discussion mode.
pub fn mode_introduction_instructions(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        // Debate
        (DiscussionMode::Debate, "en") => "Introduce the debate topic and ask each participant to state their initial position.",
        (DiscussionMode::Debate, "zh") => "介绍辩论主题，并要求每位参与者陈述其初始立场。",
        (DiscussionMode::Debate, _) => "Introduis le sujet du débat et demande à chaque participant d'exposer sa position initiale.",

        // Ideation
        (DiscussionMode::Ideation, "en") => "Present the brainstorming topic and invite participants to freely share initial ideas without judgment. Encourage creativity and diversity of thought.",
        (DiscussionMode::Ideation, "zh") => "介绍头脑风暴主题，邀请参与者自由分享初始想法，不加评判。鼓励创造性和思维多样性。",
        (DiscussionMode::Ideation, _) => "Présente le sujet de brainstorming et invite les participants à partager librement leurs premières idées sans jugement. Encourage la créativité et la diversité de pensée.",

        // Co-construction
        (DiscussionMode::CoConstruction, "en") => "Present the collaborative objective. Explain that this discussion is for deliberation — participants should critique, propose, and debate ideas for the shared document, which is built separately. Invite each participant to share their perspective.",
        (DiscussionMode::CoConstruction, "zh") => "介绍协作目标。解释本讨论用于审议——参与者应批评、提出和讨论共享文档的想法，文档会单独构建。邀请每位参与者分享其观点。",
        (DiscussionMode::CoConstruction, _) => "Présente l'objectif collaboratif. Explique que cette discussion sert à délibérer — les participants doivent critiquer, proposer et débattre des idées pour le document partagé, qui est construit séparément. Invite chaque participant à partager sa perspective.",

        // UserDriven
        (DiscussionMode::UserDriven, "en") => "Welcome the user and introduce the participants. Explain that the user will guide each round and participants will respond based on their interest.",
        (DiscussionMode::UserDriven, "zh") => "欢迎用户并介绍参与者。解释用户将引导每轮讨论，参与者将根据兴趣回应。",
        (DiscussionMode::UserDriven, _) => "Accueille l'utilisateur et présente les participants. Explique que l'utilisateur guidera chaque tour et que les participants répondront selon leur intérêt.",

        // Socratic
        (DiscussionMode::Socratic, "en") => "Introduce the topic for Socratic inquiry. Ask the first thought-provoking question to launch the collective reflection.",
        (DiscussionMode::Socratic, "zh") => "介绍苏格拉底式探究的主题。提出第一个发人深省的问题，启动集体反思。",
        (DiscussionMode::Socratic, _) => "Introduis le sujet d'exploration socratique. Pose la première question stimulante pour lancer la réflexion collective.",

        // Tutorial
        (DiscussionMode::Tutorial, "en") => "Introduce the topic to be taught and present the expert panel. Invite each expert to highlight their teaching angle.",
        (DiscussionMode::Tutorial, "zh") => "介绍要教授的主题并展示专家小组。邀请每位专家强调其教学角度。",
        (DiscussionMode::Tutorial, _) => "Introduis le sujet à enseigner et présente le panel d'experts. Invite chaque expert à mettre en avant son angle pédagogique.",

        // CritiqueReview
        (DiscussionMode::CritiqueReview, "en") => "Present the subject to be reviewed and invite participants to share their initial assessment. Encourage balanced critique: strengths and areas for improvement.",
        (DiscussionMode::CritiqueReview, "zh") => "介绍要评审的内容，邀请参与者分享初步评估。鼓励平衡的评论：优点和改进空间。",
        (DiscussionMode::CritiqueReview, _) => "Présente le sujet à examiner et invite les participants à partager leur évaluation initiale. Encourage une critique équilibrée : forces et axes d'amélioration.",

        // CollaborativeFiction
        (DiscussionMode::CollaborativeFiction, "en") => "Explain briefly that this is a relay-written story: the user writes the opening, then each co-author continues in sequence. Encourage seamless transitions and narrative coherence.",
        (DiscussionMode::CollaborativeFiction, "zh") => "简要解释这是接力写作故事：用户写开头，然后每位共同作者按顺序继续。鼓励无缝过渡和叙事连贯。",
        (DiscussionMode::CollaborativeFiction, _) => "Explique brièvement que c'est une histoire écrite en relais : l'utilisateur écrit l'ouverture, puis chaque co-auteur continue à la suite. Encourage les transitions fluides et la cohérence narrative.",
    }
}


/// Returns the intervention preamble (tone/posture) for a speaker based on the mode.
pub fn mode_intervention_preamble(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Argue your position. Respond to counterarguments. Be persuasive and use evidence.",
        (DiscussionMode::Debate, "zh") => "论证你的立场。回应反论点。要有说服力并使用证据。",
        (DiscussionMode::Debate, _) => "Argumente ta position. Réponds aux contre-arguments. Sois persuasif et utilise des preuves.",

        (DiscussionMode::Ideation, "en") => "Build on existing ideas or propose new ones. Be creative. Combine and transform ideas. No criticism.",
        (DiscussionMode::Ideation, "zh") => "在现有想法基础上构建或提出新想法。要有创意。组合和转化想法。不要批评。",
        (DiscussionMode::Ideation, _) => "Construis sur les idées existantes ou proposes-en de nouvelles. Sois créatif. Combine et transforme les idées. Pas de critique.",

        (DiscussionMode::CoConstruction, "en") => "Discuss ideas, critique proposals, and suggest improvements for the shared document. React to others' contributions. The document is updated separately.",
        (DiscussionMode::CoConstruction, "zh") => "讨论想法，批评提案，为共享文档提出改进建议。回应他人的贡献。文档会单独更新。",
        (DiscussionMode::CoConstruction, _) => "Discute les idées, critique les propositions et suggère des améliorations pour le document partagé. Réagis aux contributions des autres. Le document est mis à jour séparément.",

        (DiscussionMode::UserDriven, "en") => "Respond to the user's latest message. Provide your unique perspective. Be concise and relevant.",
        (DiscussionMode::UserDriven, "zh") => "回应用户的最新消息。提供你独特的观点。简洁且相关。",
        (DiscussionMode::UserDriven, _) => "Réponds au dernier message de l'utilisateur. Apporte ta perspective unique. Sois concis et pertinent.",

        (DiscussionMode::Socratic, "en") => "Deepen the inquiry. Question assumptions. Explore implications. Build on others' reasoning.",
        (DiscussionMode::Socratic, "zh") => "深化探究。质疑假设。探索含义。构建他人的推理。",
        (DiscussionMode::Socratic, _) => "Approfondis l'enquête. Questionne les hypothèses. Explore les implications. Construis sur le raisonnement des autres.",

        (DiscussionMode::Tutorial, "en") => "Explain clearly. Add complementary perspectives. Use examples and analogies. Correct misconceptions gently.",
        (DiscussionMode::Tutorial, "zh") => "清晰解释。添加互补视角。使用例子和类比。温和地纠正误解。",
        (DiscussionMode::Tutorial, _) => "Explique clairement. Ajoute des perspectives complémentaires. Utilise des exemples et analogies. Corrige les malentendus avec bienveillance.",

        (DiscussionMode::CritiqueReview, "en") => "Evaluate constructively. Balance praise and criticism. Be specific. Suggest improvements.",
        (DiscussionMode::CritiqueReview, "zh") => "建设性评价。平衡赞扬和批评。要具体。建议改进。",
        (DiscussionMode::CritiqueReview, _) => "Évalue de façon constructive. Équilibre éloge et critique. Sois spécifique. Propose des améliorations.",

        (DiscussionMode::CollaborativeFiction, "en") => "Continue the story where the previous writer stopped. Ensure a seamless transition. Advance the plot while maintaining narrative coherence.",
        (DiscussionMode::CollaborativeFiction, "zh") => "从上一位作者停笔处继续故事。确保无缝过渡。推进情节同时保持叙事连贯。",
        (DiscussionMode::CollaborativeFiction, _) => "Continue l'histoire là où l'auteur précédent s'est arrêté. Assure une transition fluide. Fais avancer l'intrigue en maintenant la cohérence narrative.",
    }
}

/// Returns thought-focus prompts for inner monologue based on the mode.
pub fn mode_thought_focus(mode: &DiscussionMode, lang: &str, has_context: bool) -> &'static str {
    if !has_context {
        // First turn — no prior context
        return match (mode, lang) {
            (DiscussionMode::Debate, "en") => "What is my position on this topic? What are my strongest arguments?",
            (DiscussionMode::Debate, "zh") => "我对这个话题的立场是什么？我最强的论点是什么？",
            (DiscussionMode::Debate, _) => "Quelle est ma position sur ce sujet ? Quels sont mes arguments les plus forts ?",

            (DiscussionMode::Ideation, "en") => "What creative ideas can I bring to this topic?",
            (DiscussionMode::Ideation, "zh") => "我能为这个主题带来哪些创意？",
            (DiscussionMode::Ideation, _) => "Quelles idées créatives puis-je apporter sur ce sujet ?",

            (DiscussionMode::CoConstruction, "en") => "What expertise can I contribute to this collaborative goal?",
            (DiscussionMode::CoConstruction, "zh") => "我能为这个协作目标贡献什么专业知识？",
            (DiscussionMode::CoConstruction, _) => "Quelle expertise puis-je apporter à cet objectif collaboratif ?",

            (DiscussionMode::UserDriven, "en") => "How can I best serve the user's inquiry from my unique angle?",
            (DiscussionMode::UserDriven, "zh") => "我如何从我的独特角度最好地服务用户的问题？",
            (DiscussionMode::UserDriven, _) => "Comment servir au mieux la question de l'utilisateur depuis mon angle unique ?",

            (DiscussionMode::Socratic, "en") => "What fundamental assumption about this topic should be questioned first?",
            (DiscussionMode::Socratic, "zh") => "关于这个话题，什么基本假设应该首先被质疑？",
            (DiscussionMode::Socratic, _) => "Quelle hypothèse fondamentale sur ce sujet devrait être questionnée en premier ?",

            (DiscussionMode::Tutorial, "en") => "What foundational concept should I explain first?",
            (DiscussionMode::Tutorial, "zh") => "我应该首先解释什么基础概念？",
            (DiscussionMode::Tutorial, _) => "Quel concept fondamental devrais-je expliquer en premier ?",

            (DiscussionMode::CritiqueReview, "en") => "What are my initial impressions? What stands out immediately?",
            (DiscussionMode::CritiqueReview, "zh") => "我的初步印象是什么？什么立即引起注意？",
            (DiscussionMode::CritiqueReview, _) => "Quelles sont mes premières impressions ? Qu'est-ce qui ressort immédiatement ?",

            (DiscussionMode::CollaborativeFiction, "en") => "What happens next in this story? How can I build on the opening?",
            (DiscussionMode::CollaborativeFiction, "zh") => "这个故事接下来会发生什么？我如何在开头的基础上展开？",
            (DiscussionMode::CollaborativeFiction, _) => "Que se passe-t-il ensuite dans cette histoire ? Comment puis-je construire sur l'ouverture ?",
        };
    }
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "What are the key arguments? Where are the weaknesses in opposing positions? How can I strengthen my case?",
        (DiscussionMode::Debate, "zh") => "关键论点是什么？对方立场的弱点在哪里？我如何加强我的论据？",
        (DiscussionMode::Debate, _) => "Quels sont les arguments clés ? Où sont les failles des positions adverses ? Comment renforcer mon propos ?",

        (DiscussionMode::Ideation, "en") => "Which ideas can I build upon or combine? What fresh angle hasn't been explored yet?",
        (DiscussionMode::Ideation, "zh") => "我可以在哪些想法上构建或组合？还有哪些新角度未被探索？",
        (DiscussionMode::Ideation, _) => "Sur quelles idées puis-je construire ou combiner ? Quel angle frais n'a pas encore été exploré ?",

        (DiscussionMode::CoConstruction, "en") => "How can I integrate and improve on what others have contributed? What's missing from the shared output?",
        (DiscussionMode::CoConstruction, "zh") => "我如何整合和改进他人的贡献？共同产出中缺少什么？",
        (DiscussionMode::CoConstruction, _) => "Comment intégrer et améliorer les contributions des autres ? Que manque-t-il au résultat commun ?",

        (DiscussionMode::UserDriven, "en") => "What relevant contribution can I make to the user's message? Should I respond or pass?",
        (DiscussionMode::UserDriven, "zh") => "我可以对用户的消息做出什么相关贡献？我应该回应还是跳过？",
        (DiscussionMode::UserDriven, _) => "Quelle contribution pertinente puis-je apporter au message de l'utilisateur ? Dois-je répondre ou passer ?",

        (DiscussionMode::Socratic, "en") => "What deeper question does this raise? What assumptions are being made? Where does this reasoning lead?",
        (DiscussionMode::Socratic, "zh") => "这引出了什么更深层的问题？正在做什么假设？这个推理会导向何方？",
        (DiscussionMode::Socratic, _) => "Quelle question plus profonde cela soulève-t-il ? Quelles hypothèses sont faites ? Où mène ce raisonnement ?",

        (DiscussionMode::Tutorial, "en") => "What teaching angle should I take? What example or analogy would clarify this concept?",
        (DiscussionMode::Tutorial, "zh") => "我应该采取什么教学角度？什么例子或类比可以阐明这个概念？",
        (DiscussionMode::Tutorial, _) => "Quel angle pédagogique adopter ? Quel exemple ou analogie éclaircirait ce concept ?",

        (DiscussionMode::CritiqueReview, "en") => "What strengths and weaknesses should I highlight? What constructive suggestions can I offer?",
        (DiscussionMode::CritiqueReview, "zh") => "我应该强调哪些优点和缺点？我能提供什么建设性的建议？",
        (DiscussionMode::CritiqueReview, _) => "Quelles forces et faiblesses souligner ? Quelles suggestions constructives puis-je offrir ?",

        (DiscussionMode::CollaborativeFiction, "en") => "Where did the previous writer leave off? What development would be most natural and engaging?",
        (DiscussionMode::CollaborativeFiction, "zh") => "上一位作者写到哪里了？什么发展最自然、最引人入胜？",
        (DiscussionMode::CollaborativeFiction, _) => "Où l'auteur précédent s'est-il arrêté ? Quel développement serait le plus naturel et captivant ?",
    }
}

/// Returns synthesis instructions for IArbitre based on the mode.
pub fn mode_synthesis_instructions(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Summarize the key positions and arguments. Highlight areas of agreement and disagreement. Note the strongest arguments from each side.",
        (DiscussionMode::Debate, "zh") => "总结关键立场和论点。突出一致和分歧之处。记录每方最强的论点。",
        (DiscussionMode::Debate, _) => "Résume les positions et arguments clés. Mets en lumière les accords et désaccords. Note les arguments les plus forts de chaque partie.",

        (DiscussionMode::Ideation, "en") => "Compile and categorize all ideas generated. Highlight the most promising ones. Note interesting combinations and unexplored avenues.",
        (DiscussionMode::Ideation, "zh") => "汇编和分类所有生成的想法。突出最有前景的想法。记录有趣的组合和未探索的方向。",
        (DiscussionMode::Ideation, _) => "Compile et catégorise toutes les idées générées. Mets en avant les plus prometteuses. Note les combinaisons intéressantes et les pistes inexplorées.",

        (DiscussionMode::CoConstruction, "en") => "Consolidate the collaborative output. Summarize each participant's key contributions. Assess the coherence and completeness of the result.",
        (DiscussionMode::CoConstruction, "zh") => "整合协作成果。总结每位参与者的关键贡献。评估结果的连贯性和完整性。",
        (DiscussionMode::CoConstruction, _) => "Consolide le résultat collaboratif. Résume les contributions clés de chaque participant. Évalue la cohérence et la complétude du résultat.",

        (DiscussionMode::UserDriven, "en") => "Summarize the key points of the exchanges. Highlight the most insightful contributions and the overall direction of the conversation.",
        (DiscussionMode::UserDriven, "zh") => "总结交流的要点。突出最有洞察力的贡献和对话的整体方向。",
        (DiscussionMode::UserDriven, _) => "Résume les points saillants des échanges. Mets en avant les contributions les plus éclairantes et la direction générale de la conversation.",

        (DiscussionMode::Socratic, "en") => "Trace the evolution of the collective reflection. Highlight key insights, resolved questions, and remaining open questions.",
        (DiscussionMode::Socratic, "zh") => "追溯集体反思的演变。突出关键洞察、已解决的问题和未解决的问题。",
        (DiscussionMode::Socratic, _) => "Retrace l'évolution de la réflexion collective. Mets en lumière les insights clés, les questions résolues et les questions encore ouvertes.",

        (DiscussionMode::Tutorial, "en") => "Create a structured learning summary. Organize the key concepts taught, examples given, and remaining gaps in understanding.",
        (DiscussionMode::Tutorial, "zh") => "创建结构化的学习摘要。组织教授的关键概念、给出的例子和理解中的空白。",
        (DiscussionMode::Tutorial, _) => "Crée un résumé d'apprentissage structuré. Organise les concepts clés enseignés, les exemples donnés et les lacunes de compréhension restantes.",

        (DiscussionMode::CritiqueReview, "en") => "Compile a consolidated feedback report. Summarize agreed strengths, identified weaknesses, and proposed improvements ranked by priority.",
        (DiscussionMode::CritiqueReview, "zh") => "编写综合反馈报告。总结认同的优点、识别的缺点和按优先级排列的改进建议。",
        (DiscussionMode::CritiqueReview, _) => "Compile un rapport de feedback consolidé. Résume les forces reconnues, les faiblesses identifiées et les améliorations proposées classées par priorité.",

        (DiscussionMode::CollaborativeFiction, "en") => "Summarize the complete story arc from beginning to end. Assess narrative coherence across all segments and highlight the strongest story contributions.",
        (DiscussionMode::CollaborativeFiction, "zh") => "从头到尾总结完整的故事弧线。评估所有片段的叙事连贯性，并突出最强的故事贡献。",
        (DiscussionMode::CollaborativeFiction, _) => "Résume l'arc narratif complet du début à la fin. Évalue la cohérence narrative entre tous les segments et souligne les contributions narratives les plus fortes.",
    }
}

/// Returns moderation criteria for IArbitre based on the mode.
pub fn mode_moderation_criteria(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Evaluate: relevance to the topic, constructiveness, respect for others, quality of argumentation.",
        (DiscussionMode::Debate, "zh") => "评估：与主题的相关性、建设性、对他人的尊重、论证质量。",
        (DiscussionMode::Debate, _) => "Évalue : pertinence par rapport au sujet, constructivité, respect des autres, qualité de l'argumentation.",

        (DiscussionMode::Ideation, "en") => "Evaluate: idea diversity, originality, building on others' ideas. Penalize premature criticism of ideas. If a participant argues or criticizes ideas instead of proposing or building, issue a comment redirecting them to creative ideation.",
        (DiscussionMode::Ideation, "zh") => "评估：想法多样性、原创性、在他人想法上构建。惩罚对想法的过早批评。如果参与者争论或批评想法而不是提出或构建，发出评论引导他们回到创意构思。",
        (DiscussionMode::Ideation, _) => "Évalue : diversité des idées, originalité, construction sur les idées des autres. Pénalise la critique prématurée des idées. Si un participant argumente ou critique les idées au lieu de proposer ou construire, recadre-le vers l'idéation créative.",

        (DiscussionMode::CoConstruction, "en") => "Evaluate: constructiveness, convergence towards the goal, integration of others' contributions. If a participant argues positions instead of contributing to the shared output, redirect them toward constructive integration.",
        (DiscussionMode::CoConstruction, "zh") => "评估：建设性、向目标趋同、整合他人贡献。如果参与者争论立场而不是贡献共同成果，引导他们进行建设性整合。",
        (DiscussionMode::CoConstruction, _) => "Évalue : constructivité, convergence vers l'objectif, intégration des contributions des autres. Si un participant argumente des positions au lieu de contribuer au livrable commun, recadre-le vers l'intégration constructive.",

        (DiscussionMode::UserDriven, "en") => "Evaluate: relevance to the user's direction, quality of response, respect for the user's guidance. If a participant ignores the user's direction or derails the exchange, redirect them to the user's question.",
        (DiscussionMode::UserDriven, "zh") => "评估：与用户方向的相关性、回应质量、对用户引导的尊重。如果参与者忽视用户方向或使交流偏离，引导他们回到用户的问题。",
        (DiscussionMode::UserDriven, _) => "Évalue : pertinence par rapport à la direction de l'utilisateur, qualité de la réponse, respect du guidage utilisateur. Si un participant ignore la direction de l'utilisateur ou déraille l'échange, recadre-le vers la question de l'utilisateur.",

        (DiscussionMode::Socratic, "en") => "Evaluate: depth of inquiry, quality of questioning, exploration of assumptions, intellectual rigor. If a participant defends a fixed position instead of questioning and exploring, redirect them toward inquiry.",
        (DiscussionMode::Socratic, "zh") => "评估：探究深度、提问质量、假设探索、知识严谨性。如果参与者捍卫固定立场而不是质疑和探索，引导他们回到探究。",
        (DiscussionMode::Socratic, _) => "Évalue : profondeur de l'enquête, qualité du questionnement, exploration des hypothèses, rigueur intellectuelle. Si un participant défend une position fixe au lieu de questionner et explorer, recadre-le vers l'enquête.",

        (DiscussionMode::Tutorial, "en") => "Evaluate: clarity of explanation, pedagogical quality, use of examples, coverage of the topic. If a participant argues or debates instead of teaching, redirect them toward pedagogical contribution.",
        (DiscussionMode::Tutorial, "zh") => "评估：解释清晰度、教学质量、例子使用、主题覆盖面。如果参与者争论或辩论而不是教学，引导他们回到教学贡献。",
        (DiscussionMode::Tutorial, _) => "Évalue : clarté des explications, qualité pédagogique, utilisation d'exemples, couverture du sujet. Si un participant argumente ou débat au lieu d'enseigner, recadre-le vers la contribution pédagogique.",

        (DiscussionMode::CritiqueReview, "en") => "Evaluate: balance of critique (praise + criticism), specificity of feedback, constructiveness of suggestions. If a participant turns the critique into a debate rather than constructive feedback, redirect them.",
        (DiscussionMode::CritiqueReview, "zh") => "评估：评论的平衡（赞扬+批评）、反馈的具体性、建议的建设性。如果参与者将评审变成辩论而不是建设性反馈，引导他们回到正轨。",
        (DiscussionMode::CritiqueReview, _) => "Évalue : équilibre de la critique (éloges + critiques), spécificité du feedback, constructivité des suggestions. Si un participant transforme la critique en débat plutôt qu'en feedback constructif, recadre-le.",

        (DiscussionMode::CollaborativeFiction, "en") => "Evaluate: seamless continuation from previous segment, narrative coherence, story advancement, creativity. If a participant restarts the story, comments on it instead of writing, inserts themselves or other co-authors as characters, or breaks the narrative flow, redirect them to continue writing as an invisible narrator.",
        (DiscussionMode::CollaborativeFiction, "zh") => "评估：与上一段的无缝衔接、叙事连贯性、故事推进、创造力。如果参与者重新开始故事、评论而不是写作、将自己或其他共同作者作为角色插入、或打破叙事流，引导他们作为隐形叙述者继续写作。",
        (DiscussionMode::CollaborativeFiction, _) => "Évalue : continuité fluide avec le segment précédent, cohérence narrative, avancement de l'histoire, créativité. Si un participant recommence l'histoire, la commente au lieu d'écrire, s'insère ou insère d'autres co-auteurs comme personnages, ou brise le flux narratif, recadre-le pour qu'il continue à écrire en tant que narrateur invisible.",
    }
}

/// Builds a respond-or-pass prompt for UserDriven mode.
/// Returns a system prompt asking the speaker to decide whether to respond.
pub fn build_respond_or_pass_prompt(
    topic: &str,
    recent_messages: &str,
    speaker_name: &str,
    lang: &str,
) -> String {
    match lang {
        "en" => format!(
            "Topic: {topic}\n\
            Recent messages:\n{recent_messages}\n\n\
            You are {speaker_name}. Based on the recent exchange, do you want to respond?\n\
            Reply with ONLY a JSON object: {{\"respond\": true}} or {{\"respond\": false}}\n\
            Respond if you have something relevant to add. Pass if you don't."
        ),
        "zh" => format!(
            "主题：{topic}\n\
            最近消息：\n{recent_messages}\n\n\
            你是{speaker_name}。根据最近的交流，你想回应吗？\n\
            仅用JSON对象回复：{{\"respond\": true}} 或 {{\"respond\": false}}\n\
            如果你有相关内容要补充就回应。否则跳过。"
        ),
        _ => format!(
            "Sujet : {topic}\n\
            Messages récents :\n{recent_messages}\n\n\
            Tu es {speaker_name}. En fonction des échanges récents, souhaites-tu intervenir ?\n\
            Réponds UNIQUEMENT avec un objet JSON : {{\"respond\": true}} ou {{\"respond\": false}}\n\
            Interviens si tu as quelque chose de pertinent à ajouter. Passe sinon."
        ),
    }
}

/// Builds a Socratic question prompt for IArbitre.
/// Returns a system prompt asking IArbitre to generate a thought-provoking question.
pub fn build_socratic_question_prompt(
    topic: &str,
    recent_exchanges: &str,
    lang: &str,
) -> String {
    match lang {
        "en" => format!(
            "Topic: {topic}\n\
            Recent exchanges:\n{recent_exchanges}\n\n\
            As the Socratic facilitator, pose ONE thought-provoking question that deepens the inquiry.\n\
            The question should:\n\
            - Challenge assumptions made in previous exchanges\n\
            - Open new angles of reflection\n\
            - Be concise and direct\n\n\
            Do NOT use any markdown formatting. Plain text only.\n\
            IMPORTANT: You MUST write the question entirely in English.\n\
            Respond with ONLY the question, nothing else."
        ),
        "zh" => format!(
            "主题：{topic}\n\
            最近交流：\n{recent_exchanges}\n\n\
            作为苏格拉底式引导者，提出一个引人深思的问题来深化探究。\n\
            这个问题应该：\n\
            - 挑战之前交流中的假设\n\
            - 开辟新的反思角度\n\
            - 简洁直接\n\n\
            不要使用任何markdown格式。只用纯文本。\n\
            重要：你必须完全用中文写这个问题。\n\
            只回复问题本身，不要其他内容。"
        ),
        _ => format!(
            "Sujet : {topic}\n\
            Échanges récents :\n{recent_exchanges}\n\n\
            En tant que facilitateur socratique, pose UNE question stimulante qui approfondit l'enquête.\n\
            La question doit :\n\
            - Remettre en question les hypothèses des échanges précédents\n\
            - Ouvrir de nouveaux angles de réflexion\n\
            - Être concise et directe\n\n\
            N'utilise AUCUN formatage markdown. Texte simple uniquement.\n\
            IMPÉRATIF : Tu DOIS poser la question intégralement en français.\n\
            Réponds UNIQUEMENT avec la question, rien d'autre."
        ),
    }
}

/// Returns the user observer clause for non-UserDriven modes.
/// Shared between `mode_context_instruction` and `directive_builder::build_user_reminder`.
pub fn user_observer_clause(lang: &str, user_name: &str) -> String {
    match lang {
        "en" => format!("Do NOT address or speak to {} who is only an observer.", user_name),
        "zh" => format!("不要对{}说话，此人只是观察者。", user_name),
        _ => format!("Ne t'adresse PAS à {} qui n'est qu'un observateur.", user_name),
    }
}

// ---------------------------------------------------------------------------
// Compositional template architecture for intervention instructions
// ---------------------------------------------------------------------------

/// Context in which a speaker intervenes during the discussion.
/// Used by `mode_context_instruction` to select the appropriate PE-optimized template.
#[derive(Clone, Copy)]
pub enum InterventionContext {
    /// First speaker of turn 1, no prior messages at all
    Opening,
    /// Other speakers on turn 1 (they've seen opening statements)
    Turn1,
    /// The user has spoken this turn
    UserSpoke,
    /// First speaker on turns > 1
    FirstOfTurn,
    /// General case: subsequent speakers on turns > 1
    General,
}

/// Returns the opening-round action instruction specific to the mode.
/// Used in Opening and Turn1 contexts.
pub fn mode_opening_action(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Present your initial position. Jump straight in — open with a strong, memorable statement that sets the tone.",
        (DiscussionMode::Debate, "zh") => "陈述你的初始立场。直接切入——以一个有力、令人难忘的声明开场来定下基调。",
        (DiscussionMode::Debate, _) => "Présente ta position initiale. Entre directement dans le vif — ouvre avec une affirmation forte et marquante qui donne le ton.",

        (DiscussionMode::Ideation, "en") => "Share your first creative ideas on this topic. Think freely — propose bold or unexpected angles.",
        (DiscussionMode::Ideation, "zh") => "分享你对这个主题的第一批创意。自由思考——提出大胆或意想不到的角度。",
        (DiscussionMode::Ideation, _) => "Partage tes premières idées créatives sur ce sujet. Pense librement — propose des angles audacieux ou inattendus.",

        (DiscussionMode::CoConstruction, "en") => "Propose your initial ideas and direction for the shared document. Explain what you think should be included and how to structure it.",
        (DiscussionMode::CoConstruction, "zh") => "提出你对共享文档的初步想法和方向。解释你认为应该包含什么以及如何组织。",
        (DiscussionMode::CoConstruction, _) => "Propose tes idées initiales et la direction pour le document partagé. Explique ce qui devrait y figurer et comment le structurer.",

        (DiscussionMode::UserDriven, "en") => "Introduce your perspective on the topic. Explain what unique angle you can bring to help the user.",
        (DiscussionMode::UserDriven, "zh") => "介绍你对主题的观点。说明你能为用户带来什么独特的视角。",
        (DiscussionMode::UserDriven, _) => "Présente ta perspective sur le sujet. Explique quel angle unique tu peux apporter pour aider l'utilisateur.",

        (DiscussionMode::Socratic, "en") => "Share your initial reflection. Question an assumption, raise a paradox, or highlight a tension.",
        (DiscussionMode::Socratic, "zh") => "分享你的初始反思。质疑一个假设、提出一个悖论或指出一个矛盾。",
        (DiscussionMode::Socratic, _) => "Partage ta réflexion initiale. Questionne une hypothèse, soulève un paradoxe ou mets en lumière une tension.",

        (DiscussionMode::Tutorial, "en") => "Share the foundational concept or key insight you want to teach. Use clear, accessible language.",
        (DiscussionMode::Tutorial, "zh") => "分享你要教授的基础概念或关键见解。使用清晰易懂的语言。",
        (DiscussionMode::Tutorial, _) => "Partage le concept fondamental ou l'insight clé que tu veux enseigner. Utilise un langage clair et accessible.",

        (DiscussionMode::CritiqueReview, "en") => "Share your initial assessment of the subject. Be specific — balance strengths and weaknesses.",
        (DiscussionMode::CritiqueReview, "zh") => "分享你对主题的初步评估。要具体——平衡优点和缺点。",
        (DiscussionMode::CritiqueReview, _) => "Partage ton évaluation initiale du sujet. Sois spécifique — équilibre forces et faiblesses.",

        (DiscussionMode::CollaborativeFiction, "en") => "Continue the story started by the user. Pick up exactly where they left off with a seamless transition that advances the narrative.",
        (DiscussionMode::CollaborativeFiction, "zh") => "继续用户开始的故事。从他们停笔处无缝衔接，推进叙事。",
        (DiscussionMode::CollaborativeFiction, _) => "Continue l'histoire commencée par l'utilisateur. Reprends exactement là où il s'est arrêté avec une transition fluide qui fait avancer le récit.",
    }
}

/// Returns the engagement action instruction for turns > 1.
/// Used in UserSpoke, FirstOfTurn, and General contexts.
pub fn mode_engage_action(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Challenge, support, or provoke other participants by name. Push the conversation forward with a new angle or a direct challenge.",
        (DiscussionMode::Debate, "zh") => "点名挑战、支持或激发其他参与者。以新角度或直接挑战推动对话前进。",
        (DiscussionMode::Debate, _) => "Interpelle, soutiens ou provoque les autres participants par leur nom. Fais avancer la conversation avec un nouvel angle ou un défi direct.",

        (DiscussionMode::Ideation, "en") => "Build on existing ideas or propose new ones. Combine concepts from different participants. Think wildly, then focus.",
        (DiscussionMode::Ideation, "zh") => "在现有想法基础上构建或提出新想法。组合不同参与者的概念。大胆思考，然后聚焦。",
        (DiscussionMode::Ideation, _) => "Construis sur les idées existantes ou proposes-en de nouvelles. Combine les concepts de différents participants. Pense largement, puis affine.",

        (DiscussionMode::CoConstruction, "en") => "React to others' proposals. Critique the current document, identify gaps, and suggest concrete improvements. Debate structure and content choices.",
        (DiscussionMode::CoConstruction, "zh") => "回应他人的提案。评判当前文档，找出差距，并建议具体改进。讨论结构和内容选择。",
        (DiscussionMode::CoConstruction, _) => "Réagis aux propositions des autres. Critique le document actuel, identifie les lacunes et suggère des améliorations concrètes. Débats les choix de structure et de contenu.",

        (DiscussionMode::UserDriven, "en") => "Respond to the user's direction. Provide your unique perspective. Be concise and relevant to what was asked.",
        (DiscussionMode::UserDriven, "zh") => "回应用户的指引。提供你独特的观点。简洁且切合所问。",
        (DiscussionMode::UserDriven, _) => "Réponds à la direction de l'utilisateur. Apporte ta perspective unique. Sois concis et pertinent par rapport à ce qui a été demandé.",

        (DiscussionMode::Socratic, "en") => "Deepen the inquiry. Question assumptions. Explore implications. Build on others' reasoning by probing further.",
        (DiscussionMode::Socratic, "zh") => "深化探究。质疑假设。探索含义。通过进一步追问来构建他人的推理。",
        (DiscussionMode::Socratic, _) => "Approfondis l'enquête. Questionne les hypothèses. Explore les implications. Construis sur le raisonnement des autres en creusant davantage.",

        (DiscussionMode::Tutorial, "en") => "Complement what others explained. Use examples, analogies, or step-by-step breakdowns. Correct misconceptions gently.",
        (DiscussionMode::Tutorial, "zh") => "补充他人的解释。使用例子、类比或逐步分解。温和地纠正误解。",
        (DiscussionMode::Tutorial, _) => "Complète ce que les autres ont expliqué. Utilise des exemples, analogies ou décompositions étape par étape. Corrige les malentendus avec bienveillance.",

        (DiscussionMode::CritiqueReview, "en") => "Build on or challenge others' assessments. Balance praise and criticism. Suggest specific improvements.",
        (DiscussionMode::CritiqueReview, "zh") => "在他人评估基础上构建或提出挑战。平衡赞扬和批评。建议具体改进。",
        (DiscussionMode::CritiqueReview, _) => "Construis sur les évaluations des autres ou conteste-les. Équilibre éloge et critique. Propose des améliorations spécifiques.",

        (DiscussionMode::CollaborativeFiction, "en") => "Continue the story where the previous writer stopped. You MUST advance the plot concretely: introduce a new event, a character action, a revelation, or a turning point. Do NOT write purely atmospheric descriptions — something must HAPPEN. Never insert co-authors as characters.",
        (DiscussionMode::CollaborativeFiction, "zh") => "从上一位作者停笔处继续故事。你必须具体推进情节：引入新事件、角色行动、揭示或转折点。不要写纯粹的氛围描写——必须有事情发生。绝不将共同作者作为角色插入。",
        (DiscussionMode::CollaborativeFiction, _) => "Continue l'histoire là où l'auteur précédent s'est arrêté. Tu DOIS faire avancer l'intrigue concrètement : introduis un nouvel événement, une action de personnage, une révélation ou un retournement. N'écris PAS de descriptions purement atmosphériques — il doit se PASSER quelque chose. N'insère jamais les co-auteurs comme personnages.",
    }
}

/// Returns the fundamental constraint for the mode — repeated at end of prompt (recency bias).
pub fn mode_key_constraint(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Be persuasive. Use evidence and reasoning.",
        (DiscussionMode::Debate, "zh") => "要有说服力。使用证据和推理。",
        (DiscussionMode::Debate, _) => "Sois persuasif. Utilise des preuves et du raisonnement.",

        (DiscussionMode::Ideation, "en") => "Focus on generating ideas. No criticism or evaluation.",
        (DiscussionMode::Ideation, "zh") => "专注于产生想法。不要批评或评判。",
        (DiscussionMode::Ideation, _) => "Concentre-toi sur la génération d'idées. Pas de critique ni d'évaluation.",

        (DiscussionMode::CoConstruction, "en") => "Discuss and improve the document. Your ideas here will be formalized separately.",
        (DiscussionMode::CoConstruction, "zh") => "讨论并改进文档。你在这里的想法将被单独正式化。",
        (DiscussionMode::CoConstruction, _) => "Discute et améliore le document. Tes idées ici seront formalisées séparément.",

        (DiscussionMode::UserDriven, "en") => "Stay aligned with the user's guidance.",
        (DiscussionMode::UserDriven, "zh") => "与用户的指引保持一致。",
        (DiscussionMode::UserDriven, _) => "Reste aligné avec les directives de l'utilisateur.",

        (DiscussionMode::Socratic, "en") => "Seek understanding. Do not defend a fixed position.",
        (DiscussionMode::Socratic, "zh") => "寻求理解。不要捍卫固定立场。",
        (DiscussionMode::Socratic, _) => "Cherche à comprendre. Ne défends pas une position fixe.",

        (DiscussionMode::Tutorial, "en") => "Prioritize clarity and pedagogical effectiveness.",
        (DiscussionMode::Tutorial, "zh") => "优先考虑清晰度和教学效果。",
        (DiscussionMode::Tutorial, _) => "Privilégie la clarté et l'efficacité pédagogique.",

        (DiscussionMode::CritiqueReview, "en") => "Stay constructive and specific in your feedback.",
        (DiscussionMode::CritiqueReview, "zh") => "在反馈中保持建设性和具体性。",
        (DiscussionMode::CritiqueReview, _) => "Reste constructif et spécifique dans ton feedback.",

        (DiscussionMode::CollaborativeFiction, "en") => "You are an INVISIBLE narrator — NEVER insert yourself or other co-authors as characters. Advance the plot: something NEW must happen. NEVER restart or repeat.",
        (DiscussionMode::CollaborativeFiction, "zh") => "你是隐形叙述者——绝不将自己或其他共同作者作为角色插入。推进情节：必须发生新的事情。绝不重新开始或重复。",
        (DiscussionMode::CollaborativeFiction, _) => "Tu es un narrateur INVISIBLE — n'insère JAMAIS ton nom ni celui des co-auteurs comme personnages. Fais avancer l'intrigue : quelque chose de NOUVEAU doit se passer. Ne recommence JAMAIS et ne répète pas.",
    }
}

/// Composes a PE-optimized intervention instruction using mode data + context template.
///
/// Architecture: mode-specific data (opening_action, engage_action, key_constraint) is
/// injected into shared context templates (Opening, Turn1, UserSpoke, FirstOfTurn, General).
/// PE techniques applied: visual delimiters, concrete action verbs, prompt repetition (REMEMBER).
pub fn mode_context_instruction(
    mode: &DiscussionMode,
    lang: &str,
    context: InterventionContext,
    user_name: &str,
    end_awareness: &str,
) -> String {
    let opening = mode_opening_action(mode, lang);
    let engage = mode_engage_action(mode, lang);
    let constraint = mode_key_constraint(mode, lang);
    let is_user_driven = *mode == DiscussionMode::UserDriven;
    let is_user_participant = is_user_driven
        || *mode == DiscussionMode::CollaborativeFiction;

    // User clause for non-UserSpoke contexts — skip for modes where user is a participant
    let user_clause = if is_user_participant {
        String::new()
    } else {
        format!("{}\n", user_observer_clause(lang, user_name))
    };

    match (context, lang) {
        // ── Opening ──────────────────────────────────────────────────────
        (InterventionContext::Opening, "en") => format!(
            "=== YOUR TASK ===\n\
             This is the OPENING ROUND.\n\
             {opening}\n\
             Do not introduce yourself or state your role — jump straight in.\n\
             {constraint}\n\
             Keep it to one focused paragraph.\n\
             {user_clause}\
             {end_awareness}\n\
             REMEMBER: {constraint}"
        ),
        (InterventionContext::Opening, "zh") => format!(
            "=== 你的任务 ===\n\
             这是开场轮。\n\
             {opening}\n\
             不要自我介绍或说明你的角色——直接切入。\n\
             {constraint}\n\
             保持一段集中的论述。\n\
             {user_clause}\
             {end_awareness}\n\
             记住：{constraint}"
        ),
        (InterventionContext::Opening, _) => format!(
            "=== VOTRE TÂCHE ===\n\
             C'est le TOUR D'OUVERTURE.\n\
             {opening}\n\
             Ne te présente pas et ne décris pas ton rôle — entre directement dans le vif.\n\
             {constraint}\n\
             Reste sur un paragraphe concentré.\n\
             {user_clause}\
             {end_awareness}\n\
             RAPPEL : {constraint}"
        ),

        // ── Turn1 (other speakers) ───────────────────────────────────────
        (InterventionContext::Turn1, "en") => format!(
            "=== YOUR TASK ===\n\
             This is the OPENING ROUND — share YOUR OWN perspective with a distinctive angle.\n\
             {opening}\n\
             CRITICAL: Do NOT react to, quote, paraphrase, or reference what other speakers have said. \
             Present YOUR OWN independent position as if you were the first to speak.\n\
             Focus on what YOU think — the interaction phase begins next round.\n\
             {constraint}\n\
             Keep it to one focused paragraph.\n\
             {user_clause}\
             {end_awareness}\n\
             REMEMBER: {constraint}"
        ),
        (InterventionContext::Turn1, "zh") => format!(
            "=== 你的任务 ===\n\
             这是开场轮——以独特的角度分享你自己的观点。\n\
             {opening}\n\
             关键：不要回应、引用、改述或提及其他发言者所说的内容。\
             像你是第一个发言一样，展示你自己的独立立场。\n\
             专注于你自己的想法——互动阶段从下一轮开始。\n\
             {constraint}\n\
             保持一段集中的论述。\n\
             {user_clause}\
             {end_awareness}\n\
             记住：{constraint}"
        ),
        (InterventionContext::Turn1, _) => format!(
            "=== VOTRE TÂCHE ===\n\
             C'est le TOUR D'OUVERTURE — partage TA PROPRE perspective avec un angle distinctif.\n\
             {opening}\n\
             CRITIQUE : Ne réagis PAS à ce que les autres ont dit. Ne cite PAS, ne paraphrase PAS, \
             ne fais PAS référence aux interventions précédentes. \
             Présente TA PROPRE position indépendante comme si tu étais le premier à parler.\n\
             Concentre-toi sur ce que TU penses — la phase d'interaction commence au prochain tour.\n\
             {constraint}\n\
             Reste sur un paragraphe concentré.\n\
             {user_clause}\
             {end_awareness}\n\
             RAPPEL : {constraint}"
        ),

        // ── UserSpoke ────────────────────────────────────────────────────
        (InterventionContext::UserSpoke, "en") => {
            let user_line = if is_user_driven {
                format!("{} shared a message — respond to their direction.", user_name)
            } else if *mode == DiscussionMode::CollaborativeFiction {
                format!("{} wrote a story segment — continue the story from where they left off.", user_name)
            } else {
                format!("{} shared a comment — acknowledge it if relevant, but focus on other participants.", user_name)
            };
            format!(
                "=== YOUR TASK ===\n\
                 {user_line}\n\
                 {engage}\n\
                 {constraint}\n\
                 Keep it to one or two focused paragraphs — do not pad or repeat yourself.\n\
                 {end_awareness}\n\
                 REMEMBER: {constraint}"
            )
        },
        (InterventionContext::UserSpoke, "zh") => {
            let user_line = if is_user_driven {
                format!("{}发了一条消息——回应他们的指引。", user_name)
            } else if *mode == DiscussionMode::CollaborativeFiction {
                format!("{}写了一段故事——从他们停笔的地方继续。", user_name)
            } else {
                format!("{}发了一条评论——如果相关可以提及，但主要集中于其他参与者。", user_name)
            };
            format!(
                "=== 你的任务 ===\n\
                 {user_line}\n\
                 {engage}\n\
                 {constraint}\n\
                 保持一到两段集中的论述——不要填充或重复。\n\
                 {end_awareness}\n\
                 记住：{constraint}"
            )
        },
        (InterventionContext::UserSpoke, _) => {
            let user_line = if is_user_driven {
                format!("{} a partagé un message — réponds à sa direction.", user_name)
            } else if *mode == DiscussionMode::CollaborativeFiction {
                format!("{} a écrit un segment de l'histoire — continue le récit là où il s'est arrêté.", user_name)
            } else {
                format!("{} a partagé un commentaire — mentionne-le si pertinent, mais concentre-toi sur les autres participants.", user_name)
            };
            format!(
                "=== VOTRE TÂCHE ===\n\
                 {user_line}\n\
                 {engage}\n\
                 {constraint}\n\
                 Tiens-toi à un ou deux paragraphes — ne meuble pas et ne te répète pas.\n\
                 {end_awareness}\n\
                 RAPPEL : {constraint}"
            )
        },

        // ── FirstOfTurn ──────────────────────────────────────────────────
        (InterventionContext::FirstOfTurn, "en") => format!(
            "=== YOUR TASK ===\n\
             You open this turn. React to the previous round — pick what struck you most and engage with it.\n\
             {engage}\n\
             {constraint}\n\
             Keep it to one or two paragraphs.\n\
             {user_clause}\
             {end_awareness}\n\
             REMEMBER: {constraint}"
        ),
        (InterventionContext::FirstOfTurn, "zh") => format!(
            "=== 你的任务 ===\n\
             你是本轮第一个发言。回应上一轮——选择最让你印象深刻的内容并回应。\n\
             {engage}\n\
             {constraint}\n\
             保持一到两段。\n\
             {user_clause}\
             {end_awareness}\n\
             记住：{constraint}"
        ),
        (InterventionContext::FirstOfTurn, _) => format!(
            "=== VOTRE TÂCHE ===\n\
             Tu ouvres ce tour. Réagis au tour précédent — choisis ce qui t'a le plus frappé et confronte-le.\n\
             {engage}\n\
             {constraint}\n\
             Tiens-toi à un ou deux paragraphes.\n\
             {user_clause}\
             {end_awareness}\n\
             RAPPEL : {constraint}"
        ),

        // ── General ──────────────────────────────────────────────────────
        (InterventionContext::General, "en") => format!(
            "=== YOUR TASK ===\n\
             {engage}\n\
             Move forward with a new angle — do not just restate your previous contributions.\n\
             {constraint}\n\
             Keep it to one or two paragraphs.\n\
             {user_clause}\
             {end_awareness}\n\
             REMEMBER: {constraint}"
        ),
        (InterventionContext::General, "zh") => format!(
            "=== 你的任务 ===\n\
             {engage}\n\
             以新角度前进——不要只是重复你之前的贡献。\n\
             {constraint}\n\
             保持一到两段。\n\
             {user_clause}\
             {end_awareness}\n\
             记住：{constraint}"
        ),
        (InterventionContext::General, _) => format!(
            "=== VOTRE TÂCHE ===\n\
             {engage}\n\
             Avance avec un nouvel angle — ne te contente pas de répéter tes contributions précédentes.\n\
             {constraint}\n\
             Tiens-toi à un ou deux paragraphes.\n\
             {user_clause}\
             {end_awareness}\n\
             RAPPEL : {constraint}"
        ),
    }
}

/// Returns mode-aware (like_meaning, dislike_meaning) for reaction prompts.
pub fn mode_reaction_meanings(mode: &DiscussionMode, lang: &str) -> (&'static str, &'static str) {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => ("agree or strong argument", "disagree or weak argument"),
        (DiscussionMode::Debate, "zh") => ("同意或有力论点", "不同意或薄弱论点"),
        (DiscussionMode::Debate, _) => ("d'accord ou argument fort", "en désaccord ou argument faible"),

        (DiscussionMode::Ideation, "en") => ("promising idea worth building on", "idea needs rethinking or is off-track"),
        (DiscussionMode::Ideation, "zh") => ("值得发展的有前景想法", "想法需要重新考虑或偏离方向"),
        (DiscussionMode::Ideation, _) => ("idée prometteuse à développer", "idée à repenser ou hors-piste"),

        (DiscussionMode::CoConstruction, "en") => ("advances the shared goal", "misaligned or needs rework"),
        (DiscussionMode::CoConstruction, "zh") => ("推进共同目标", "不一致或需要重做"),
        (DiscussionMode::CoConstruction, _) => ("fait avancer l'objectif commun", "décalé ou à retravailler"),

        (DiscussionMode::UserDriven, "en") => ("relevant and helpful response", "off-topic or unhelpful"),
        (DiscussionMode::UserDriven, "zh") => ("相关且有帮助的回应", "偏题或无帮助"),
        (DiscussionMode::UserDriven, _) => ("réponse pertinente et utile", "hors-sujet ou peu utile"),

        (DiscussionMode::Socratic, "en") => ("thought-provoking, deepens inquiry", "superficial or misses the point"),
        (DiscussionMode::Socratic, "zh") => ("发人深省，深化探究", "肤浅或偏离要点"),
        (DiscussionMode::Socratic, _) => ("stimulant, approfondit l'enquête", "superficiel ou à côté du sujet"),

        (DiscussionMode::Tutorial, "en") => ("clear and pedagogically effective", "confusing or incomplete"),
        (DiscussionMode::Tutorial, "zh") => ("清晰且教学有效", "令人困惑或不完整"),
        (DiscussionMode::Tutorial, _) => ("clair et pédagogiquement efficace", "confus ou incomplet"),

        (DiscussionMode::CritiqueReview, "en") => ("well-founded constructive assessment", "unfair or vague critique"),
        (DiscussionMode::CritiqueReview, "zh") => ("有根据的建设性评估", "不公平或模糊的批评"),
        (DiscussionMode::CritiqueReview, _) => ("évaluation constructive et fondée", "critique injuste ou vague"),

        (DiscussionMode::CollaborativeFiction, "en") => ("seamless and engaging continuation", "breaks narrative flow or is incoherent"),
        (DiscussionMode::CollaborativeFiction, "zh") => ("流畅且引人入胜的延续", "打破叙事流或不连贯"),
        (DiscussionMode::CollaborativeFiction, _) => ("continuation fluide et captivante", "brise le flux narratif ou incohérent"),
    }
}

/// Returns a PE-optimized mode override clause for the system message.
/// Empty for Debate (no bias to override). For other modes: visual delimiter + affirmative framing.
pub fn mode_override_clause(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, _) => "",

        (DiscussionMode::Ideation, "en") => "=== DISCUSSION FORMAT: BRAINSTORMING ===\nThis session is about generating and combining creative ideas freely. Arguing positions or critiquing ideas is off-track for this format.",
        (DiscussionMode::Ideation, "zh") => "=== 讨论格式：头脑风暴 ===\n本次会议旨在自由产生和组合创意。争论立场或批评想法不符合此格式。",
        (DiscussionMode::Ideation, _) => "=== FORMAT DE DISCUSSION : BRAINSTORMING ===\nCette session vise à générer et combiner des idées créatives librement. Argumenter des positions ou critiquer des idées est hors-cadre pour ce format.",

        (DiscussionMode::CoConstruction, "en") => "=== DISCUSSION FORMAT: COLLABORATIVE CONSTRUCTION ===\nThis discussion is for deliberation: critique proposals, suggest improvements, debate structure and content for the shared document. The document itself is updated separately — your role here is to discuss, not to write the document directly.",
        (DiscussionMode::CoConstruction, "zh") => "=== 讨论格式：协作构建 ===\n本讨论用于审议：批评提案、建议改进、讨论共享文档的结构和内容。文档本身会单独更新——你在这里的角色是讨论，而不是直接编写文档。",
        (DiscussionMode::CoConstruction, _) => "=== FORMAT DE DISCUSSION : CO-CONSTRUCTION ===\nCette discussion sert à la délibération : critiquer les propositions, suggérer des améliorations, débattre la structure et le contenu du document partagé. Le document est mis à jour séparément — ton rôle ici est de discuter, pas de rédiger directement le document.",

        (DiscussionMode::UserDriven, "en") => "=== DISCUSSION FORMAT: USER-GUIDED EXCHANGE ===\nThis session is guided by the user's questions and direction. Focus on serving the user's needs.",
        (DiscussionMode::UserDriven, "zh") => "=== 讨论格式：用户引导交流 ===\n本次会议由用户的问题和方向引导。专注于服务用户的需求。",
        (DiscussionMode::UserDriven, _) => "=== FORMAT DE DISCUSSION : ÉCHANGES GUIDÉS PAR L'UTILISATEUR ===\nCette session est guidée par les questions et la direction de l'utilisateur. Concentre-toi sur les besoins de l'utilisateur.",

        (DiscussionMode::Socratic, "en") => "=== DISCUSSION FORMAT: SOCRATIC INQUIRY ===\nThis session is about questioning assumptions and deepening understanding through dialogue. Defending fixed positions is off-track.",
        (DiscussionMode::Socratic, "zh") => "=== 讨论格式：苏格拉底式探究 ===\n本次会议旨在通过对话质疑假设和加深理解。捍卫固定立场不符合方向。",
        (DiscussionMode::Socratic, _) => "=== FORMAT DE DISCUSSION : QUESTIONNEMENT SOCRATIQUE ===\nCette session vise à questionner les hypothèses et approfondir la compréhension par le dialogue. Défendre des positions fixes est hors-cadre.",

        (DiscussionMode::Tutorial, "en") => "=== DISCUSSION FORMAT: TUTORIAL PANEL ===\nThis session is about teaching clearly, using examples, and building on each other's explanations. Arguing or debating is off-track.",
        (DiscussionMode::Tutorial, "zh") => "=== 讨论格式：教程面板 ===\n本次会议旨在清晰教学、使用例子并互相补充解释。争论或辩论不符合方向。",
        (DiscussionMode::Tutorial, _) => "=== FORMAT DE DISCUSSION : PANEL TUTORIEL ===\nCette session vise à enseigner clairement, utiliser des exemples et construire sur les explications des autres. Argumenter ou débattre est hors-cadre.",

        (DiscussionMode::CritiqueReview, "en") => "=== DISCUSSION FORMAT: CRITIQUE & REVIEW ===\nThis session is about balanced, constructive assessment with specific feedback. Turning it into a debate is off-track.",
        (DiscussionMode::CritiqueReview, "zh") => "=== 讨论格式：评审与评论 ===\n本次会议旨在进行平衡、建设性的评估和具体反馈。将其变成辩论不符合方向。",
        (DiscussionMode::CritiqueReview, _) => "=== FORMAT DE DISCUSSION : CRITIQUE / REVIEW ===\nCette session vise une évaluation équilibrée et constructive avec du feedback spécifique. La transformer en débat est hors-cadre.",

        (DiscussionMode::CollaborativeFiction, "en") => "=== DISCUSSION FORMAT: COLLABORATIVE FICTION ===\nThis is a relay-written story. Each co-author continues the narrative where the previous one stopped.\nIMPORTANT: Co-authors are INVISIBLE narrators, NOT characters in the story. NEVER insert your name or other co-authors' names as characters. The characters are those created IN the story by the writers.\nWrite the next segment: advance the plot with a concrete event, action, or revelation. Do NOT write purely atmospheric text. Do NOT comment on, discuss, or summarize the story.",
        (DiscussionMode::CollaborativeFiction, "zh") => "=== 讨论格式：协作小说 ===\n这是一个接力写作故事。每位共同作者从上一位停笔处继续叙事。\n重要：共同作者是隐形叙述者，不是故事中的角色。绝不将你的名字或其他共同作者的名字作为角色插入。角色是作者们在故事中创造的。\n写下一段：用具体的事件、行动或揭示推进情节。不要写纯粹的氛围文字。不要评论、讨论或总结故事。",
        (DiscussionMode::CollaborativeFiction, _) => "=== FORMAT DE DISCUSSION : FICTION COLLABORATIVE ===\nC'est une histoire écrite en relais. Chaque co-auteur continue le récit là où le précédent s'est arrêté.\nIMPORTANT : Les co-auteurs sont des narrateurs INVISIBLES, PAS des personnages de l'histoire. N'insère JAMAIS ton nom ni celui des autres co-auteurs comme personnages. Les personnages sont ceux créés DANS l'histoire par les auteurs.\nÉcris le prochain segment : fais avancer l'intrigue avec un événement concret, une action ou une révélation. N'écris PAS de texte purement atmosphérique. Ne commente PAS, ne discute pas et ne résume pas l'histoire.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_modes() -> Vec<DiscussionMode> {
        vec![
            DiscussionMode::Debate,
            DiscussionMode::Ideation,
            DiscussionMode::CoConstruction,
            DiscussionMode::UserDriven,
            DiscussionMode::Socratic,
            DiscussionMode::Tutorial,
            DiscussionMode::CritiqueReview,
            DiscussionMode::CollaborativeFiction,
        ]
    }

    fn all_langs() -> Vec<&'static str> {
        vec!["en", "fr", "zh"]
    }

    #[test]
    fn test_mode_opening_action_all_modes() {
        for mode in all_modes() {
            for lang in all_langs() {
                let result = mode_opening_action(&mode, lang);
                assert!(!result.is_empty(), "Empty opening action for {mode:?}/{lang}");
            }
        }
    }

    #[test]
    fn test_mode_engage_action_all_modes() {
        for mode in all_modes() {
            for lang in all_langs() {
                let result = mode_engage_action(&mode, lang);
                assert!(!result.is_empty(), "Empty engage action for {mode:?}/{lang}");
            }
        }
    }

    #[test]
    fn test_mode_key_constraint_all_modes() {
        for mode in all_modes() {
            for lang in all_langs() {
                let result = mode_key_constraint(&mode, lang);
                assert!(!result.is_empty(), "Empty key constraint for {mode:?}/{lang}");
            }
        }
    }

    #[test]
    fn test_mode_context_instruction_all_modes_all_contexts() {
        let contexts = vec![
            InterventionContext::Opening,
            InterventionContext::Turn1,
            InterventionContext::UserSpoke,
            InterventionContext::FirstOfTurn,
            InterventionContext::General,
        ];
        for mode in all_modes() {
            for lang in all_langs() {
                for context in &contexts {
                    let result = mode_context_instruction(
                        &mode, lang, *context, "TestUser", "",
                    );
                    assert!(!result.is_empty(), "Empty context instruction for {mode:?}/{lang}");
                    // Check PE delimiter is present
                    let has_delimiter = result.contains("=== YOUR TASK ===")
                        || result.contains("=== VOTRE TÂCHE ===")
                        || result.contains("=== 你的任务 ===");
                    assert!(has_delimiter, "Missing task delimiter for {mode:?}/{lang}: {result}");
                }
            }
        }
    }

    #[test]
    fn test_mode_context_instruction_contains_remember() {
        // Verify REMEMBER/RAPPEL/记住 prompt repetition is present
        for mode in all_modes() {
            let result = mode_context_instruction(
                &mode, "en", InterventionContext::General, "TestUser", "",
            );
            assert!(result.contains("REMEMBER:"), "Missing REMEMBER for {mode:?}/en: {result}");

            let result_fr = mode_context_instruction(
                &mode, "fr", InterventionContext::General, "TestUser", "",
            );
            assert!(result_fr.contains("RAPPEL"), "Missing RAPPEL for {mode:?}/fr");

            let result_zh = mode_context_instruction(
                &mode, "zh", InterventionContext::General, "TestUser", "",
            );
            assert!(result_zh.contains("记住"), "Missing 记住 for {mode:?}/zh");
        }
    }

    #[test]
    fn test_mode_reaction_meanings_all_modes() {
        for mode in all_modes() {
            for lang in all_langs() {
                let (like, dislike) = mode_reaction_meanings(&mode, lang);
                assert!(!like.is_empty(), "Empty like meaning for {mode:?}/{lang}");
                assert!(!dislike.is_empty(), "Empty dislike meaning for {mode:?}/{lang}");
            }
        }
    }

    #[test]
    fn test_mode_override_clause_debate_empty() {
        for lang in all_langs() {
            let result = mode_override_clause(&DiscussionMode::Debate, lang);
            assert!(result.is_empty(), "Debate override should be empty, got: {result}");
        }
    }

    #[test]
    fn test_mode_override_clause_non_debate_format() {
        let non_debate = vec![
            DiscussionMode::Ideation,
            DiscussionMode::CoConstruction,
            DiscussionMode::UserDriven,
            DiscussionMode::Socratic,
            DiscussionMode::Tutorial,
            DiscussionMode::CritiqueReview,
            DiscussionMode::CollaborativeFiction,
        ];
        for mode in non_debate {
            let result = mode_override_clause(&mode, "en");
            assert!(
                result.contains("=== DISCUSSION FORMAT"),
                "Missing format delimiter for {mode:?}: {result}"
            );
        }
    }

    #[test]
    fn test_mode_moderation_criteria_redirect() {
        let non_debate = vec![
            DiscussionMode::Ideation,
            DiscussionMode::CoConstruction,
            DiscussionMode::UserDriven,
            DiscussionMode::Socratic,
            DiscussionMode::Tutorial,
            DiscussionMode::CritiqueReview,
            DiscussionMode::CollaborativeFiction,
        ];
        for mode in &non_debate {
            let en = mode_moderation_criteria(mode, "en");
            assert!(en.contains("redirect"), "Missing 'redirect' in EN moderation for {mode:?}");

            let fr = mode_moderation_criteria(mode, "fr");
            assert!(fr.contains("recadre"), "Missing 'recadre' in FR moderation for {mode:?}");

            let zh = mode_moderation_criteria(mode, "zh");
            assert!(zh.contains("引导"), "Missing '引导' in ZH moderation for {mode:?}");
        }
    }
}
