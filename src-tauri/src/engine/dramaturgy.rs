//! Acts (v1.18): a script per discussion mode gives the turns a shape —
//! opening, confrontation, concessions, closing statements… Each act carries a
//! templated announcement of the moderator (no LLM), an instruction for the
//! speakers and a hint for the moderation. `resolve_act` maps a turn to its act
//! from the script's shares, the stagnation signal and the stop request.

use serde::{Deserialize, Serialize};

use crate::models::discussion::DiscussionMode;

/// Every act of every script (one enum so events and reports stay typed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActKey {
    // Debate
    Opening,
    Confrontation,
    CrossExamination,
    Concessions,
    ClosingStatements,
    // Ideation
    Divergence,
    Association,
    Convergence,
    // Co-construction
    Proposal,
    Critique,
    Consolidation,
    // Socratic
    Questioning,
    Digging,
    Synthesis,
    // Tutorial
    Foundations,
    Deepening,
    Recap,
    // Critique review
    Impressions,
    Examination,
    Recommendations,
    // Fiction
    Exposition,
    Complication,
    Climax,
    Resolution,
    // Negotiation (v1.19)
    Bargaining,
    Agreement,
    // Six hats (v1.19)
    Framing,
    Exploration,
    // Crisis cell (v1.19)
    Alert,
    Response,
    Debrief,
}

/// One act of a script with its relative duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActSpec {
    pub key: ActKey,
    /// Relative share of the turns (the first act always starts on turn 1, the last one ends the discussion)
    pub share: u32,
}

const fn act(key: ActKey, share: u32) -> ActSpec {
    ActSpec { key, share }
}

const DEBATE: &[ActSpec] = &[
    act(ActKey::Opening, 1),
    act(ActKey::Confrontation, 3),
    act(ActKey::CrossExamination, 2),
    act(ActKey::Concessions, 2),
    act(ActKey::ClosingStatements, 1),
];
const IDEATION: &[ActSpec] = &[act(ActKey::Divergence, 2), act(ActKey::Association, 2), act(ActKey::Convergence, 1)];
const CO_CONSTRUCTION: &[ActSpec] = &[act(ActKey::Proposal, 2), act(ActKey::Critique, 2), act(ActKey::Consolidation, 1)];
const SOCRATIC: &[ActSpec] = &[act(ActKey::Questioning, 1), act(ActKey::Digging, 3), act(ActKey::Synthesis, 1)];
const TUTORIAL: &[ActSpec] = &[act(ActKey::Foundations, 2), act(ActKey::Deepening, 2), act(ActKey::Recap, 1)];
const CRITIQUE: &[ActSpec] = &[act(ActKey::Impressions, 1), act(ActKey::Examination, 3), act(ActKey::Recommendations, 1)];
const FICTION: &[ActSpec] = &[act(ActKey::Exposition, 1), act(ActKey::Complication, 2), act(ActKey::Climax, 1), act(ActKey::Resolution, 1)];
const TRIAL: &[ActSpec] = &[act(ActKey::Opening, 1), act(ActKey::Confrontation, 2), act(ActKey::CrossExamination, 2), act(ActKey::ClosingStatements, 1)];
const OXFORD: &[ActSpec] = &[act(ActKey::Opening, 1), act(ActKey::Confrontation, 2), act(ActKey::CrossExamination, 1), act(ActKey::ClosingStatements, 1)];
const NEGOTIATION: &[ActSpec] = &[act(ActKey::Opening, 1), act(ActKey::Bargaining, 3), act(ActKey::Concessions, 1), act(ActKey::Agreement, 1)];
const SIX_HATS: &[ActSpec] = &[act(ActKey::Framing, 1), act(ActKey::Exploration, 3), act(ActKey::Convergence, 1)];
const CRISIS: &[ActSpec] = &[act(ActKey::Alert, 1), act(ActKey::Response, 3), act(ActKey::Debrief, 1)];

/// The script of a mode (`UserDriven` has none: the user sets the rhythm).
pub fn mode_script(mode: &DiscussionMode) -> &'static [ActSpec] {
    match mode {
        DiscussionMode::Debate => DEBATE,
        DiscussionMode::Ideation => IDEATION,
        DiscussionMode::CoConstruction => CO_CONSTRUCTION,
        DiscussionMode::Socratic => SOCRATIC,
        DiscussionMode::Tutorial => TUTORIAL,
        DiscussionMode::CritiqueReview => CRITIQUE,
        DiscussionMode::CollaborativeFiction => FICTION,
        DiscussionMode::Trial => TRIAL,
        DiscussionMode::OxfordDebate => OXFORD,
        DiscussionMode::Negotiation => NEGOTIATION,
        DiscussionMode::SixHats => SIX_HATS,
        DiscussionMode::CrisisCell => CRISIS,
        DiscussionMode::UserDriven => &[],
    }
}

/// What the resolver needs to know about the turn.
#[derive(Debug, Clone, Copy)]
pub struct TurnPosition {
    pub turn: u32,
    pub max_turns: Option<u32>,
    /// The discussion is stagnating (summary, reactions or analyst signal)
    pub stagnating: bool,
    /// A soft stop was requested: this turn is the last one
    pub stop_requested: bool,
}

/// Act of a turn.
///
/// With a turn limit the shares are proportional (the first act always owns
/// turn 1, the last act always owns the last turn). Without a limit the script
/// slides: first act on turn 1, second act afterwards, the penultimate act once
/// the discussion stagnates. A soft stop makes the current turn the last act;
/// stagnation during the middle acts jumps to the penultimate one (concessions).
pub fn resolve_act(mode: &DiscussionMode, pos: TurnPosition) -> Option<ActKey> {
    let script = mode_script(mode);
    let (first, last) = (script.first()?, script.last()?);
    if script.len() == 1 || pos.turn <= 1 && !pos.stop_requested && pos.max_turns != Some(1) {
        return Some(first.key);
    }
    let is_last_turn = pos.stop_requested || pos.max_turns.is_some_and(|m| pos.turn >= m);
    if is_last_turn {
        return Some(last.key);
    }
    let penultimate = script[script.len().saturating_sub(2)];
    let scheduled = match pos.max_turns {
        Some(max) => scheduled_act(script, pos.turn, max),
        // Sliding script: the second act until the discussion stagnates
        None => script.get(1).copied().unwrap_or(*first),
    };
    // Stagnation pulls the middle acts forward to the penultimate one
    let middle = script.len() > 3 && index_of(script, scheduled.key) < script.len() - 2 && index_of(script, scheduled.key) >= 1;
    if pos.stagnating && (middle || pos.max_turns.is_none() && script.len() >= 3) {
        return Some(penultimate.key);
    }
    Some(scheduled.key)
}

fn index_of(script: &[ActSpec], key: ActKey) -> usize {
    script.iter().position(|a| a.key == key).unwrap_or(0)
}

/// Proportional schedule: turn 1 → first act, turn `max` → last act, the turns
/// in between split by share among the middle acts (every middle act gets at
/// least one turn when there are enough turns).
fn scheduled_act(script: &[ActSpec], turn: u32, max: u32) -> ActSpec {
    let middle: Vec<ActSpec> = script[1..script.len() - 1].to_vec();
    let inner_turns = max.saturating_sub(2); // turns 2..=max-1
    if middle.is_empty() || inner_turns == 0 {
        return if turn <= 1 { script[0] } else { script[script.len() - 1] };
    }
    let total_share: u32 = middle.iter().map(|a| a.share).sum::<u32>().max(1);
    let n = middle.len() as u32;
    // Turns per middle act: one each when there are enough, the rest by share
    // (largest remainder); otherwise plain proportions (some acts get none).
    let counts: Vec<u32> = if inner_turns >= n {
        let extra = inner_turns - n;
        let mut counts: Vec<u32> = middle.iter().map(|a| 1 + extra * a.share / total_share).collect();
        let mut leftover = inner_turns - counts.iter().sum::<u32>();
        let mut by_remainder: Vec<usize> = (0..middle.len()).collect();
        by_remainder.sort_by_key(|&i| std::cmp::Reverse((extra * middle[i].share) % total_share));
        for i in by_remainder {
            if leftover == 0 {
                break;
            }
            counts[i] += 1;
            leftover -= 1;
        }
        counts
    } else {
        let mut counts: Vec<u32> = middle.iter().map(|a| inner_turns * a.share / total_share).collect();
        let mut leftover = inner_turns - counts.iter().sum::<u32>();
        for c in counts.iter_mut() {
            if leftover == 0 {
                break;
            }
            *c += 1;
            leftover -= 1;
        }
        counts
    };
    let offset = turn.saturating_sub(2); // 0-based position among the inner turns
    let mut boundary = 0u32;
    for (a, count) in middle.iter().zip(counts) {
        boundary += count;
        if offset < boundary {
            return *a;
        }
    }
    middle[middle.len() - 1]
}

impl ActKey {
    /// Title shown in the UI and the timeline.
    pub fn title(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (Self::Opening, "en") => "Opening",
            (Self::Opening, "zh") => "开场",
            (Self::Opening, _) => "Ouverture",
            (Self::Confrontation, "en") => "Confrontation",
            (Self::Confrontation, "zh") => "交锋",
            (Self::Confrontation, _) => "Confrontation",
            (Self::CrossExamination, "en") => "Cross-examination",
            (Self::CrossExamination, "zh") => "交叉质询",
            (Self::CrossExamination, _) => "Contre-interrogatoire",
            (Self::Concessions, "en") => "Concessions",
            (Self::Concessions, "zh") => "让步",
            (Self::Concessions, _) => "Concessions",
            (Self::ClosingStatements, "en") => "Closing statements",
            (Self::ClosingStatements, "zh") => "结辩",
            (Self::ClosingStatements, _) => "Plaidoiries",
            (Self::Divergence, "en") => "Divergence",
            (Self::Divergence, "zh") => "发散",
            (Self::Divergence, _) => "Divergence",
            (Self::Association, "en") => "Association",
            (Self::Association, "zh") => "联想",
            (Self::Association, _) => "Association",
            (Self::Convergence, "en") => "Convergence",
            (Self::Convergence, "zh") => "收敛",
            (Self::Convergence, _) => "Convergence",
            (Self::Proposal, "en") => "Proposals",
            (Self::Proposal, "zh") => "提案",
            (Self::Proposal, _) => "Propositions",
            (Self::Critique, "en") => "Critique",
            (Self::Critique, "zh") => "批评",
            (Self::Critique, _) => "Critique",
            (Self::Consolidation, "en") => "Consolidation",
            (Self::Consolidation, "zh") => "整合",
            (Self::Consolidation, _) => "Consolidation",
            (Self::Questioning, "en") => "Questioning",
            (Self::Questioning, "zh") => "提问",
            (Self::Questioning, _) => "Questionnement",
            (Self::Digging, "en") => "Digging deeper",
            (Self::Digging, "zh") => "深挖",
            (Self::Digging, _) => "Creusement",
            (Self::Synthesis, "en") => "Synthesis",
            (Self::Synthesis, "zh") => "综合",
            (Self::Synthesis, _) => "Synthèse",
            (Self::Foundations, "en") => "Foundations",
            (Self::Foundations, "zh") => "基础",
            (Self::Foundations, _) => "Fondations",
            (Self::Deepening, "en") => "Deepening",
            (Self::Deepening, "zh") => "深入",
            (Self::Deepening, _) => "Approfondissement",
            (Self::Recap, "en") => "Recap",
            (Self::Recap, "zh") => "回顾",
            (Self::Recap, _) => "Récapitulation",
            (Self::Impressions, "en") => "First impressions",
            (Self::Impressions, "zh") => "初步印象",
            (Self::Impressions, _) => "Impressions",
            (Self::Examination, "en") => "Examination",
            (Self::Examination, "zh") => "审视",
            (Self::Examination, _) => "Examen",
            (Self::Recommendations, "en") => "Recommendations",
            (Self::Recommendations, "zh") => "建议",
            (Self::Recommendations, _) => "Recommandations",
            (Self::Exposition, "en") => "Exposition",
            (Self::Exposition, "zh") => "铺陈",
            (Self::Exposition, _) => "Exposition",
            (Self::Complication, "en") => "Complication",
            (Self::Complication, "zh") => "纠葛",
            (Self::Complication, _) => "Complication",
            (Self::Climax, "en") => "Climax",
            (Self::Climax, "zh") => "高潮",
            (Self::Climax, _) => "Climax",
            (Self::Resolution, "en") => "Resolution",
            (Self::Resolution, "zh") => "结局",
            (Self::Resolution, _) => "Résolution",
            (Self::Bargaining, "en") => "Bargaining",
            (Self::Bargaining, "zh") => "讨价还价",
            (Self::Bargaining, _) => "Marchandage",
            (Self::Agreement, "en") => "Agreement",
            (Self::Agreement, "zh") => "达成协议",
            (Self::Agreement, _) => "Accord",
            (Self::Framing, "en") => "Framing",
            (Self::Framing, "zh") => "设定框架",
            (Self::Framing, _) => "Cadrage",
            (Self::Exploration, "en") => "Exploration",
            (Self::Exploration, "zh") => "探索",
            (Self::Exploration, _) => "Exploration",
            (Self::Alert, "en") => "Alert",
            (Self::Alert, "zh") => "警报",
            (Self::Alert, _) => "Alerte",
            (Self::Response, "en") => "Response",
            (Self::Response, "zh") => "应对",
            (Self::Response, _) => "Riposte",
            (Self::Debrief, "en") => "Debrief",
            (Self::Debrief, "zh") => "复盘",
            (Self::Debrief, _) => "Débriefing",
        }
    }

    /// The moderator's announcement when the act begins (templated, no LLM).
    pub fn announcement(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (Self::Opening, "en") => "Opening: each of you states their position and what is at stake for them.",
            (Self::Opening, "zh") => "开场：请各位陈述自己的立场以及对你而言的关键所在。",
            (Self::Opening, _) => "Ouverture : chacun pose sa position et ce qui, pour lui, est en jeu.",
            (Self::Confrontation, "en") => "Confrontation: the positions are on the table — attack the arguments, not the people.",
            (Self::Confrontation, "zh") => "交锋：立场已经摆明——攻击论点，而非个人。",
            (Self::Confrontation, _) => "Confrontation : les positions sont posées — attaquez les arguments, pas les personnes.",
            (Self::CrossExamination, "en") => "Cross-examination: put precise questions to one another and answer the ones you were asked.",
            (Self::CrossExamination, "zh") => "交叉质询：相互提出明确的问题，并回答别人向你提出的问题。",
            (Self::CrossExamination, _) => "Contre-interrogatoire : posez-vous des questions précises et répondez à celles qu'on vous a posées.",
            (Self::Concessions, "en") => "Concessions: what have you heard that made you think? Grant what deserves it.",
            (Self::Concessions, "zh") => "让步：你听到了什么值得深思的？该承认的就承认。",
            (Self::Concessions, _) => "Concessions : qu'avez-vous entendu qui vous ait fait réfléchir ? Accordez ce qui le mérite.",
            (Self::ClosingStatements, "en") => "Closing statements: one last word each — your strongest point and where you now stand.",
            (Self::ClosingStatements, "zh") => "结辩：每人最后一言——你最有力的论点以及你现在的立场。",
            (Self::ClosingStatements, _) => "Plaidoiries : un dernier mot chacun — votre point le plus fort et où vous en êtes désormais.",
            (Self::Divergence, "en") => "Divergence: quantity over quality — throw in ideas without judging them.",
            (Self::Divergence, "zh") => "发散：数量优先——抛出想法，先不评判。",
            (Self::Divergence, _) => "Divergence : la quantité avant la qualité — lancez des idées sans les juger.",
            (Self::Association, "en") => "Association: build on each other's ideas — combine, twist, extend.",
            (Self::Association, "zh") => "联想：在彼此的想法上继续构建——组合、扭转、延伸。",
            (Self::Association, _) => "Association : construisez sur les idées des autres — combinez, détournez, prolongez.",
            (Self::Convergence, "en") => "Convergence: pick the ideas worth keeping and say why.",
            (Self::Convergence, "zh") => "收敛：挑出值得保留的想法并说明理由。",
            (Self::Convergence, _) => "Convergence : retenez les idées qui valent le coup et dites pourquoi.",
            (Self::Proposal, "en") => "Proposals: bring concrete material to the shared work.",
            (Self::Proposal, "zh") => "提案：为共同的成果带来具体的内容。",
            (Self::Proposal, _) => "Propositions : apportez de la matière concrète au travail commun.",
            (Self::Critique, "en") => "Critique: examine what has been proposed — gaps, contradictions, improvements.",
            (Self::Critique, "zh") => "批评：审视已提出的内容——缺口、矛盾、改进之处。",
            (Self::Critique, _) => "Critique : examinez ce qui a été proposé — manques, contradictions, améliorations.",
            (Self::Consolidation, "en") => "Consolidation: settle what remains open and make the result stand on its own.",
            (Self::Consolidation, "zh") => "整合：解决尚未决定的问题，让成果自成一体。",
            (Self::Consolidation, _) => "Consolidation : tranchez ce qui reste ouvert et rendez le résultat autoportant.",
            (Self::Questioning, "en") => "Questioning: what do we think we know about this? Name your assumptions.",
            (Self::Questioning, "zh") => "提问：关于这个话题我们自以为知道什么？说出你的假设。",
            (Self::Questioning, _) => "Questionnement : que croyons-nous savoir de cela ? Nommez vos présupposés.",
            (Self::Digging, "en") => "Digging deeper: follow each answer with a harder question.",
            (Self::Digging, "zh") => "深挖：每个回答之后都提出一个更难的问题。",
            (Self::Digging, _) => "Creusement : à chaque réponse, une question plus difficile.",
            (Self::Synthesis, "en") => "Synthesis: what has our questioning revealed? What do we now hold as better founded?",
            (Self::Synthesis, "zh") => "综合：我们的追问揭示了什么？现在我们认为哪些更有根据？",
            (Self::Synthesis, _) => "Synthèse : qu'a révélé notre questionnement ? Que tenons-nous désormais pour mieux fondé ?",
            (Self::Foundations, "en") => "Foundations: lay down the essential notions clearly, without jargon.",
            (Self::Foundations, "zh") => "基础：清晰地讲清基本概念，不用行话。",
            (Self::Foundations, _) => "Fondations : posez les notions essentielles, clairement, sans jargon.",
            (Self::Deepening, "en") => "Deepening: examples, edge cases, common mistakes.",
            (Self::Deepening, "zh") => "深入：例子、边界情况、常见错误。",
            (Self::Deepening, _) => "Approfondissement : exemples, cas limites, erreurs fréquentes.",
            (Self::Recap, "en") => "Recap: what should be remembered, in the right order.",
            (Self::Recap, "zh") => "回顾：按正确的顺序梳理应当记住的要点。",
            (Self::Recap, _) => "Récapitulation : ce qu'il faut retenir, dans le bon ordre.",
            (Self::Impressions, "en") => "First impressions: your overall reading before the details.",
            (Self::Impressions, "zh") => "初步印象：在细节之前，先谈整体感受。",
            (Self::Impressions, _) => "Impressions : votre lecture d'ensemble avant le détail.",
            (Self::Examination, "en") => "Examination: point by point, with evidence.",
            (Self::Examination, "zh") => "审视：逐点分析，拿出证据。",
            (Self::Examination, _) => "Examen : point par point, preuves à l'appui.",
            (Self::Recommendations, "en") => "Recommendations: what to change, what to keep, in order of priority.",
            (Self::Recommendations, "zh") => "建议：该改什么、该留什么，按优先级排列。",
            (Self::Recommendations, _) => "Recommandations : quoi changer, quoi garder, par ordre de priorité.",
            (Self::Exposition, "en") => "Exposition: set the world, the characters and the first tension.",
            (Self::Exposition, "zh") => "铺陈：建立世界、人物和第一重张力。",
            (Self::Exposition, _) => "Exposition : installez le monde, les personnages et la première tension.",
            (Self::Complication, "en") => "Complication: obstacles pile up, choices get costly.",
            (Self::Complication, "zh") => "纠葛：障碍层层叠加，选择代价渐高。",
            (Self::Complication, _) => "Complication : les obstacles s'accumulent, les choix coûtent.",
            (Self::Climax, "en") => "Climax: the decisive confrontation — nothing can be undone after this.",
            (Self::Climax, "zh") => "高潮：决定性的对峙——此后一切都无法挽回。",
            (Self::Climax, _) => "Climax : l'affrontement décisif — après cela, plus rien ne peut être défait.",
            (Self::Resolution, "en") => "Resolution: the consequences, and what remains of the characters.",
            (Self::Resolution, "zh") => "结局：后果，以及人物身上留下的东西。",
            (Self::Resolution, _) => "Résolution : les conséquences, et ce qu'il reste des personnages.",
            (Self::Bargaining, "en") => "Bargaining: offers and counter-offers — every concession has a price.",
            (Self::Bargaining, "zh") => "讨价还价：报价与还价——每一次让步都有代价。",
            (Self::Bargaining, _) => "Marchandage : offres et contre-offres — chaque concession se paie.",
            (Self::Agreement, "en") => "Agreement: state what you can sign — and what you still lack.",
            (Self::Agreement, "zh") => "达成协议：说出你能签署的内容——以及你还缺什么。",
            (Self::Agreement, _) => "Accord : formulez ce que vous pouvez signer — et ce qui vous manque encore.",
            (Self::Framing, "en") => "Framing: let us set the question, what we know and what we seek — each under their hat.",
            (Self::Framing, "zh") => "设定框架：明确问题、已知的事实和我们要寻找的东西——各自戴好自己的帽子。",
            (Self::Framing, _) => "Cadrage : posons la question, ce qu'on sait et ce qu'on cherche — chacun sous son chapeau.",
            (Self::Exploration, "en") => "Exploration: the hats rotate — explore the question from every angle, turn after turn.",
            (Self::Exploration, "zh") => "探索：帽子轮换——一轮又一轮，从每个角度探索问题。",
            (Self::Exploration, _) => "Exploration : les chapeaux tournent — explorez la question sous chaque angle, tour après tour.",
            (Self::Alert, "en") => "Alert: the situation is set — assessment and first decisions.",
            (Self::Alert, "zh") => "警报：局势已明——评估现状并作出首批决定。",
            (Self::Alert, _) => "Alerte : la situation est posée — état des lieux et premières décisions.",
            (Self::Response, "en") => "Response: the dispatches are coming in — decide, coordinate, own it.",
            (Self::Response, "zh") => "应对：急电不断传来——决定、协调、承担责任。",
            (Self::Response, _) => "Riposte : les dépêches tombent — décidez, coordonnez, assumez.",
            (Self::Debrief, "en") => "Debrief: the crisis is stabilising — what worked, what was missing, what we keep.",
            (Self::Debrief, "zh") => "复盘：危机趋于稳定——哪些奏效、哪些缺失、哪些经验要记住。",
            (Self::Debrief, _) => "Débriefing : la crise se stabilise — ce qui a marché, ce qui a manqué, ce qu'on retient.",
        }
    }

    /// Instruction added to the speakers' prompts while the act lasts.
    pub fn speaker_instruction(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (Self::Opening, "en") => "Act — opening: state your position and what matters to you; no rebuttal yet.",
            (Self::Opening, "zh") => "当前幕——开场：陈述你的立场和你在意的东西；先不反驳。",
            (Self::Opening, _) => "Acte — ouverture : pose ta position et ce qui compte pour toi ; pas encore de réfutation.",
            (Self::Confrontation, "en") => "Act — confrontation: take on the strongest opposing argument head-on, with evidence.",
            (Self::Confrontation, "zh") => "当前幕——交锋：直面最有力的反方论点，拿出证据。",
            (Self::Confrontation, _) => "Acte — confrontation : affronte de front l'argument adverse le plus fort, preuves à l'appui.",
            (Self::CrossExamination, "en") => "Act — cross-examination: ask one precise question to a named participant and answer the ones you were asked.",
            (Self::CrossExamination, "zh") => "当前幕——交叉质询：向某位点名的参与者提出一个明确的问题，并回答向你提出的问题。",
            (Self::CrossExamination, _) => "Acte — contre-interrogatoire : pose une question précise à un participant nommé et réponds à celles qu'on t'a posées.",
            (Self::Concessions, "en") => "Act — concessions: grant one point that deserves it and say what it changes for you.",
            (Self::Concessions, "zh") => "当前幕——让步：承认一个值得承认的观点，并说明它对你意味着什么。",
            (Self::Concessions, _) => "Acte — concessions : accorde un point qui le mérite et dis ce que cela change pour toi.",
            (Self::ClosingStatements, "en") => "Act — closing statement: this is your last word — your strongest point and where you stand now; no new topic.",
            (Self::ClosingStatements, "zh") => "当前幕——结辩：这是你的最后发言——你最有力的论点和你现在的立场；不引入新话题。",
            (Self::ClosingStatements, _) => "Acte — plaidoirie : c'est ton dernier mot — ton point le plus fort et où tu en es désormais ; aucun nouveau sujet.",
            (Self::Divergence, "en") => "Act — divergence: propose several distinct ideas, even wild ones, without judging them.",
            (Self::Divergence, "zh") => "当前幕——发散：提出几个不同的想法，哪怕很大胆，先不评判。",
            (Self::Divergence, _) => "Acte — divergence : propose plusieurs idées distinctes, même folles, sans les juger.",
            (Self::Association, "en") => "Act — association: take an idea someone else brought and combine or extend it.",
            (Self::Association, "zh") => "当前幕——联想：拿起别人提出的想法，将其组合或延伸。",
            (Self::Association, _) => "Acte — association : prends une idée apportée par un autre et combine-la ou prolonge-la.",
            (Self::Convergence, "en") => "Act — convergence: pick the one or two ideas worth keeping and justify the choice.",
            (Self::Convergence, "zh") => "当前幕——收敛：挑出一两个值得保留的想法并说明理由。",
            (Self::Convergence, _) => "Acte — convergence : retiens une ou deux idées qui valent le coup et justifie ce choix.",
            (Self::Proposal, "en") => "Act — proposals: bring a concrete, usable piece to the shared work.",
            (Self::Proposal, "zh") => "当前幕——提案：为共同成果贡献一个具体可用的部分。",
            (Self::Proposal, _) => "Acte — propositions : apporte un élément concret et utilisable au travail commun.",
            (Self::Critique, "en") => "Act — critique: examine what was proposed — a gap, a contradiction, an improvement.",
            (Self::Critique, "zh") => "当前幕——批评：审视已提出的内容——一个缺口、一个矛盾、一处改进。",
            (Self::Critique, _) => "Acte — critique : examine ce qui a été proposé — un manque, une contradiction, une amélioration.",
            (Self::Consolidation, "en") => "Act — consolidation: settle an open point and make the result stand on its own.",
            (Self::Consolidation, "zh") => "当前幕——整合：解决一个未决问题，让成果自成一体。",
            (Self::Consolidation, _) => "Acte — consolidation : tranche un point ouvert et rends le résultat autoportant.",
            (Self::Questioning, "en") => "Act — questioning: name an assumption behind what was said and question it.",
            (Self::Questioning, "zh") => "当前幕——提问：指出发言背后的一个假设并质疑它。",
            (Self::Questioning, _) => "Acte — questionnement : nomme un présupposé derrière ce qui a été dit et interroge-le.",
            (Self::Digging, "en") => "Act — digging deeper: answer the question you were asked, then ask a harder one.",
            (Self::Digging, "zh") => "当前幕——深挖：回答向你提出的问题，然后提出一个更难的问题。",
            (Self::Digging, _) => "Acte — creusement : réponds à la question qu'on t'a posée, puis pose-en une plus difficile.",
            (Self::Synthesis, "en") => "Act — synthesis: say what the questioning revealed and what you now hold as better founded.",
            (Self::Synthesis, "zh") => "当前幕——综合：说出追问揭示了什么，以及你现在认为哪些更有根据。",
            (Self::Synthesis, _) => "Acte — synthèse : dis ce que le questionnement a révélé et ce que tu tiens désormais pour mieux fondé.",
            (Self::Foundations, "en") => "Act — foundations: explain an essential notion clearly, without jargon.",
            (Self::Foundations, "zh") => "当前幕——基础：清晰地解释一个基本概念，不用行话。",
            (Self::Foundations, _) => "Acte — fondations : explique une notion essentielle, clairement, sans jargon.",
            (Self::Deepening, "en") => "Act — deepening: give an example, an edge case or a common mistake.",
            (Self::Deepening, "zh") => "当前幕——深入：给出一个例子、一个边界情况或一个常见错误。",
            (Self::Deepening, _) => "Acte — approfondissement : donne un exemple, un cas limite ou une erreur fréquente.",
            (Self::Recap, "en") => "Act — recap: restate what must be remembered, in order, nothing new.",
            (Self::Recap, "zh") => "当前幕——回顾：按顺序重述必须记住的要点，不加新内容。",
            (Self::Recap, _) => "Acte — récapitulation : reformule ce qu'il faut retenir, dans l'ordre, rien de nouveau.",
            (Self::Impressions, "en") => "Act — impressions: give your overall reading before any detail.",
            (Self::Impressions, "zh") => "当前幕——初步印象：在谈细节之前先给出整体感受。",
            (Self::Impressions, _) => "Acte — impressions : donne ta lecture d'ensemble avant tout détail.",
            (Self::Examination, "en") => "Act — examination: one precise point, with evidence.",
            (Self::Examination, "zh") => "当前幕——审视：一个明确的点，拿出证据。",
            (Self::Examination, _) => "Acte — examen : un point précis, preuves à l'appui.",
            (Self::Recommendations, "en") => "Act — recommendations: what to change first, what to keep.",
            (Self::Recommendations, "zh") => "当前幕——建议：先改什么，留什么。",
            (Self::Recommendations, _) => "Acte — recommandations : quoi changer en premier, quoi garder.",
            (Self::Exposition, "en") => "Act — exposition: set the world, a character and the first tension.",
            (Self::Exposition, "zh") => "当前幕——铺陈：建立世界、一个人物和第一重张力。",
            (Self::Exposition, _) => "Acte — exposition : installe le monde, un personnage et la première tension.",
            (Self::Complication, "en") => "Act — complication: add an obstacle that makes a choice costly.",
            (Self::Complication, "zh") => "当前幕——纠葛：加入一个让选择付出代价的障碍。",
            (Self::Complication, _) => "Acte — complication : ajoute un obstacle qui rend un choix coûteux.",
            (Self::Climax, "en") => "Act — climax: bring the decisive confrontation; something is lost for good.",
            (Self::Climax, "zh") => "当前幕——高潮：带来决定性的对峙；有些东西永远失去了。",
            (Self::Climax, _) => "Acte — climax : amène l'affrontement décisif ; quelque chose est perdu pour de bon.",
            (Self::Resolution, "en") => "Act — resolution: show the consequences and what remains of the characters; close the story.",
            (Self::Resolution, "zh") => "当前幕——结局：展示后果和人物身上留下的东西；收束故事。",
            (Self::Resolution, _) => "Acte — résolution : montre les conséquences et ce qu'il reste des personnages ; referme l'histoire.",
            (Self::Bargaining, "en") => "Act — bargaining: make a conditional offer or answer the last one; get something in return.",
            (Self::Bargaining, "zh") => "当前幕——讨价还价：提出有条件的报价或回应上一个报价；换取对等的让步。",
            (Self::Bargaining, _) => "Acte — marchandage : fais une offre conditionnelle ou réponds à la dernière ; obtiens une contrepartie.",
            (Self::Agreement, "en") => "Act — agreement: say precisely what you agree to sign and on which last condition; no new demand.",
            (Self::Agreement, "zh") => "当前幕——达成协议：明确说出你同意签署的内容和最后的条件；不再提新要求。",
            (Self::Agreement, _) => "Acte — accord : dis précisément ce que tu acceptes de signer et à quelle condition ultime ; aucune nouvelle demande.",
            (Self::Framing, "en") => "Act — framing: state what your hat sees in the question, without anticipating the conclusion.",
            (Self::Framing, "zh") => "当前幕——设定框架：说出你的帽子在这个问题上看到了什么，不要预设结论。",
            (Self::Framing, _) => "Acte — cadrage : pose ce que ton chapeau voit de la question, sans anticiper la conclusion.",
            (Self::Exploration, "en") => "Act — exploration: bring what this turn's hat reveals and the others have not seen.",
            (Self::Exploration, "zh") => "当前幕——探索：带来本轮帽子所揭示、而其他人尚未看到的东西。",
            (Self::Exploration, _) => "Acte — exploration : apporte ce que ton chapeau du tour révèle et que les autres n'ont pas vu.",
            (Self::Alert, "en") => "Act — alert: give your assessment from your expertise and one first urgent decision.",
            (Self::Alert, "zh") => "当前幕——警报：从你的专业角度给出现状评估和一个紧急的首要决定。",
            (Self::Alert, _) => "Acte — alerte : donne ton état des lieux depuis ton expertise et une première décision urgente.",
            (Self::Response, "en") => "Act — response: answer the latest dispatch with a concrete decision and say what you need from the others.",
            (Self::Response, "zh") => "当前幕——应对：用一个具体决定回应最新急电，并说明你需要其他人做什么。",
            (Self::Response, _) => "Acte — riposte : réagis à la dernière dépêche par une décision concrète et dis ce que tu attends des autres.",
            (Self::Debrief, "en") => "Act — debrief: review your decisions, own one mistake, draw one lesson; nothing new.",
            (Self::Debrief, "zh") => "当前幕——复盘：回顾你的决定，承认一个错误，总结一条经验；不引入新内容。",
            (Self::Debrief, _) => "Acte — débriefing : bilan de tes décisions, une erreur reconnue, une leçon pour la suite ; rien de nouveau.",
        }
    }

    /// Hint for the moderation prompt (what the moderator watches during the act).
    pub fn moderation_hint(&self, lang: &str) -> &'static str {
        match (self, lang) {
            (Self::Opening | Self::Divergence | Self::Proposal | Self::Questioning | Self::Foundations | Self::Impressions | Self::Exposition | Self::Framing | Self::Alert, "en") => "Act in progress: opening — let positions be stated, intervene only on the topic.",
            (Self::Opening | Self::Divergence | Self::Proposal | Self::Questioning | Self::Foundations | Self::Impressions | Self::Exposition | Self::Framing | Self::Alert, "zh") => "当前幕：开场——让各方陈述立场，只在偏题时介入。",
            (Self::Opening | Self::Divergence | Self::Proposal | Self::Questioning | Self::Foundations | Self::Impressions | Self::Exposition | Self::Framing | Self::Alert, _) => "Acte en cours : ouverture — laisse les positions se poser, n'interviens que sur le sujet.",
            (Self::Concessions | Self::Convergence | Self::Consolidation | Self::Synthesis | Self::Recap | Self::Recommendations | Self::Resolution | Self::Agreement | Self::Debrief, "en") => "Act in progress: winding down — reward concessions and convergence, curb new fronts.",
            (Self::Concessions | Self::Convergence | Self::Consolidation | Self::Synthesis | Self::Recap | Self::Recommendations | Self::Resolution | Self::Agreement | Self::Debrief, "zh") => "当前幕：收束——鼓励让步与收敛，抑制新的战线。",
            (Self::Concessions | Self::Convergence | Self::Consolidation | Self::Synthesis | Self::Recap | Self::Recommendations | Self::Resolution | Self::Agreement | Self::Debrief, _) => "Acte en cours : resserrement — valorise les concessions et la convergence, freine les nouveaux fronts.",
            (Self::ClosingStatements, "en") => "Act in progress: closing statements — no new topic, no personal attack; a comment only if a statement drifts.",
            (Self::ClosingStatements, "zh") => "当前幕：结辩——不引入新话题，不进行人身攻击；仅在发言跑偏时评论。",
            (Self::ClosingStatements, _) => "Acte en cours : plaidoiries — aucun nouveau sujet, aucune attaque personnelle ; un commentaire seulement si une plaidoirie dérive.",
            (_, "en") => "Act in progress: confrontation — sharp exchanges are expected; sanction only personal attacks or repeated off-topic.",
            (_, "zh") => "当前幕：交锋——激烈交锋在预期之中；仅对人身攻击或反复偏题予以处罚。",
            (_, _) => "Acte en cours : confrontation — les échanges vifs sont attendus ; ne sanctionne que les attaques personnelles ou le hors-sujet répété.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(turn: u32, max_turns: Option<u32>) -> TurnPosition {
        TurnPosition { turn, max_turns, stagnating: false, stop_requested: false }
    }

    #[test]
    fn debate_with_five_turns_walks_the_whole_script() {
        let acts: Vec<ActKey> = (1..=5).map(|t| resolve_act(&DiscussionMode::Debate, pos(t, Some(5))).unwrap()).collect();
        assert_eq!(acts, vec![ActKey::Opening, ActKey::Confrontation, ActKey::CrossExamination, ActKey::Concessions, ActKey::ClosingStatements]);
        // Longer discussions keep the proportions and every middle act
        let acts: Vec<ActKey> = (1..=10).map(|t| resolve_act(&DiscussionMode::Debate, pos(t, Some(10))).unwrap()).collect();
        assert_eq!(acts[0], ActKey::Opening);
        assert_eq!(acts[9], ActKey::ClosingStatements);
        assert!(acts.contains(&ActKey::CrossExamination) && acts.contains(&ActKey::Concessions));
        assert!(acts.windows(2).all(|w| index_of(DEBATE, w[0]) <= index_of(DEBATE, w[1])), "acts never go backwards: {acts:?}");
    }

    #[test]
    fn short_discussions_and_edge_cases() {
        // Two turns: opening then closing
        assert_eq!(resolve_act(&DiscussionMode::Debate, pos(1, Some(2))), Some(ActKey::Opening));
        assert_eq!(resolve_act(&DiscussionMode::Debate, pos(2, Some(2))), Some(ActKey::ClosingStatements));
        // One turn: the single turn is both opening and closing — the closing wins (last word)
        assert_eq!(resolve_act(&DiscussionMode::Debate, pos(1, Some(1))), Some(ActKey::ClosingStatements));
        // Three-act scripts
        let acts: Vec<ActKey> = (1..=4).map(|t| resolve_act(&DiscussionMode::Ideation, pos(t, Some(4))).unwrap()).collect();
        assert_eq!(acts, vec![ActKey::Divergence, ActKey::Association, ActKey::Association, ActKey::Convergence]);
        // UserDriven has no acts
        assert_eq!(resolve_act(&DiscussionMode::UserDriven, pos(2, Some(5))), None);
    }

    #[test]
    fn sliding_script_without_a_limit_reacts_to_stagnation_and_stop() {
        assert_eq!(resolve_act(&DiscussionMode::Debate, pos(1, None)), Some(ActKey::Opening));
        assert_eq!(resolve_act(&DiscussionMode::Debate, pos(7, None)), Some(ActKey::Confrontation));
        let stagnating = TurnPosition { stagnating: true, ..pos(4, None) };
        assert_eq!(resolve_act(&DiscussionMode::Debate, stagnating), Some(ActKey::Concessions));
        let stopped = TurnPosition { stop_requested: true, ..pos(3, None) };
        assert_eq!(resolve_act(&DiscussionMode::Debate, stopped), Some(ActKey::ClosingStatements));
        // With a limit, stagnation in the middle acts also jumps to concessions; never on the last turn
        let stagnating = TurnPosition { stagnating: true, ..pos(3, Some(10)) };
        assert_eq!(resolve_act(&DiscussionMode::Debate, stagnating), Some(ActKey::Concessions));
        let stagnating_last = TurnPosition { stagnating: true, ..pos(10, Some(10)) };
        assert_eq!(resolve_act(&DiscussionMode::Debate, stagnating_last), Some(ActKey::ClosingStatements));
    }

    #[test]
    fn every_act_has_its_three_texts_in_three_languages() {
        for mode in DiscussionMode::ALL.iter().filter(|m| **m != DiscussionMode::UserDriven) {
            let script = mode_script(mode);
            assert!(script.len() >= 3, "{mode:?}");
            for a in script {
                for lang in ["fr", "en", "zh"] {
                    assert!(!a.key.title(lang).is_empty() && !a.key.announcement(lang).is_empty() && !a.key.speaker_instruction(lang).is_empty() && !a.key.moderation_hint(lang).is_empty(), "{:?} {lang}", a.key);
                }
            }
        }
        assert_eq!(serde_json::to_string(&ActKey::ClosingStatements).unwrap(), "\"closingStatements\"");
    }
}
