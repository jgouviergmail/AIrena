use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ArgumentType {
    Support,
    Counter,
    Evidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThesisNode {
    pub id: String,
    pub label: String,
    pub speaker_id: String,
    pub speaker_name: String,
    pub arguments: Vec<ArgumentNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentNode {
    pub id: String,
    pub label: String,
    pub arg_type: ArgumentType,
    pub speaker_id: String,
    pub speaker_name: String,
    pub targets_thesis_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentMap {
    pub theses: Vec<ThesisNode>,
}

impl ArgumentMap {
    pub fn theses_count(&self) -> usize {
        self.theses.len()
    }

    pub fn arguments_count(&self) -> usize {
        self.theses.iter().map(|t| t.arguments.len()).sum()
    }

    fn arg_icon(arg_type: &ArgumentType) -> &'static str {
        match arg_type {
            ArgumentType::Support => "\u{2705}",
            ArgumentType::Counter => "\u{274C}",
            ArgumentType::Evidence => "\u{1F4CA}",
        }
    }

    /// Converts the argument map to hierarchical markdown for markmap rendering.
    /// Structure: `# Topic / ## Speaker / ### Thesis / - Arg` so markmap assigns
    /// one color per speaker (depth-2 branches).
    pub fn to_markdown(&self, topic: &str) -> String {
        // Collect unique speakers in order of first appearance
        let mut speakers: Vec<&str> = Vec::new();
        for thesis in &self.theses {
            if !speakers.contains(&thesis.speaker_name.as_str()) {
                speakers.push(&thesis.speaker_name);
            }
            for arg in &thesis.arguments {
                if !speakers.contains(&arg.speaker_name.as_str()) {
                    speakers.push(&arg.speaker_name);
                }
            }
        }

        let mut md = format!("# {topic}\n");

        for speaker in &speakers {
            md.push_str(&format!("## {speaker}\n"));

            // 1. This speaker's own theses with their arguments
            for thesis in &self.theses {
                if thesis.speaker_name == *speaker {
                    md.push_str(&format!("### {}\n", thesis.label));
                    for arg in &thesis.arguments {
                        let icon = Self::arg_icon(&arg.arg_type);
                        md.push_str(&format!("- {icon} {}\n", arg.label));
                    }
                }
            }

            // 2. This speaker's arguments on OTHER speakers' theses
            for thesis in &self.theses {
                if thesis.speaker_name != *speaker {
                    let cross_args: Vec<_> = thesis
                        .arguments
                        .iter()
                        .filter(|a| a.speaker_name == *speaker)
                        .collect();
                    if !cross_args.is_empty() {
                        let arrow = "\u{2192}"; // →
                        md.push_str(&format!("### {arrow} {}\n", thesis.label));
                        for arg in cross_args {
                            let icon = Self::arg_icon(&arg.arg_type);
                            md.push_str(&format!("- {icon} {}\n", arg.label));
                        }
                    }
                }
            }
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_markdown_empty() {
        let map = ArgumentMap::default();
        let md = map.to_markdown("Test Topic");
        assert_eq!(md, "# Test Topic\n");
    }

    #[test]
    fn test_to_markdown_with_theses() {
        let map = ArgumentMap {
            theses: vec![ThesisNode {
                id: "t-0".to_string(),
                label: "AI is beneficial".to_string(),
                speaker_id: "s1".to_string(),
                speaker_name: "Alice".to_string(),
                arguments: vec![
                    ArgumentNode {
                        id: "a-0-0".to_string(),
                        label: "Increases productivity".to_string(),
                        arg_type: ArgumentType::Support,
                        speaker_id: "s1".to_string(),
                        speaker_name: "Alice".to_string(),
                        targets_thesis_id: None,
                    },
                    ArgumentNode {
                        id: "a-0-1".to_string(),
                        label: "Job displacement risk".to_string(),
                        arg_type: ArgumentType::Counter,
                        speaker_id: "s2".to_string(),
                        speaker_name: "Bob".to_string(),
                        targets_thesis_id: Some("t-0".to_string()),
                    },
                    ArgumentNode {
                        id: "a-0-2".to_string(),
                        label: "McKinsey 2024 report".to_string(),
                        arg_type: ArgumentType::Evidence,
                        speaker_id: "s1".to_string(),
                        speaker_name: "Alice".to_string(),
                        targets_thesis_id: None,
                    },
                ],
            }],
        };
        let md = map.to_markdown("AI Debate");
        assert!(md.starts_with("# AI Debate\n"));
        // Alice's branch: her thesis with all args under it
        assert!(md.contains("## Alice\n"));
        assert!(md.contains("### AI is beneficial\n"));
        assert!(md.contains("- \u{2705} Increases productivity\n"));
        assert!(md.contains("- \u{1F4CA} McKinsey 2024 report\n"));
        // Bob's branch: cross-reference to Alice's thesis
        assert!(md.contains("## Bob\n"));
        assert!(md.contains("### \u{2192} AI is beneficial\n"));
        assert!(md.contains("- \u{274C} Job displacement risk\n"));
    }

    #[test]
    fn test_counts() {
        let map = ArgumentMap {
            theses: vec![
                ThesisNode {
                    id: "t-0".to_string(),
                    label: "Thesis A".to_string(),
                    speaker_id: "s1".to_string(),
                    speaker_name: "A".to_string(),
                    arguments: vec![ArgumentNode {
                        id: "a-0-0".to_string(),
                        label: "Arg 1".to_string(),
                        arg_type: ArgumentType::Support,
                        speaker_id: "s1".to_string(),
                        speaker_name: "A".to_string(),
                        targets_thesis_id: None,
                    }],
                },
                ThesisNode {
                    id: "t-1".to_string(),
                    label: "Thesis B".to_string(),
                    speaker_id: "s2".to_string(),
                    speaker_name: "B".to_string(),
                    arguments: vec![
                        ArgumentNode {
                            id: "a-1-0".to_string(),
                            label: "Arg 2".to_string(),
                            arg_type: ArgumentType::Counter,
                            speaker_id: "s1".to_string(),
                            speaker_name: "A".to_string(),
                            targets_thesis_id: Some("t-1".to_string()),
                        },
                        ArgumentNode {
                            id: "a-1-1".to_string(),
                            label: "Arg 3".to_string(),
                            arg_type: ArgumentType::Evidence,
                            speaker_id: "s2".to_string(),
                            speaker_name: "B".to_string(),
                            targets_thesis_id: None,
                        },
                    ],
                },
            ],
        };
        assert_eq!(map.theses_count(), 2);
        assert_eq!(map.arguments_count(), 3);
    }
}
