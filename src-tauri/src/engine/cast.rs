//! Who is who (v1.20.3): a short portrait of every participant, read from the
//! persona kernel, so each speaker knows the others' role, creed and register —
//! and the moderator its cast — without carrying their whole prompts.

use crate::constants;
use crate::engine::truncate_at_word_boundary;

/// A participant as the others perceive them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastPortrait {
    pub name: String,
    /// The role line of `<identity>` ("Chercheur en sciences expérimentales"), if any
    pub headline: Option<String>,
    /// The creed of `<identity>` (the quoted line), if any
    pub creed: Option<String>,
    /// The `Registre:` line of `<voice>`, if any
    pub register: Option<String>,
}

fn section<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim())
}

/// Read a portrait from a persona system prompt. A prompt without a kernel
/// yields a portrait with the name only (custom personas written in prose).
pub fn portrait_from_kernel(name: &str, system_prompt: &str) -> CastPortrait {
    let identity = section(system_prompt, "identity");
    let mut lines = identity.map(|s| s.lines().map(str::trim).filter(|l| !l.is_empty())).into_iter().flatten();
    let headline = lines.next().and_then(|first| first.split_once(" — ").map(|(_, role)| role.trim().to_string()).filter(|r| !r.is_empty()));
    let creed = lines.next().map(|l| l.trim_matches(|c| c == '"' || c == '«' || c == '»' || c == ' ').to_string()).filter(|c| !c.is_empty());
    let register = section(system_prompt, "voice")
        .and_then(|v| v.lines().map(str::trim).find_map(|l| l.strip_prefix("Registre:").or_else(|| l.strip_prefix("Register:")).or_else(|| l.strip_prefix("语域:"))))
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty());
    CastPortrait { name: name.to_string(), headline, creed, register }
}

/// One portrait line, bounded.
fn portrait_line(p: &CastPortrait, lang: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(h) = &p.headline {
        parts.push(h.clone());
    }
    if let Some(c) = &p.creed {
        parts.push(format!("« {c} »"));
    }
    if let Some(r) = &p.register {
        let label = match lang {
            "en" => "register",
            "zh" => "语域",
            _ => "registre",
        };
        parts.push(format!("{label} {}", r.to_lowercase()));
    }
    let line = if parts.is_empty() { format!("- {}", p.name) } else { format!("- {} : {}", p.name, parts.join(" ; ")) };
    truncate_at_word_boundary(&line, constants::CAST_PORTRAIT_MAX_CHARS)
}

/// The "[Les autres participants]" block appended to a speaker's system prompt
/// (or "[Your cast]" for the moderator): who they are, in a few words each.
/// Empty when there is nobody to portray. Bounded by `CAST_BLOCK_MAX_CHARS`.
pub fn build_cast_block(portraits: &[CastPortrait], lang: &str, for_moderator: bool) -> String {
    if portraits.is_empty() {
        return String::new();
    }
    let (header, footer) = match (lang, for_moderator) {
        ("en", true) => ("[Your cast — who they are]", "Address each of them for who they are: their role, their creed, their way of speaking."),
        ("en", false) => ("[The other participants — who they are]", "Speak to them as the people they are: anticipate their angle, name their creed when you take it on, never confuse two of them."),
        ("zh", true) => ("[你的阵容——他们是谁]", "按他们各自的身份对待每一位：角色、信条、说话方式。"),
        ("zh", false) => ("[其他参与者——他们是谁]", "把他们当作真实的人来对话：预判其角度，在反驳时点出其信条，绝不混淆两个人。"),
        (_, true) => ("[Ton plateau — qui ils sont]", "Adresse-toi à chacun pour ce qu'il est : son rôle, son credo, sa façon de parler."),
        (_, false) => ("[Les autres participants — qui ils sont]", "Parle-leur comme aux personnes qu'ils sont : anticipe leur angle, nomme leur credo quand tu t'y attaques, ne confonds jamais deux d'entre eux."),
    };
    let mut lines: Vec<String> = Vec::new();
    let mut used = header.chars().count() + footer.chars().count() + 2;
    for p in portraits {
        let line = portrait_line(p, lang);
        let cost = line.chars().count() + 1;
        if used + cost > constants::CAST_BLOCK_MAX_CHARS {
            break;
        }
        used += cost;
        lines.push(line);
    }
    format!("{header}\n{}\n{footer}", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KERNEL: &str = "<system_kernel>\n<identity>\nLe Scientifique — Chercheur en sciences expérimentales\n\"Sans données reproductibles, vous n'avez qu'une anecdote.\"\nDocteur en sciences…\n</identity>\n<psychology>\nOCEAN: O=8 C=9 E=4 A=4 N=3\n</psychology>\n<voice>\nRegistre: SOUTENU, TECHNIQUE\nTics: \"Les données montrent que…\"\n</voice>\n</system_kernel>";

    /// v1.20.3 — a portrait is read from the kernel; prose personas keep their name only.
    #[test]
    fn portraits_come_from_the_kernel_and_degrade_to_the_name() {
        let p = portrait_from_kernel("Le Scientifique", KERNEL);
        assert_eq!(p.headline.as_deref(), Some("Chercheur en sciences expérimentales"));
        assert_eq!(p.creed.as_deref(), Some("Sans données reproductibles, vous n'avez qu'une anecdote."));
        assert_eq!(p.register.as_deref(), Some("SOUTENU, TECHNIQUE"));
        let prose = portrait_from_kernel("Mon perso", "Tu es un pirate bourru qui parle fort.");
        assert_eq!(prose, CastPortrait { name: "Mon perso".into(), headline: None, creed: None, register: None });
    }

    /// The block names everyone in a few words, is bounded, and differs for the moderator.
    #[test]
    fn cast_block_is_bounded_and_addresses_the_reader() {
        let sci = portrait_from_kernel("Le Scientifique", KERNEL);
        let prose = portrait_from_kernel("Mon perso", "prose");
        let block = build_cast_block(&[sci.clone(), prose], "fr", false);
        assert!(block.starts_with("[Les autres participants — qui ils sont]\n- Le Scientifique : Chercheur en sciences expérimentales ; « Sans données reproductibles, vous n'avez qu'une anecdote. » ; registre soutenu, technique\n- Mon perso\n"), "{block}");
        assert!(block.ends_with("ne confonds jamais deux d'entre eux."));
        let moderator = build_cast_block(std::slice::from_ref(&sci), "en", true);
        assert!(moderator.starts_with("[Your cast — who they are]") && moderator.contains("register soutenu, technique"), "{moderator}");
        assert!(build_cast_block(&[], "fr", false).is_empty());
        // Bounded: a crowd of long portraits is cut at the block's cap, never mid-line
        let crowd: Vec<CastPortrait> = (0..40).map(|i| CastPortrait { name: format!("Participant {i}"), headline: Some("x".repeat(150)), creed: None, register: None }).collect();
        let big = build_cast_block(&crowd, "fr", false);
        assert!(big.chars().count() <= constants::CAST_BLOCK_MAX_CHARS + 1, "{}", big.chars().count());
        assert!(big.lines().all(|l| l.chars().count() <= constants::CAST_PORTRAIT_MAX_CHARS + 1));
    }
}
