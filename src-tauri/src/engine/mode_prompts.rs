use super::focus::Focus;
use crate::constants;
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

        (DiscussionMode::Trial, "en") => "trial",
        (DiscussionMode::Trial, "zh") => "审判",
        (DiscussionMode::Trial, _) => "procès",

        (DiscussionMode::OxfordDebate, "en") => "Oxford-style debate",
        (DiscussionMode::OxfordDebate, "zh") => "牛津式辩论",
        (DiscussionMode::OxfordDebate, _) => "débat d'Oxford",

        (DiscussionMode::Negotiation, "en") => "negotiation",
        (DiscussionMode::Negotiation, "zh") => "谈判",
        (DiscussionMode::Negotiation, _) => "négociation",

        (DiscussionMode::SixHats, "en") => "six thinking hats session",
        (DiscussionMode::SixHats, "zh") => "六顶思考帽",
        (DiscussionMode::SixHats, _) => "session des six chapeaux",

        (DiscussionMode::CrisisCell, "en") => "crisis cell",
        (DiscussionMode::CrisisCell, "zh") => "危机小组",
        (DiscussionMode::CrisisCell, _) => "cellule de crise",
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
        (DiscussionMode::CollaborativeFiction, "en") => "Explain briefly that this is a relay-written story: the user is invited to write the opening (a co-author does it otherwise), then each co-author continues in sequence. Encourage seamless transitions and narrative coherence.",
        (DiscussionMode::CollaborativeFiction, "zh") => "简要解释这是接力写作故事：邀请用户写开头（否则由一位共同作者来写），然后每位共同作者按顺序继续。鼓励无缝过渡和叙事连贯。",
        (DiscussionMode::CollaborativeFiction, _) => "Explique brièvement que c'est une histoire écrite en relais : l'utilisateur est invité à écrire l'ouverture (sinon un co-auteur s'en charge), puis chaque co-auteur continue à la suite. Encourage les transitions fluides et la cohérence narrative.",

        // Trial
        (DiscussionMode::Trial, "en") => "Open the hearing: restate the question on trial (the topic), present the roles — prosecution, defence, witnesses, jurors — and the rules: everyone speaks in turn, the jurors listen and question, the verdict comes at the end. Give the floor to the prosecution.",
        (DiscussionMode::Trial, "zh") => "开庭：重申受审的问题（主题），介绍各方角色——控方、辩方、证人、陪审员——以及规则：依次发言，陪审员倾听并提问，裁决在最后作出。请控方发言。",
        (DiscussionMode::Trial, _) => "Ouvre l'audience : rappelle la question jugée (le sujet), présente les rôles — accusation, défense, témoins, jurés — et les règles : chacun parle à son tour, les jurés écoutent et questionnent, le verdict viendra à la fin. Donne la parole à l'accusation.",

        // Oxford debate
        (DiscussionMode::OxfordDebate, "en") => "Open the Oxford debate: state the motion (the topic) exactly as it will be voted, present the camp For and the camp Against, remind everyone that the audience votes before and after and that the camp moving the most votes wins. Invite the first speaker of the camp For.",
        (DiscussionMode::OxfordDebate, "zh") => "开启牛津式辩论：按将要表决的措辞陈述辩题（主题），介绍正方和反方，提醒大家听众在辩论前后投票，争取到最多改变票数的一方获胜。请正方第一位辩手发言。",
        (DiscussionMode::OxfordDebate, _) => "Ouvre le débat d'Oxford : énonce la motion (le sujet) telle qu'elle sera votée, présente le camp Pour et le camp Contre, rappelle que le public vote avant et après et que le camp qui déplace le plus de voix l'emporte. Invite le premier orateur du camp Pour.",

        // Negotiation
        (DiscussionMode::Negotiation, "en") => "Open the negotiation: restate what is at stake (the topic), present the parties and their apparent interests, set the frame — an agreement acceptable to all, opening positions first, bargaining next. Invite each party to table its opening offer.",
        (DiscussionMode::Negotiation, "zh") => "开启谈判：重申谈判对象（主题），介绍各方及其表面利益，设定框架——寻求各方都能接受的协议，先陈述开局立场，再讨价还价。请各方提出开局报价。",
        (DiscussionMode::Negotiation, _) => "Ouvre la négociation : rappelle l'objet (le sujet), présente les parties et leurs intérêts apparents, fixe le cadre — recherche d'un accord acceptable par tous, positions d'ouverture d'abord, marchandage ensuite. Invite chaque partie à poser son offre d'ouverture.",

        // Six hats
        (DiscussionMode::SixHats, "en") => "Open the six-hats session: restate the question (the topic), explain that every turn each participant wears a hat that dictates one way of thinking (facts, feelings, caution, benefits, creativity, process) and that the hats rotate every turn. Ask for the first contributions, each strictly under their hat.",
        (DiscussionMode::SixHats, "zh") => "开启六顶思考帽会议：重申问题（主题），说明每一轮每位参与者都戴一顶帽子，它规定一种思维方式（事实、情感、谨慎、益处、创意、流程），并且帽子每轮轮换。请大家各自严格按帽子作出第一轮贡献。",
        (DiscussionMode::SixHats, _) => "Ouvre la session des six chapeaux : rappelle la question (le sujet), explique qu'à chaque tour chaque participant porte un chapeau qui impose un mode de pensée (faits, émotions, prudence, bénéfices, créativité, processus) et que les chapeaux tournent à chaque tour. Demande les premières contributions, chacune strictement sous son chapeau.",

        // Crisis cell
        (DiscussionMode::CrisisCell, "en") => "Open the crisis cell: lay out the situation (the topic) as an unfolding crisis, present the members of the cell and their expertise, set the rule — dispatches will come in every turn, everyone must decide and act, not merely analyse. Ask for a first assessment.",
        (DiscussionMode::CrisisCell, "zh") => "启动危机小组：把局势（主题）作为正在发生的危机来陈述，介绍小组成员及其专长，设定规则——每一轮都会有急电传来，每个人都必须决定并行动，而不只是分析。请大家给出首次评估。",
        (DiscussionMode::CrisisCell, _) => "Ouvre la cellule de crise : expose la situation (le sujet) comme une crise en cours, présente les membres de la cellule et leurs expertises, fixe la règle — des dépêches tomberont à chaque tour, chacun doit décider et agir, pas seulement analyser. Demande un premier état des lieux.",
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

        (DiscussionMode::Trial, "en") => "Hold your role in the trial: the prosecution accuses and proves, the defence contests and protects, the witness reports what they know without pleading, the juror listens, questions and reserves judgement. Address the court.",
        (DiscussionMode::Trial, "zh") => "在审判中坚守你的角色：控方指控并举证，辩方质疑并保护，证人陈述所知而不辩护，陪审员倾听、提问并保留裁决。向法庭发言。",
        (DiscussionMode::Trial, _) => "Tiens ton rôle dans le procès : l'accusation accuse et prouve, la défense conteste et protège, le témoin rapporte ce qu'il sait sans plaider, le juré écoute, questionne et se réserve. Parle à la cour.",

        (DiscussionMode::OxfordDebate, "en") => "Defend your camp on the motion, never switching sides. Aim at the audience: they are the ones you must win over, not your opponent. Rebut point by point, then push forward.",
        (DiscussionMode::OxfordDebate, "zh") => "为你的阵营捍卫辩题，绝不换边。面向听众：你要争取的是他们，而不是对手。逐点反驳，然后推进。",
        (DiscussionMode::OxfordDebate, _) => "Défends ton camp sur la motion, sans jamais en changer. Vise le public : c'est lui que tu dois faire basculer, pas ton adversaire. Réfute point par point, puis avance.",

        (DiscussionMode::Negotiation, "en") => "Negotiate for your party: defend your interests, not only your positions. Make conditional offers, get something for every concession, seek the agreement you could sign.",
        (DiscussionMode::Negotiation, "zh") => "为你的一方谈判：捍卫你的利益，而不只是立场。提出有条件的报价，每一次让步都要换取回报，寻求你能签署的协议。",
        (DiscussionMode::Negotiation, _) => "Négocie pour ta partie : défends tes intérêts, pas seulement tes positions. Fais des offres conditionnelles, obtiens quelque chose pour chaque concession, cherche l'accord que tu pourras signer.",

        (DiscussionMode::SixHats, "en") => "Think only with the hat you wear this turn, even against your temperament. One mode of thinking at a time: that is the rule of the game.",
        (DiscussionMode::SixHats, "zh") => "只用你本轮所戴的帽子思考，哪怕它与你的性情相悖。一次只用一种思维方式：这是游戏规则。",
        (DiscussionMode::SixHats, _) => "Pense uniquement avec le chapeau que tu portes ce tour, même s'il va contre ton tempérament. Un seul mode de pensée à la fois : c'est la règle du jeu.",

        (DiscussionMode::CrisisCell, "en") => "You sit in a crisis cell: react to the latest dispatch, decide, propose concrete and prioritised actions, own the uncertainty. No analysis without a decision.",
        (DiscussionMode::CrisisCell, "zh") => "你身处危机小组：回应最新急电，作出决定，提出具体且有优先级的行动，承担不确定性。没有决定的分析毫无意义。",
        (DiscussionMode::CrisisCell, _) => "Tu es en cellule de crise : réagis à la dernière dépêche, décide, propose des actions concrètes et priorisées, assume l'incertitude. Pas d'analyse sans décision.",
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

            (DiscussionMode::Trial, "en") => "What is my role, and which decisive fact must I establish or contest from the start?",
            (DiscussionMode::Trial, "zh") => "我的角色是什么？我必须从一开始就确立或质疑的决定性事实是什么？",
            (DiscussionMode::Trial, _) => "Quel est mon rôle et quel est le fait décisif que je dois établir ou contester d'entrée ?",

            (DiscussionMode::OxfordDebate, "en") => "What is the most convincing argument for my camp in the eyes of the audience?",
            (DiscussionMode::OxfordDebate, "zh") => "在听众眼中，对我方最有说服力的论点是什么？",
            (DiscussionMode::OxfordDebate, _) => "Quel est l'argument le plus convaincant pour mon camp aux yeux du public ?",

            (DiscussionMode::Negotiation, "en") => "What are my real interests, my opening offer and my walk-away line?",
            (DiscussionMode::Negotiation, "zh") => "我的真实利益、开局报价和底线是什么？",
            (DiscussionMode::Negotiation, _) => "Quels sont mes intérêts réels, mon offre d'ouverture et ma limite ?",

            (DiscussionMode::SixHats, "en") => "What does my hat for this turn say about the question?",
            (DiscussionMode::SixHats, "zh") => "我本轮的帽子对这个问题说了什么？",
            (DiscussionMode::SixHats, _) => "Que dit mon chapeau de ce tour sur cette question ?",

            (DiscussionMode::CrisisCell, "en") => "What is the state of the situation and the first decision to take?",
            (DiscussionMode::CrisisCell, "zh") => "局势如何？首先要作出什么决定？",
            (DiscussionMode::CrisisCell, _) => "Quel est l'état de la situation et la première décision à prendre ?",
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

        (DiscussionMode::Trial, "en") => "What has been established? Which testimony or argument must I exploit or dismantle now, given my role?",
        (DiscussionMode::Trial, "zh") => "已经确立了什么？根据我的角色，现在必须利用或拆解哪份证词或论点？",
        (DiscussionMode::Trial, _) => "Qu'est-ce qui a été établi ? Quel témoignage ou argument dois-je exploiter ou démonter maintenant, selon mon rôle ?",

        (DiscussionMode::OxfordDebate, "en") => "Which opposing point lands with the audience, and how do I turn it around?",
        (DiscussionMode::OxfordDebate, "zh") => "对方哪个论点打动了听众？我如何扭转它？",
        (DiscussionMode::OxfordDebate, _) => "Quel point adverse fait mouche auprès du public et comment le retourner ?",

        (DiscussionMode::Negotiation, "en") => "What does the other party really want? Which concession can I trade, and for what?",
        (DiscussionMode::Negotiation, "zh") => "对方真正想要什么？我可以用哪项让步换取什么？",
        (DiscussionMode::Negotiation, _) => "Qu'est-ce que l'autre partie veut vraiment ? Quelle concession puis-je échanger contre quoi ?",

        (DiscussionMode::SixHats, "en") => "Under this turn's hat, what have the others missed?",
        (DiscussionMode::SixHats, "zh") => "戴着本轮的帽子，其他人遗漏了什么？",
        (DiscussionMode::SixHats, _) => "Sous mon chapeau du tour, qu'est-ce que les autres ont manqué ?",

        (DiscussionMode::CrisisCell, "en") => "What does the latest dispatch change? Which action do I decide now, and at what cost?",
        (DiscussionMode::CrisisCell, "zh") => "最新急电改变了什么？我现在决定采取什么行动，代价是什么？",
        (DiscussionMode::CrisisCell, _) => "Que change la dernière dépêche ? Quelle action décider maintenant et à quel coût ?",
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

        (DiscussionMode::Trial, "en") => "Write the hearing report: the charges, the prosecution's and the defence's arguments, what the testimonies established, the jurors' questions, then the verdict returned and its grounds.",
        (DiscussionMode::Trial, "zh") => "撰写庭审报告：罪状、控方与辩方的论点、证词确立了什么、陪审员的提问，然后是作出的裁决及其理由。",
        (DiscussionMode::Trial, _) => "Rédige le compte rendu d'audience : les charges, les arguments de l'accusation et de la défense, ce que les témoignages ont établi, les questions des jurés, puis le verdict rendu et sa motivation.",

        (DiscussionMode::OxfordDebate, "en") => "Write the debate's review: the motion, each camp's best arguments, the turning points, the audience's vote before and after, and the camp that won by moving votes.",
        (DiscussionMode::OxfordDebate, "zh") => "撰写辩论总结：辩题、双方最佳论点、转折点、听众辩论前后的投票，以及通过改变票数获胜的一方。",
        (DiscussionMode::OxfordDebate, _) => "Rédige le bilan du débat : la motion, les meilleurs arguments de chaque camp, les moments de bascule, le vote du public avant et après et le camp qui l'a emporté par déplacement.",

        (DiscussionMode::Negotiation, "en") => "Write the negotiation record: opening positions, concessions traded, points of agreement and deadlock, the final agreement (or its absence) and what each party obtains.",
        (DiscussionMode::Negotiation, "zh") => "撰写谈判纪要：开局立场、交换的让步、达成一致和僵持的要点、最终协议（或未达成协议）以及各方所得。",
        (DiscussionMode::Negotiation, _) => "Rédige le relevé de négociation : positions d'ouverture, concessions échangées, points d'accord et de blocage, l'accord final (ou l'absence d'accord) et ce que chaque partie obtient.",

        (DiscussionMode::SixHats, "en") => "Write the synthesis hat by hat: established facts (white), feelings (red), risks (black), benefits (yellow), new ideas (green), and the process conclusion (blue): what to decide and how.",
        (DiscussionMode::SixHats, "zh") => "按帽子撰写总结：确立的事实（白）、感受（红）、风险（黑）、益处（黄）、新点子（绿），以及流程结论（蓝）：决定什么、如何决定。",
        (DiscussionMode::SixHats, _) => "Rédige la synthèse par chapeau : faits établis (blanc), ressentis (rouge), risques (noir), bénéfices (jaune), idées neuves (vert), et la conclusion de processus (bleu) : que décider et comment.",

        (DiscussionMode::CrisisCell, "en") => "Write the crisis report: timeline of the dispatches, decisions taken and by whom, actions launched, remaining risks, and the lessons for next time.",
        (DiscussionMode::CrisisCell, "zh") => "撰写危机报告：急电时间线、已作出的决定及决策人、已启动的行动、剩余风险，以及今后的经验教训。",
        (DiscussionMode::CrisisCell, _) => "Rédige le rapport de crise : chronologie des dépêches, décisions prises et par qui, actions engagées, risques restants, et les leçons pour la suite.",
    }
}

/// Returns moderation criteria for IArbitre based on the mode.
pub fn mode_moderation_criteria(mode: &DiscussionMode, lang: &str) -> &'static str {
    match (mode, lang) {
        (DiscussionMode::Debate, "en") => "Evaluate: relevance to the topic, constructiveness, respect for others, quality of argumentation. Depth: if a speaker piles up claims without answering the objections made to them, ask in one sentence for an answer on the merits — without imposing a topic.",
        (DiscussionMode::Debate, "zh") => "评估：与主题的相关性、建设性、对他人的尊重、论证质量。深度：如果发言者堆砌主张而不回应针对他们的反驳，用一句话要求就实质作出回应——不强加话题。",
        (DiscussionMode::Debate, _) => "Évalue : pertinence par rapport au sujet, constructivité, respect des autres, qualité de l'argumentation. Profondeur : si un orateur empile des affirmations sans répondre aux objections qui lui ont été faites, demande en une phrase d'y répondre sur le fond — sans imposer de sujet.",

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

        (DiscussionMode::Trial, "en") => "Evaluate: respect of the role (the prosecution accuses, the defence defends, the witness does not plead, the juror does not rule before the end), rigour of the evidence, respect for the court. If a participant steps out of their role or attacks the person rather than the facts, redirect them.",
        (DiscussionMode::Trial, "zh") => "评估：角色的遵守（控方指控、辩方辩护、证人不辩护、陪审员在结束前不裁决）、证据的严谨性、对法庭的尊重。如果参与者脱离角色或攻击个人而非事实，引导他们回到正轨。",
        (DiscussionMode::Trial, _) => "Évalue : respect du rôle (l'accusation accuse, la défense défend, le témoin ne plaide pas, le juré ne tranche pas avant la fin), rigueur des preuves, respect de la cour. Si un participant sort de son rôle ou attaque la personne plutôt que les faits, recadre-le.",

        (DiscussionMode::OxfordDebate, "en") => "Evaluate: loyalty to the camp, persuasive force for the audience, fair rebuttal. If a speaker switches sides, ignores the motion or attacks the opponent rather than their arguments, redirect them.",
        (DiscussionMode::OxfordDebate, "zh") => "评估：对阵营的忠诚、对听众的说服力、公允的反驳。如果辩手换边、无视辩题或攻击对手而非其论点，引导他们回到正轨。",
        (DiscussionMode::OxfordDebate, _) => "Évalue : fidélité au camp, force de persuasion pour le public, réfutation loyale. Si un orateur change de camp, ignore la motion ou attaque l'adversaire plutôt que ses arguments, recadre-le.",

        (DiscussionMode::Negotiation, "en") => "Evaluate: good faith, concrete offers, movement towards an agreement. If a party stonewalls without counterpart, bluffs crudely or attacks the other, redirect them towards the search for an agreement.",
        (DiscussionMode::Negotiation, "zh") => "评估：诚意、具体的报价、向协议推进。如果一方无对价地阻挠、粗暴虚张声势或攻击对方，引导他们回到寻求协议。",
        (DiscussionMode::Negotiation, _) => "Évalue : bonne foi, offres concrètes, mouvement vers l'accord. Si une partie bloque sans contrepartie, bluffe grossièrement ou attaque l'autre, recadre-la vers la recherche d'accord.",

        (DiscussionMode::SixHats, "en") => "Evaluate: strict respect of the hat worn this turn. If a participant thinks with another hat than their own (for instance criticises under the yellow hat), redirect them to their hat.",
        (DiscussionMode::SixHats, "zh") => "评估：是否严格遵守本轮所戴的帽子。如果参与者用别的帽子思考（例如戴着黄帽却在批评），引导他们回到自己的帽子。",
        (DiscussionMode::SixHats, _) => "Évalue : respect strict du chapeau porté ce tour. Si un participant pense avec un autre chapeau que le sien (par exemple critique sous le chapeau jaune), recadre-le vers son chapeau.",

        (DiscussionMode::CrisisCell, "en") => "Evaluate: responsiveness to the dispatches, concrete decisions, coordination with the other members. If a participant analyses without deciding or ignores the latest dispatch, redirect them towards action.",
        (DiscussionMode::CrisisCell, "zh") => "评估：对急电的响应、具体的决定、与其他成员的协调。如果参与者只分析不决定或无视最新急电，引导他们采取行动。",
        (DiscussionMode::CrisisCell, _) => "Évalue : réactivité aux dépêches, décisions concrètes, coordination avec les autres membres. Si un participant analyse sans décider ou ignore la dernière dépêche, recadre-le vers l'action.",
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
    previous_questions: &[String],
) -> String {
    let previous = previous_questions
        .iter()
        .rev()
        .take(constants::SOCRATIC_PREVIOUS_QUESTIONS)
        .map(|q| format!("- {q}"))
        .collect::<Vec<_>>()
        .join("\n");
    let previous_block = if previous.is_empty() {
        String::new()
    } else {
        match lang {
            "en" => format!("Questions you ALREADY asked (do not repeat or rephrase them — explore a different angle):\n{previous}\n\n"),
            "zh" => format!("你已经提过的问题（不要重复或改述——探索不同的角度）：\n{previous}\n\n"),
            _ => format!("Questions que tu as DÉJÀ posées (ne les répète pas et ne les reformule pas — explore un autre angle) :\n{previous}\n\n"),
        }
    };
    match lang {
        "en" => format!(
            "Topic: {topic}\n\
            Recent exchanges:\n{recent_exchanges}\n\n\
            {previous_block}\
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
            {previous_block}\
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
            {previous_block}\
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

/// Who the speaker should address this time (turns ≥ 2, debate-like modes).
/// Shared by the cognitive directive (layer 5) and the mode templates so that
/// both prompt paths rotate attention the same way.
pub fn focus_instruction(focus: Option<&Focus>, lang: &str) -> Option<String> {
    match focus? {
        Focus::Speaker(name) => Some(match lang {
            "en" => format!("Address {name} in priority — pick up one of their points and respond to it directly (agree, challenge or build on it). You may mention others briefly, but {name} is your main interlocutor this time."),
            "zh" => format!("优先回应{name}——抓住其一个观点直接回应（赞同、质疑或延伸）。可以简短提及他人，但这次{name}是你的主要对话者。"),
            _ => format!("Adresse-toi en priorité à {name} — reprends un de ses points et réponds-y directement (accord, contestation ou prolongement). Tu peux évoquer les autres brièvement, mais {name} est ton interlocuteur principal cette fois-ci."),
        }),
        Focus::Topic => Some(match lang {
            "en" => "This time, do not answer anyone in particular: push the topic forward with a fresh angle, a fact or a question nobody has raised yet.".to_string(),
            "zh" => "这次不要回应任何特定的人：用一个新角度、一个事实或一个还没人提出的问题来推进话题。".to_string(),
            _ => "Cette fois, ne réponds à personne en particulier : fais avancer le sujet avec un angle neuf, un fait ou une question que personne n'a encore soulevés.".to_string(),
        }),
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
    /// CollaborativeFiction: nothing has been written yet — write the opening
    FictionOpening,
    /// CollaborativeFiction: a segment exists — continue from the anchor
    FictionContinue,
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

        (DiscussionMode::CollaborativeFiction, "en") => "Continue the story from its opening. Pick up exactly where the previous writer left off with a seamless transition that advances the narrative.",
        (DiscussionMode::CollaborativeFiction, "zh") => "从故事的开头继续。从上一位作者停笔处无缝衔接，推进叙事。",
        (DiscussionMode::CollaborativeFiction, _) => "Continue l'histoire à partir de son ouverture. Reprends exactement là où l'auteur précédent s'est arrêté avec une transition fluide qui fait avancer le récit.",

        (DiscussionMode::Trial, "en") => "Set your role from the start: prosecution → state the charges and your key piece of evidence; defence → contest them and announce your line; witness → say what you saw or know; juror → say what you expect to see established.",
        (DiscussionMode::Trial, "zh") => "一开始就确立你的角色：控方→陈述罪状和关键证据；辩方→质疑并宣布辩护路线；证人→说出你所见或所知；陪审员→说出你期望被确立的事实。",
        (DiscussionMode::Trial, _) => "Pose ton rôle d'entrée : accusation → énonce les charges et la preuve maîtresse ; défense → conteste et annonce ta ligne ; témoin → dis ce que tu as vu ou sais ; juré → dis ce que tu attends d'être établi.",

        (DiscussionMode::OxfordDebate, "en") => "Open for your camp: state the motion as you defend it and your strongest argument, addressed to the audience.",
        (DiscussionMode::OxfordDebate, "zh") => "为你的阵营开场：按你捍卫的方式陈述辩题和你最有力的论点，面向听众。",
        (DiscussionMode::OxfordDebate, _) => "Ouvre pour ton camp : énonce la motion telle que tu la défends et ton argument massue, adressé au public.",

        (DiscussionMode::Negotiation, "en") => "Table your opening offer: what you want, what you propose in exchange, and what is not negotiable for now.",
        (DiscussionMode::Negotiation, "zh") => "提出你的开局报价：你想要什么、你愿意用什么交换、目前哪些不可谈判。",
        (DiscussionMode::Negotiation, _) => "Pose ton offre d'ouverture : ce que tu veux, ce que tu proposes en échange, et ce qui n'est pas négociable pour l'instant.",

        (DiscussionMode::SixHats, "en") => "Put on this turn's hat and apply it to the question: nothing but that mode of thinking.",
        (DiscussionMode::SixHats, "zh") => "戴上本轮的帽子并将它用于这个问题：只用这一种思维方式。",
        (DiscussionMode::SixHats, _) => "Prends ton chapeau du tour et applique-le à la question : rien d'autre que ce mode de pensée.",

        (DiscussionMode::CrisisCell, "en") => "Give a first assessment from your expertise and propose the first urgent decision.",
        (DiscussionMode::CrisisCell, "zh") => "从你的专业角度给出首次评估，并提出第一个紧急决定。",
        (DiscussionMode::CrisisCell, _) => "Dresse un premier état des lieux depuis ton expertise et propose la première décision urgente.",
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

        (DiscussionMode::Trial, "en") => "React according to your role: exploit or dismantle the latest testimony, address the opposing side by name, ask the court to record a point.",
        (DiscussionMode::Trial, "zh") => "按你的角色回应：利用或拆解最新的证词，点名对方，请法庭记录一个要点。",
        (DiscussionMode::Trial, _) => "Réagis selon ton rôle : exploite ou démonte le dernier témoignage, interpelle la partie adverse par son nom, demande à la cour d'acter un point.",

        (DiscussionMode::OxfordDebate, "en") => "Rebut the latest opposing argument point by point, then push on for the audience with a fresh argument or a striking example.",
        (DiscussionMode::OxfordDebate, "zh") => "逐点反驳对方最新的论点，然后用一个新论点或鲜明的例子向听众推进。",
        (DiscussionMode::OxfordDebate, _) => "Réfute le dernier argument adverse point par point, puis relance pour le public avec un argument neuf ou un exemple frappant.",

        (DiscussionMode::Negotiation, "en") => "Answer the latest offer: accept, counter or condition it. Every concession must get something in return. Name the party you address.",
        (DiscussionMode::Negotiation, "zh") => "回应最新的报价：接受、还价或附加条件。每一次让步都必须换取回报。点名你所针对的一方。",
        (DiscussionMode::Negotiation, _) => "Réponds à la dernière offre : accepte, contre-propose ou conditionne. Chaque concession doit obtenir quelque chose en retour. Nomme la partie à qui tu t'adresses.",

        (DiscussionMode::SixHats, "en") => "Under this turn's hat, complete or correct what the other hats brought — without leaving your mode of thinking.",
        (DiscussionMode::SixHats, "zh") => "戴着本轮的帽子，补充或纠正其他帽子带来的内容——不要脱离你的思维方式。",
        (DiscussionMode::SixHats, _) => "Sous ton chapeau du tour, complète ou corrige ce que les autres chapeaux ont apporté — sans sortir de ton mode de pensée.",

        (DiscussionMode::CrisisCell, "en") => "React to the latest dispatch: what it changes, what you decide, what you ask of the other members — by name.",
        (DiscussionMode::CrisisCell, "zh") => "回应最新急电：它改变了什么、你决定什么、你要求其他成员做什么——点名说明。",
        (DiscussionMode::CrisisCell, _) => "Réagis à la dernière dépêche : ce qu'elle change, ce que tu décides, ce que tu demandes aux autres membres — nommément.",
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

        (DiscussionMode::Trial, "en") => "Stay in your role. Facts and evidence first.",
        (DiscussionMode::Trial, "zh") => "坚守你的角色。事实和证据优先。",
        (DiscussionMode::Trial, _) => "Reste dans ton rôle. Les faits et les preuves d'abord.",

        (DiscussionMode::OxfordDebate, "en") => "Never switch sides. Win the audience over.",
        (DiscussionMode::OxfordDebate, "zh") => "绝不换边。说服听众。",
        (DiscussionMode::OxfordDebate, _) => "Ne change jamais de camp. Convaincs le public.",

        (DiscussionMode::Negotiation, "en") => "Seek the agreement. Nothing without a counterpart.",
        (DiscussionMode::Negotiation, "zh") => "寻求协议。没有对价就不让步。",
        (DiscussionMode::Negotiation, _) => "Cherche l'accord. Rien sans contrepartie.",

        (DiscussionMode::SixHats, "en") => "One hat only: this turn's.",
        (DiscussionMode::SixHats, "zh") => "只戴一顶帽子：本轮的那顶。",
        (DiscussionMode::SixHats, _) => "Un seul chapeau : celui du tour.",

        (DiscussionMode::CrisisCell, "en") => "Decide and act. Every intervention carries a decision.",
        (DiscussionMode::CrisisCell, "zh") => "决定并行动。每次发言都要包含一个决定。",
        (DiscussionMode::CrisisCell, _) => "Décide et agis. Chaque intervention contient une décision.",
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
    focus: Option<&Focus>,
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

    // Rotating focus (turns ≥ 2): a line, or nothing when no focus was drawn
    let focus_line = focus_instruction(focus, lang).map(|f| format!("{f}\n")).unwrap_or_default();

    match (context, lang) {
        // ── Fiction ──────────────────────────────────────────────────────
        (InterventionContext::FictionOpening, "en") => format!(
            "=== YOUR TASK ===\n\
             Nobody has written yet: write the OPENING of the story.\n\
             Set the scene, introduce a protagonist and plant an inciting event that the next co-authors can build on.\n\
             {constraint}\n\
             Keep it to one or two paragraphs.\n\
             {end_awareness}\n\
             REMEMBER: {constraint}"
        ),
        (InterventionContext::FictionOpening, "zh") => format!(
            "=== 你的任务 ===\n\
             还没有人动笔：写下故事的开头。\n\
             设定场景，引入一位主角，埋下一个引发事件，让接下来的共同作者可以延续。\n\
             {constraint}\n\
             保持一到两段。\n\
             {end_awareness}\n\
             记住：{constraint}"
        ),
        (InterventionContext::FictionOpening, _) => format!(
            "=== VOTRE TÂCHE ===\n\
             Personne n'a encore écrit : écris l'OUVERTURE de l'histoire.\n\
             Pose le décor, introduis un protagoniste et plante un événement déclencheur sur lequel les co-auteurs suivants pourront construire.\n\
             {constraint}\n\
             Tiens-toi à un ou deux paragraphes.\n\
             {end_awareness}\n\
             RAPPEL : {constraint}"
        ),
        (InterventionContext::FictionContinue, "en") => format!(
            "=== YOUR TASK ===\n\
             {engage}\n\
             {constraint}\n\
             Keep it to one or two paragraphs.\n\
             {end_awareness}\n\
             REMEMBER: {constraint}"
        ),
        (InterventionContext::FictionContinue, "zh") => format!(
            "=== 你的任务 ===\n\
             {engage}\n\
             {constraint}\n\
             保持一到两段。\n\
             {end_awareness}\n\
             记住：{constraint}"
        ),
        (InterventionContext::FictionContinue, _) => format!(
            "=== VOTRE TÂCHE ===\n\
             {engage}\n\
             {constraint}\n\
             Tiens-toi à un ou deux paragraphes.\n\
             {end_awareness}\n\
             RAPPEL : {constraint}"
        ),

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
                format!("{} spoke from the audience — answer them first, by name, then carry on with the other participants.", user_name)
            };
            format!(
                "=== YOUR TASK ===\n\
                 {user_line}\n\
                 {focus_line}\
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
                format!("现场观众{}发言了——先点名回应他，再继续与其他参与者交流。", user_name)
            };
            format!(
                "=== 你的任务 ===\n\
                 {user_line}\n\
                 {focus_line}\
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
                format!("{} a pris la parole depuis le public — réponds-lui d'abord, en le nommant, puis poursuis avec les autres participants.", user_name)
            };
            format!(
                "=== VOTRE TÂCHE ===\n\
                 {user_line}\n\
                 {focus_line}\
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
             {focus_line}\
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
             {focus_line}\
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
             {focus_line}\
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
             {focus_line}\
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
             {focus_line}\
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
             {focus_line}\
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

        (DiscussionMode::Trial, "en") => ("point convincingly established", "out of role or weak evidence"),
        (DiscussionMode::Trial, "zh") => ("令人信服地确立了要点", "脱离角色或证据薄弱"),
        (DiscussionMode::Trial, _) => ("point établi de façon convaincante", "hors rôle ou preuve faible"),

        (DiscussionMode::OxfordDebate, "en") => ("argument that sways the audience", "weak or unfair argument"),
        (DiscussionMode::OxfordDebate, "zh") => ("能改变听众的论点", "薄弱或不公允的论点"),
        (DiscussionMode::OxfordDebate, _) => ("argument qui fait basculer le public", "argument faible ou déloyal"),

        (DiscussionMode::Negotiation, "en") => ("constructive offer that brings the agreement closer", "stonewalling or bad faith"),
        (DiscussionMode::Negotiation, "zh") => ("有助于达成协议的建设性报价", "阻挠或缺乏诚意"),
        (DiscussionMode::Negotiation, _) => ("offre constructive qui rapproche de l'accord", "blocage ou mauvaise foi"),

        (DiscussionMode::SixHats, "en") => ("true to their hat and enlightening", "off their hat or shallow"),
        (DiscussionMode::SixHats, "zh") => ("忠于帽子且富有启发", "脱离帽子或肤浅"),
        (DiscussionMode::SixHats, _) => ("fidèle à son chapeau et éclairant", "hors chapeau ou superficiel"),

        (DiscussionMode::CrisisCell, "en") => ("clear and relevant decision", "analysis without decision or off the dispatch"),
        (DiscussionMode::CrisisCell, "zh") => ("清晰而切题的决定", "只分析不决定或偏离急电"),
        (DiscussionMode::CrisisCell, _) => ("décision claire et pertinente", "analyse sans décision ou hors dépêche"),
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

        (DiscussionMode::Trial, "en") => "=== DISCUSSION FORMAT: TRIAL ===\nThis session is an adversarial trial. Everyone holds a role — prosecution, defence, witness, juror — and never leaves it. Facts and evidence outweigh eloquence; the jurors return the verdict at the end.",
        (DiscussionMode::Trial, "zh") => "=== 讨论格式：审判 ===\n本次会议是一场对抗式审判。每个人都扮演一个角色——控方、辩方、证人、陪审员——并始终坚守。事实和证据重于雄辩；陪审员在最后作出裁决。",
        (DiscussionMode::Trial, _) => "=== FORMAT DE DISCUSSION : PROCÈS ===\nCette session est un procès contradictoire. Chacun tient un rôle — accusation, défense, témoin, juré — et ne le quitte pas. Les faits et les preuves priment sur l'éloquence ; le verdict sera rendu à la fin par les jurés.",

        (DiscussionMode::OxfordDebate, "en") => "=== DISCUSSION FORMAT: OXFORD DEBATE ===\nThis session is a formal debate on a motion, in two fixed camps — For and Against. The audience votes before and after: the goal is to move votes, not to be right among yourselves. Switching sides is off-track.",
        (DiscussionMode::OxfordDebate, "zh") => "=== 讨论格式：牛津式辩论 ===\n本次会议是围绕一个辩题的正式辩论，分为固定的两个阵营——正方和反方。听众在辩论前后投票：目标是改变票数，而不是在你们之间争对错。换边不符合此格式。",
        (DiscussionMode::OxfordDebate, _) => "=== FORMAT DE DISCUSSION : DÉBAT D'OXFORD ===\nCette session est un débat formel sur une motion, en deux camps fixes — Pour et Contre. Le public vote avant et après : l'objectif est de déplacer des voix, pas d'avoir raison entre soi. Changer de camp est hors-cadre.",

        (DiscussionMode::Negotiation, "en") => "=== DISCUSSION FORMAT: NEGOTIATION ===\nThis session is a negotiation between parties with distinct interests. The goal is an agreement acceptable to all; each party defends its interests, makes offers and obtains counterparts. Debating to be right is off-track.",
        (DiscussionMode::Negotiation, "zh") => "=== 讨论格式：谈判 ===\n本次会议是利益各异的各方之间的谈判。目标是各方都能接受的协议；各方捍卫自身利益、提出报价并获得对价。为争对错而辩论不符合此格式。",
        (DiscussionMode::Negotiation, _) => "=== FORMAT DE DISCUSSION : NÉGOCIATION ===\nCette session est une négociation entre parties aux intérêts distincts. L'objectif est un accord acceptable par tous ; chaque partie défend ses intérêts, fait des offres et obtient des contreparties. Débattre pour avoir raison est hors-cadre.",

        (DiscussionMode::SixHats, "en") => "=== DISCUSSION FORMAT: SIX THINKING HATS ===\nThis session applies the six-hats method: every turn, each participant thinks only according to the hat they are given (white: facts; red: feelings; black: caution; yellow: benefits; green: creativity; blue: process). Thinking outside one's hat is off-track.",
        (DiscussionMode::SixHats, "zh") => "=== 讨论格式：六顶思考帽 ===\n本次会议采用六顶思考帽方法：每一轮，每位参与者只按分配到的帽子思考（白：事实；红：情感；黑：谨慎；黄：益处；绿：创意；蓝：流程）。脱离帽子思考不符合此格式。",
        (DiscussionMode::SixHats, _) => "=== FORMAT DE DISCUSSION : SIX CHAPEAUX ===\nCette session applique la méthode des six chapeaux : à chaque tour, chaque participant pense uniquement selon le chapeau qui lui est attribué (blanc : faits ; rouge : émotions ; noir : prudence ; jaune : bénéfices ; vert : créativité ; bleu : processus). Penser hors de son chapeau est hors-cadre.",

        (DiscussionMode::CrisisCell, "en") => "=== DISCUSSION FORMAT: CRISIS CELL ===\nThis session is a real-time crisis cell: dispatches come in every turn and demand decisions. Every intervention carries a decision or a concrete action. Analysing without deciding is off-track.",
        (DiscussionMode::CrisisCell, "zh") => "=== 讨论格式：危机小组 ===\n本次会议是一个实时危机小组：每一轮都有急电传来并要求作出决定。每次发言都要包含一个决定或具体行动。只分析不决定不符合此格式。",
        (DiscussionMode::CrisisCell, _) => "=== FORMAT DE DISCUSSION : CELLULE DE CRISE ===\nCette session est une cellule de crise en temps réel : des dépêches tombent à chaque tour et exigent des décisions. Chaque intervention contient une décision ou une action concrète. Analyser sans décider est hors-cadre.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_modes() -> Vec<DiscussionMode> {
        DiscussionMode::ALL.to_vec()
    }

    fn non_debate_modes() -> Vec<DiscussionMode> {
        all_modes().into_iter().filter(|m| *m != DiscussionMode::Debate).collect()
    }

    #[test]
    fn every_mode_has_its_full_set_of_texts_in_three_languages() {
        for mode in all_modes() {
            for lang in all_langs() {
                for (name, text) in [
                    ("descriptor", mode_descriptor(&mode, lang)),
                    ("introduction", mode_introduction_instructions(&mode, lang)),
                    ("preamble", mode_intervention_preamble(&mode, lang)),
                    ("thought (first)", mode_thought_focus(&mode, lang, false)),
                    ("thought", mode_thought_focus(&mode, lang, true)),
                    ("synthesis", mode_synthesis_instructions(&mode, lang)),
                    ("moderation", mode_moderation_criteria(&mode, lang)),
                ] {
                    assert!(!text.is_empty(), "{mode:?}/{lang}: empty {name}");
                }
            }
        }
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
            InterventionContext::FictionOpening,
            InterventionContext::FictionContinue,
        ];
        for mode in all_modes() {
            for lang in all_langs() {
                for context in &contexts {
                    let result = mode_context_instruction(
                        &mode, lang, *context, "TestUser", "", None,
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
                &mode, "en", InterventionContext::General, "TestUser", "", None,
            );
            assert!(result.contains("REMEMBER:"), "Missing REMEMBER for {mode:?}/en: {result}");

            let result_fr = mode_context_instruction(
                &mode, "fr", InterventionContext::General, "TestUser", "", None,
            );
            assert!(result_fr.contains("RAPPEL"), "Missing RAPPEL for {mode:?}/fr");

            let result_zh = mode_context_instruction(
                &mode, "zh", InterventionContext::General, "TestUser", "", None,
            );
            assert!(result_zh.contains("记住"), "Missing 记住 for {mode:?}/zh");
        }
    }

    #[test]
    fn test_focus_line_in_engagement_contexts_only() {
        let focus = Focus::Speaker("Le Sceptique".to_string());
        for ctx in [InterventionContext::UserSpoke, InterventionContext::FirstOfTurn, InterventionContext::General] {
            let text = mode_context_instruction(&DiscussionMode::Debate, "fr", ctx, "Léo", "", Some(&focus));
            assert!(text.contains("Adresse-toi en priorité à Le Sceptique"), "{text}");
        }
        for ctx in [InterventionContext::Opening, InterventionContext::Turn1, InterventionContext::FictionContinue] {
            let text = mode_context_instruction(&DiscussionMode::Debate, "fr", ctx, "Léo", "", Some(&focus));
            assert!(!text.contains("Le Sceptique"), "{text}");
        }
        let topic = mode_context_instruction(&DiscussionMode::Debate, "en", InterventionContext::General, "Léo", "", Some(&Focus::Topic));
        assert!(topic.contains("do not answer anyone in particular"));
        let none = mode_context_instruction(&DiscussionMode::Debate, "en", InterventionContext::General, "Léo", "", None);
        assert!(!none.contains("in priority"));
    }

    #[test]
    fn test_socratic_prompt_lists_previous_questions() {
        let prev: Vec<String> = (1..=5).map(|i| format!("Question {i} ?")).collect();
        let p = build_socratic_question_prompt("Sujet", "A: bla", "fr", &prev);
        assert!(p.contains("DÉJÀ posées"));
        // Only the most recent N are injected
        assert!(p.contains("Question 5 ?") && p.contains("Question 3 ?"));
        assert!(!p.contains("Question 2 ?"));
        let p0 = build_socratic_question_prompt("Sujet", "A: bla", "fr", &[]);
        assert!(!p0.contains("DÉJÀ posées"));
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
        for mode in non_debate_modes() {
            let result = mode_override_clause(&mode, "en");
            assert!(
                result.contains("=== DISCUSSION FORMAT"),
                "Missing format delimiter for {mode:?}: {result}"
            );
        }
    }

    #[test]
    fn test_mode_moderation_criteria_redirect() {
        for mode in &non_debate_modes() {
            let en = mode_moderation_criteria(mode, "en");
            assert!(en.contains("redirect"), "Missing 'redirect' in EN moderation for {mode:?}");

            let fr = mode_moderation_criteria(mode, "fr");
            assert!(fr.contains("recadre"), "Missing 'recadre' in FR moderation for {mode:?}");

            let zh = mode_moderation_criteria(mode, "zh");
            assert!(zh.contains("引导"), "Missing '引导' in ZH moderation for {mode:?}");
        }
    }
}
