//! Roles of the structured modes (v1.19): who plays what in a trial or an
//! Oxford debate — set in the config (`mode_role`) or dealt by default from
//! the casting order — and the six thinking hats, which rotate every turn.

use crate::constants;
use crate::engine::truncate_str;
use crate::models::discussion::DiscussionMode;

pub const ROLE_PROSECUTOR: &str = "prosecutor";
pub const ROLE_DEFENSE: &str = "defense";
pub const ROLE_WITNESS: &str = "witness";
pub const ROLE_JUROR: &str = "juror";
pub const ROLE_FOR: &str = "for";
pub const ROLE_AGAINST: &str = "against";

/// Roles a trial offers (in the order they are dealt when nobody chose).
pub const TRIAL_ROLES: [&str; 4] = [ROLE_PROSECUTOR, ROLE_DEFENSE, ROLE_WITNESS, ROLE_JUROR];
/// The two camps of an Oxford debate.
pub const OXFORD_ROLES: [&str; 2] = [ROLE_FOR, ROLE_AGAINST];
/// The six thinking hats, in rotation order.
pub const HATS: [&str; 6] = ["white", "red", "black", "yellow", "green", "blue"];

/// Roles the user may assign in the wizard for this mode (empty when the mode
/// deals them itself or has none).
pub fn selectable_roles(mode: &DiscussionMode) -> &'static [&'static str] {
    match mode {
        DiscussionMode::Trial => &TRIAL_ROLES,
        DiscussionMode::OxfordDebate => &OXFORD_ROLES,
        _ => &[],
    }
}

/// Default role of the participant at `index` in the casting order: a trial
/// always gets a prosecutor and a defence first, then jurors and witnesses
/// alternate; an Oxford debate alternates the camps.
pub fn default_role(mode: &DiscussionMode, index: usize) -> Option<&'static str> {
    match mode {
        DiscussionMode::Trial => Some(match index {
            0 => ROLE_PROSECUTOR,
            1 => ROLE_DEFENSE,
            i if i % 2 == 0 => ROLE_JUROR,
            _ => ROLE_WITNESS,
        }),
        DiscussionMode::OxfordDebate => Some(OXFORD_ROLES[index % 2]),
        _ => None,
    }
}

/// The role a participant plays: the configured one when it belongs to the
/// mode, the default one otherwise (`None` for modes without roles).
pub fn resolve_role(mode: &DiscussionMode, configured: Option<&str>, index: usize) -> Option<&'static str> {
    let roles = selectable_roles(mode);
    configured
        .and_then(|c| roles.iter().copied().find(|r| *r == c))
        .or_else(|| default_role(mode, index))
}

/// Hat worn by the participant at `index` on `turn` (turns start at 1): every
/// turn shifts the hats by one, so each participant cycles through all six.
pub fn hat_for(turn: u32, index: usize) -> &'static str {
    HATS[(index + turn.saturating_sub(1) as usize) % HATS.len()]
}

/// Display label of a role or a hat.
pub fn role_label(role: &str, lang: &str) -> &'static str {
    match (role, lang) {
        (ROLE_PROSECUTOR, "en") => "Prosecution",
        (ROLE_PROSECUTOR, "zh") => "控方",
        (ROLE_PROSECUTOR, _) => "Accusation",
        (ROLE_DEFENSE, "en") => "Defence",
        (ROLE_DEFENSE, "zh") => "辩方",
        (ROLE_DEFENSE, _) => "Défense",
        (ROLE_WITNESS, "en") => "Witness",
        (ROLE_WITNESS, "zh") => "证人",
        (ROLE_WITNESS, _) => "Témoin",
        (ROLE_JUROR, "en") => "Juror",
        (ROLE_JUROR, "zh") => "陪审员",
        (ROLE_JUROR, _) => "Juré",
        (ROLE_FOR, "en") => "For the motion",
        (ROLE_FOR, "zh") => "正方",
        (ROLE_FOR, _) => "Camp Pour",
        (ROLE_AGAINST, "en") => "Against the motion",
        (ROLE_AGAINST, "zh") => "反方",
        (ROLE_AGAINST, _) => "Camp Contre",
        ("white", "en") => "White hat — facts",
        ("white", "zh") => "白帽——事实",
        ("white", _) => "Chapeau blanc — faits",
        ("red", "en") => "Red hat — feelings",
        ("red", "zh") => "红帽——情感",
        ("red", _) => "Chapeau rouge — émotions",
        ("black", "en") => "Black hat — caution",
        ("black", "zh") => "黑帽——谨慎",
        ("black", _) => "Chapeau noir — prudence",
        ("yellow", "en") => "Yellow hat — benefits",
        ("yellow", "zh") => "黄帽——益处",
        ("yellow", _) => "Chapeau jaune — bénéfices",
        ("green", "en") => "Green hat — creativity",
        ("green", "zh") => "绿帽——创意",
        ("green", _) => "Chapeau vert — créativité",
        ("blue", "en") => "Blue hat — process",
        ("blue", "zh") => "蓝帽——流程",
        ("blue", _) => "Chapeau bleu — processus",
        _ => "",
    }
}

/// What the role demands of the speaker (the body of the "[Ton rôle]" block).
fn role_brief(role: &str, lang: &str) -> &'static str {
    match (role, lang) {
        (ROLE_PROSECUTOR, "en") => "you carry the accusation: state the charges, establish the facts with evidence, question the witnesses, take the defence's arguments apart. You never concede the essential.",
        (ROLE_PROSECUTOR, "zh") => "你负责指控：陈述罪状，用证据确立事实，质询证人，拆解辩方的论点。你绝不在核心问题上让步。",
        (ROLE_PROSECUTOR, _) => "tu portes l'accusation : énonce les charges, établis les faits par les preuves, interroge les témoins, démonte les arguments de la défense. Tu ne concèdes jamais l'essentiel.",
        (ROLE_DEFENSE, "en") => "you defend: challenge every charge, raise doubt, protect what is accused, turn the witnesses' words around. The burden of proof is on the prosecution — remind everyone of it.",
        (ROLE_DEFENSE, "zh") => "你负责辩护：质疑每一项指控，提出疑点，保护被指控的一方，反转证人的证词。举证责任在控方——请提醒所有人。",
        (ROLE_DEFENSE, _) => "tu défends : conteste chaque charge, sème le doute, protège ce qui est accusé, retourne les propos des témoins. La charge de la preuve pèse sur l'accusation — rappelle-le.",
        (ROLE_WITNESS, "en") => "you testify: report what you know or saw, precisely and honestly, answer the questions put to you. You do not plead and you do not judge.",
        (ROLE_WITNESS, "zh") => "你是证人：准确而诚实地陈述你所知或所见，回答向你提出的问题。你不辩护，也不评判。",
        (ROLE_WITNESS, _) => "tu témoignes : rapporte ce que tu sais ou as vu, précisément et honnêtement, réponds aux questions qu'on te pose. Tu ne plaides pas et tu ne juges pas.",
        (ROLE_JUROR, "en") => "you sit on the jury: listen, ask the questions that would settle your doubt, weigh the evidence. You reserve your verdict until the end and let nothing sway you but the facts.",
        (ROLE_JUROR, "zh") => "你是陪审员：倾听，提出能消除疑虑的问题，权衡证据。你把裁决留到最后，只让事实左右你。",
        (ROLE_JUROR, _) => "tu sièges au jury : écoute, pose les questions qui lèveraient ton doute, pèse les preuves. Tu réserves ton verdict pour la fin et ne te laisses convaincre que par les faits.",
        (ROLE_FOR, "en") => "you argue FOR the motion, without ever wavering. Your target is the audience: make them switch sides with strong arguments, striking examples and loyal rebuttals.",
        (ROLE_FOR, "zh") => "你为正方辩护，绝不动摇。你的目标是听众：用有力的论据、鲜明的例子和公允的反驳让他们改变立场。",
        (ROLE_FOR, _) => "tu défends le camp POUR la motion, sans jamais fléchir. Ta cible est le public : fais-le basculer par des arguments forts, des exemples frappants et des réfutations loyales.",
        (ROLE_AGAINST, "en") => "you argue AGAINST the motion, without ever wavering. Your target is the audience: make them switch sides with strong arguments, striking examples and loyal rebuttals.",
        (ROLE_AGAINST, "zh") => "你为反方辩护，绝不动摇。你的目标是听众：用有力的论据、鲜明的例子和公允的反驳让他们改变立场。",
        (ROLE_AGAINST, _) => "tu défends le camp CONTRE la motion, sans jamais fléchir. Ta cible est le public : fais-le basculer par des arguments forts, des exemples frappants et des réfutations loyales.",
        ("white", "en") => "facts and data only: what we know, what we do not, what would be needed to know. No opinion, no feeling.",
        ("white", "zh") => "只谈事实和数据：我们知道什么、不知道什么、还需要知道什么。不发表意见，不带情绪。",
        ("white", _) => "faits et données uniquement : ce qu'on sait, ce qu'on ignore, ce qu'il faudrait savoir. Aucune opinion, aucun ressenti.",
        ("red", "en") => "feelings and intuitions, stated as such, without justifying them: what this inspires in you, what you sense, what worries or excites you.",
        ("red", "zh") => "情感和直觉，如实说出而不加辩解：这让你有什么感受、你察觉到什么、什么让你担忧或兴奋。",
        ("red", _) => "émotions et intuitions, dites comme telles, sans les justifier : ce que cela t'inspire, ce que tu pressens, ce qui t'inquiète ou t'enthousiasme.",
        ("black", "en") => "caution and risks: what can fail, the flaws, the costs, the reasons it will not work. Rigorous, not gloomy for its own sake.",
        ("black", "zh") => "谨慎与风险：可能失败之处、缺陷、成本、行不通的理由。严谨，而非为悲观而悲观。",
        ("black", _) => "prudence et risques : ce qui peut échouer, les failles, les coûts, les raisons pour lesquelles ça ne marchera pas. Rigoureux, pas sombre pour le plaisir.",
        ("yellow", "en") => "benefits and opportunities: why it can work, the value, the best-case scenario — with reasons, not wishful thinking.",
        ("yellow", "zh") => "益处与机会：为什么可行、价值所在、最佳情形——要有理由，而非一厢情愿。",
        ("yellow", _) => "bénéfices et opportunités : pourquoi ça peut marcher, la valeur, le meilleur scénario — avec des raisons, pas des vœux.",
        ("green", "en") => "creativity: alternatives, unexpected ideas, ways around the obstacles the others raised. Propose, do not judge.",
        ("green", "zh") => "创意：替代方案、意想不到的点子、绕过他人提出的障碍的方法。只提出，不评判。",
        ("green", _) => "créativité : alternatives, idées inattendues, manières de contourner les obstacles soulevés par les autres. Propose, ne juge pas.",
        ("blue", "en") => "process: where the group stands, what is missing, what to decide next and how. You steer the thinking, you do not add content.",
        ("blue", "zh") => "流程：小组进行到哪里、还缺什么、接下来决定什么以及如何决定。你引导思考，不添加内容。",
        ("blue", _) => "processus : où en est le groupe, ce qui manque, quoi décider ensuite et comment. Tu pilotes la réflexion, tu n'ajoutes pas de contenu.",
        _ => "",
    }
}

/// "[Ton rôle]" block appended to the persona for the modes with roles or
/// hats (bounded by `ROLE_BLOCK_MAX_CHARS`); empty for an unknown role.
pub fn role_block(mode: &DiscussionMode, role: &str, lang: &str) -> String {
    let brief = role_brief(role, lang);
    if brief.is_empty() {
        return String::new();
    }
    let label = role_label(role, lang);
    let header = match (mode, lang) {
        (DiscussionMode::SixHats, "en") => "[Your hat this turn — think with it and nothing else]",
        (DiscussionMode::SixHats, "zh") => "[你本轮的帽子——只用它思考]",
        (DiscussionMode::SixHats, _) => "[Ton chapeau ce tour — pense avec lui et rien d'autre]",
        (_, "en") => "[Your role — hold it from start to finish]",
        (_, "zh") => "[你的角色——自始至终坚持它]",
        (_, _) => "[Ton rôle — tiens-le du début à la fin]",
    };
    let block = format!("{header}\n{label} : {brief}");
    truncate_str(&block, constants::ROLE_BLOCK_MAX_CHARS).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_dealt_by_default_and_configured_ones_win_when_valid() {
        let dealt: Vec<_> = (0..6).map(|i| default_role(&DiscussionMode::Trial, i).unwrap()).collect();
        assert_eq!(dealt, vec![ROLE_PROSECUTOR, ROLE_DEFENSE, ROLE_JUROR, ROLE_WITNESS, ROLE_JUROR, ROLE_WITNESS]);
        let camps: Vec<_> = (0..4).map(|i| default_role(&DiscussionMode::OxfordDebate, i).unwrap()).collect();
        assert_eq!(camps, vec![ROLE_FOR, ROLE_AGAINST, ROLE_FOR, ROLE_AGAINST]);
        assert_eq!(resolve_role(&DiscussionMode::Trial, Some(ROLE_JUROR), 0), Some(ROLE_JUROR));
        // A role of another mode is ignored → default
        assert_eq!(resolve_role(&DiscussionMode::Trial, Some(ROLE_FOR), 0), Some(ROLE_PROSECUTOR));
        assert_eq!(resolve_role(&DiscussionMode::Debate, Some(ROLE_JUROR), 0), None);
        assert!(selectable_roles(&DiscussionMode::SixHats).is_empty());
    }

    #[test]
    fn hats_rotate_so_that_everyone_wears_each_one() {
        // Two participants, six turns: each sees the six hats, never the same one at once
        for index in 0..2 {
            let worn: std::collections::HashSet<&str> = (1..=6).map(|t| hat_for(t, index)).collect();
            assert_eq!(worn.len(), 6);
        }
        for turn in 1..=6 {
            assert_ne!(hat_for(turn, 0), hat_for(turn, 1));
        }
        assert_eq!(hat_for(1, 0), "white");
        assert_eq!(hat_for(2, 0), "red");
        assert_eq!(hat_for(7, 0), "white");
    }

    #[test]
    fn role_blocks_exist_in_three_languages_and_stay_bounded() {
        for role in TRIAL_ROLES.iter().chain(OXFORD_ROLES.iter()).chain(HATS.iter()) {
            for lang in ["fr", "en", "zh"] {
                let mode = if HATS.contains(role) { DiscussionMode::SixHats } else { DiscussionMode::Trial };
                let block = role_block(&mode, role, lang);
                assert!(block.starts_with('[') && block.contains(role_label(role, lang)), "{role} {lang}: {block}");
                assert!(block.chars().count() <= constants::ROLE_BLOCK_MAX_CHARS, "{role} {lang}");
            }
        }
        assert!(role_block(&DiscussionMode::Trial, "unknown", "fr").is_empty());
        assert!(role_block(&DiscussionMode::SixHats, "black", "fr").starts_with("[Ton chapeau ce tour"));
    }
}
