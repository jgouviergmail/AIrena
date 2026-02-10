//! Parse the `<dynamics>` section from a persona's system prompt.
//! Extracts trilingual fields (FR/EN/ZH) into a flat struct.

/// Parsed dynamics fields from a persona's `<dynamics>` section.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParsedDynamics {
    pub values: String,
    pub triggers: String,
    pub under_pressure: String,
    pub confident: String,
    pub disengaged: String,
    /// Arbitre-specific field (only present for arbitre personas)
    pub enthusiastic: Option<String>,
}

/// Attempt to parse the `<dynamics>` block from a system prompt.
/// Returns `None` if no `<dynamics>` section is found.
pub fn parse_dynamics(system_prompt: &str) -> Option<ParsedDynamics> {
    // Extract content between <dynamics> and </dynamics>
    let start = system_prompt.find("<dynamics>")?;
    let end = system_prompt.find("</dynamics>")?;
    if end <= start {
        return None;
    }
    let content = &system_prompt[start + "<dynamics>".len()..end];

    Some(ParsedDynamics {
        values: extract_field(content, &["Valeurs:", "Values:", "价值观:"]),
        triggers: extract_field(content, &["Déclencheurs:", "Triggers:", "触发点:"]),
        under_pressure: extract_field(content, &["Sous pression:", "Under pressure:", "承压时:"]),
        confident: extract_field(content, &["En confiance:", "Confident:", "自信时:"]),
        disengaged: extract_field(content, &["Désengagé:", "Disengaged:", "无兴趣时:"]),
        enthusiastic: {
            let val = extract_field(content, &["Enthousiaste:", "Enthusiastic:", "热情时:"]);
            if val.is_empty() { None } else { Some(val) }
        },
    })
}

/// All known field labels in order of appearance (used for "next label" detection).
const ALL_LABELS: &[&[&str]] = &[
    &["Valeurs:", "Values:", "价值观:"],
    &["Déclencheurs:", "Triggers:", "触发点:"],
    &["Sous pression:", "Under pressure:", "承压时:"],
    &["En confiance:", "Confident:", "自信时:"],
    &["Désengagé:", "Disengaged:", "无兴趣时:"],
    &["Enthousiaste:", "Enthusiastic:", "热情时:"],
    // Arbitre-specific moderation labels (act as terminators if present)
    &["Style:", "Style:", "风格:"],
    &["Recadrage:", "Redirection:", "纠偏:"],
];

/// Extract a field value by finding its label and taking text until the next label or end.
fn extract_field(content: &str, labels: &[&str]) -> String {
    // Find the label position (try all language variants)
    let (label_end, _) = labels
        .iter()
        .filter_map(|label| {
            content.find(label).map(|pos| (pos + label.len(), label))
        })
        .min_by_key(|(_, _)| 0) // take first found
        .unwrap_or((0, &""));

    if label_end == 0 {
        return String::new();
    }

    let rest = &content[label_end..];

    // Find the next label start (any language variant of any field)
    let next_label_pos = ALL_LABELS
        .iter()
        .flat_map(|label_group| {
            label_group.iter().filter_map(|label| {
                // Don't match the label we just found
                if labels.contains(label) {
                    None
                } else {
                    // Look for label at start of a line
                    rest.find(label)
                }
            })
        })
        .min();

    let value = match next_label_pos {
        Some(pos) => &rest[..pos],
        None => rest,
    };

    value.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gladiateur_dynamics_fr() {
        let prompt = r#"<persona>
<identity>Le Scientifique</identity>
<dynamics>
Valeurs: La méthode scientifique, la reproductibilité.
Déclencheurs: Arguments d'autorité non sourcés.
Sous pression: Devient glacial et méthodique.
En confiance: Généreux en explications.
Désengagé: Répond par des faits bruts.
</dynamics>
</persona>"#;

        let parsed = parse_dynamics(prompt).unwrap();
        assert_eq!(parsed.values, "La méthode scientifique, la reproductibilité.");
        assert_eq!(parsed.triggers, "Arguments d'autorité non sourcés.");
        assert!(parsed.under_pressure.starts_with("Devient glacial"));
        assert!(parsed.confident.starts_with("Généreux"));
        assert!(parsed.disengaged.starts_with("Répond par"));
        assert!(parsed.enthusiastic.is_none());
    }

    #[test]
    fn test_parse_dynamics_en() {
        let prompt = r#"<dynamics>
Values: The scientific method.
Triggers: Unsourced authority arguments.
Under pressure: Becomes cold and methodical.
Confident: Generous with explanations.
Disengaged: Responds with bare facts.
</dynamics>"#;

        let parsed = parse_dynamics(prompt).unwrap();
        assert_eq!(parsed.values, "The scientific method.");
        assert!(parsed.under_pressure.contains("cold"));
    }

    #[test]
    fn test_parse_dynamics_zh() {
        let prompt = r#"<dynamics>
价值观: 科学方法。
触发点: 无来源的权威论证。
承压时: 变得冰冷而有条理。
自信时: 慷慨地解释。
无兴趣时: 只回应简单事实。
</dynamics>"#;

        let parsed = parse_dynamics(prompt).unwrap();
        assert_eq!(parsed.values, "科学方法。");
    }

    #[test]
    fn test_parse_arbitre_dynamics() {
        let prompt = r#"<dynamics>
Sous pression: Reste calme mais ferme.
Enthousiaste: Encourage vivement les échanges.
</dynamics>"#;

        let parsed = parse_dynamics(prompt).unwrap();
        assert!(parsed.under_pressure.starts_with("Reste calme"));
        assert!(parsed.enthusiastic.is_some());
        assert!(parsed.enthusiastic.unwrap().starts_with("Encourage"));
    }

    #[test]
    fn test_no_dynamics_section() {
        let prompt = "<persona><identity>Test</identity></persona>";
        assert!(parse_dynamics(prompt).is_none());
    }

    #[test]
    fn test_empty_dynamics() {
        let prompt = "<dynamics>\n</dynamics>";
        let parsed = parse_dynamics(prompt).unwrap();
        assert!(parsed.values.is_empty());
        assert!(parsed.triggers.is_empty());
    }
}
