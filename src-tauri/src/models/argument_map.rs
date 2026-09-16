use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::constants;

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
    #[serde(default)]
    pub children: Vec<ArgumentNode>,
}

impl ArgumentNode {
    /// Recursively count this argument + all descendants.
    pub fn count_all(&self) -> usize {
        1 + self.children.iter().map(|c| c.count_all()).sum::<usize>()
    }

    /// Find the depth of an argument matching `label_lower` (already lowercased).
    /// Returns `Some(depth)` where depth is relative to the starting `current_depth`.
    pub fn find_depth_by_label(&self, label_lower: &str, current_depth: usize) -> Option<usize> {
        let self_lower = self.label.to_lowercase();
        if self_lower == label_lower
            || self_lower.contains(label_lower)
            || label_lower.contains(&self_lower)
        {
            return Some(current_depth);
        }
        for child in &self.children {
            if let Some(d) = child.find_depth_by_label(label_lower, current_depth + 1) {
                return Some(d);
            }
        }
        None
    }

    /// Find an argument by label (fuzzy, case-insensitive containment) in the tree.
    /// `label_lower` must already be lowercased by the caller.
    /// Returns a mutable reference to the matching node, or None.
    pub fn find_by_label_mut(&mut self, label_lower: &str) -> Option<&mut ArgumentNode> {
        let self_lower = self.label.to_lowercase();
        if self_lower == label_lower
            || self_lower.contains(label_lower)
            || label_lower.contains(&self_lower)
        {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_by_label_mut(label_lower) {
                return Some(found);
            }
        }
        None
    }
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

    /// Total argument count across all theses, recursively including nested children.
    pub fn arguments_count(&self) -> usize {
        self.theses
            .iter()
            .flat_map(|t| &t.arguments)
            .map(|a| a.count_all())
            .sum()
    }

    pub(crate) fn arg_icon(arg_type: &ArgumentType) -> &'static str {
        match arg_type {
            ArgumentType::Support => "\u{2705}",
            ArgumentType::Counter => "\u{274C}",
            ArgumentType::Evidence => "\u{1F4CA}",
        }
    }

    /// Thesis-centric markdown: `# Topic / ## Thesis (Speaker) / - Icon Speaker: Arg`
    /// Each thesis gets its own markmap color (depth-2 branch). Speaker names inline.
    /// Nodes whose id is in `new_ids` are flagged as new (latest extraction).
    pub fn to_markdown(&self, topic: &str, new_ids: &HashSet<String>) -> String {
        let mut md = format!("# {topic}\n");

        for thesis in &self.theses {
            md.push_str(&format!("## {}{} ({})\n", Self::marker(new_ids, &thesis.id), thesis.label, thesis.speaker_name));
            for arg in &thesis.arguments {
                Self::render_argument_recursive(&mut md, arg, 0, new_ids);
            }
        }

        md
    }

    /// Speaker-centric markdown: `# Topic / ## Speaker / ### Thesis / - Icon Speaker: Arg`
    /// Each speaker gets its own markmap color (depth-2 branch).
    /// Theses appear under their owner; cross-speaker arguments are inline with speaker name.
    /// A speaker who only contributed arguments gets an "Arguments" branch (label per
    /// `lang`) listing them with the thesis they target. New nodes are flagged.
    pub fn to_markdown_by_speaker(&self, topic: &str, new_ids: &HashSet<String>, lang: &str) -> String {
        // Speakers in order of first appearance: thesis owners first, then argument-only ones
        let mut speakers: Vec<&str> = Vec::new();
        for thesis in &self.theses {
            if !speakers.contains(&thesis.speaker_name.as_str()) {
                speakers.push(&thesis.speaker_name);
            }
        }
        for thesis in &self.theses {
            for arg in &thesis.arguments {
                Self::collect_speakers(arg, &mut speakers);
            }
        }

        let mut md = format!("# {topic}\n");

        for speaker in &speakers {
            md.push_str(&format!("## {speaker}\n"));
            let owns_thesis = self.theses.iter().any(|t| t.speaker_name == *speaker);
            if owns_thesis {
                for thesis in self.theses.iter().filter(|t| t.speaker_name == *speaker) {
                    md.push_str(&format!("### {}{}\n", Self::marker(new_ids, &thesis.id), thesis.label));
                    for arg in &thesis.arguments {
                        Self::render_argument_recursive(&mut md, arg, 0, new_ids);
                    }
                }
            } else {
                let branch = match lang {
                    "en" => "Arguments",
                    "zh" => "论据",
                    _ => "Arguments",
                };
                md.push_str(&format!("### {branch}\n"));
                for thesis in &self.theses {
                    for arg in &thesis.arguments {
                        Self::render_own_arguments(&mut md, arg, speaker, &thesis.label, new_ids);
                    }
                }
            }
        }

        md
    }

    fn marker(new_ids: &HashSet<String>, id: &str) -> &'static str {
        if new_ids.contains(id) { constants::ARGMAP_NEW_MARKER_PREFIX } else { "" }
    }

    /// Render a single argument and its children recursively with indentation.
    fn render_argument_recursive(md: &mut String, arg: &ArgumentNode, indent_level: usize, new_ids: &HashSet<String>) {
        let indent = "  ".repeat(indent_level);
        let icon = Self::arg_icon(&arg.arg_type);
        md.push_str(&format!(
            "{indent}- {icon} {}{}: {}\n",
            Self::marker(new_ids, &arg.id), arg.speaker_name, arg.label
        ));
        for child in &arg.children {
            Self::render_argument_recursive(md, child, indent_level + 1, new_ids);
        }
    }

    /// Flat list of the arguments a given speaker made anywhere under a thesis,
    /// each with the thesis it relates to.
    fn render_own_arguments(md: &mut String, arg: &ArgumentNode, speaker: &str, thesis_label: &str, new_ids: &HashSet<String>) {
        if arg.speaker_name == speaker {
            let icon = Self::arg_icon(&arg.arg_type);
            md.push_str(&format!(
                "- {icon} {}{} (→ {thesis_label})\n",
                Self::marker(new_ids, &arg.id), arg.label
            ));
        }
        for child in &arg.children {
            Self::render_own_arguments(md, child, speaker, thesis_label, new_ids);
        }
    }

    fn collect_speakers<'a>(arg: &'a ArgumentNode, speakers: &mut Vec<&'a str>) {
        if !speakers.contains(&arg.speaker_name.as_str()) {
            speakers.push(&arg.speaker_name);
        }
        for child in &arg.children {
            Self::collect_speakers(child, speakers);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_arg(
        id: &str,
        label: &str,
        arg_type: ArgumentType,
        speaker_id: &str,
        speaker_name: &str,
        children: Vec<ArgumentNode>,
    ) -> ArgumentNode {
        ArgumentNode {
            id: id.into(),
            label: label.into(),
            arg_type,
            speaker_id: speaker_id.into(),
            speaker_name: speaker_name.into(),
            targets_thesis_id: None,
            children,
        }
    }

    #[test]
    fn test_to_markdown_empty() {
        let map = ArgumentMap::default();
        let md = map.to_markdown("Test Topic", &HashSet::new());
        assert_eq!(md, "# Test Topic\n");
    }

    #[test]
    fn test_to_markdown_with_theses() {
        let map = ArgumentMap {
            theses: vec![ThesisNode {
                id: "t-0".into(),
                label: "AI is beneficial".into(),
                speaker_id: "s1".into(),
                speaker_name: "Alice".into(),
                arguments: vec![
                    make_arg("a-0", "Increases productivity", ArgumentType::Support, "s1", "Alice", vec![]),
                    make_arg("a-1", "Job displacement risk", ArgumentType::Counter, "s2", "Bob", vec![]),
                    make_arg("a-2", "McKinsey 2024 report", ArgumentType::Evidence, "s1", "Alice", vec![]),
                ],
            }],
        };
        let md = map.to_markdown("AI Debate", &HashSet::new());
        assert!(md.starts_with("# AI Debate\n"));
        // Thesis-centric: thesis with speaker attribution
        assert!(md.contains("## AI is beneficial (Alice)\n"));
        // Arguments with speaker inline
        assert!(md.contains("- \u{2705} Alice: Increases productivity\n"));
        assert!(md.contains("- \u{274C} Bob: Job displacement risk\n"));
        assert!(md.contains("- \u{1F4CA} Alice: McKinsey 2024 report\n"));
    }

    #[test]
    fn test_counts() {
        let map = ArgumentMap {
            theses: vec![
                ThesisNode {
                    id: "t-0".into(),
                    label: "Thesis A".into(),
                    speaker_id: "s1".into(),
                    speaker_name: "A".into(),
                    arguments: vec![make_arg("a-0", "Arg 1", ArgumentType::Support, "s1", "A", vec![])],
                },
                ThesisNode {
                    id: "t-1".into(),
                    label: "Thesis B".into(),
                    speaker_id: "s2".into(),
                    speaker_name: "B".into(),
                    arguments: vec![
                        make_arg("a-1", "Arg 2", ArgumentType::Counter, "s1", "A", vec![]),
                        make_arg("a-2", "Arg 3", ArgumentType::Evidence, "s2", "B", vec![]),
                    ],
                },
            ],
        };
        assert_eq!(map.theses_count(), 2);
        assert_eq!(map.arguments_count(), 3);
    }

    #[test]
    fn test_to_markdown_recursive() {
        let map = ArgumentMap {
            theses: vec![ThesisNode {
                id: "t-0".into(),
                label: "AI is beneficial".into(),
                speaker_id: "s1".into(),
                speaker_name: "Alice".into(),
                arguments: vec![make_arg(
                    "a-0",
                    "Increases productivity",
                    ArgumentType::Support,
                    "s1",
                    "Alice",
                    vec![make_arg(
                        "a-1",
                        "Most gains go to corporations",
                        ArgumentType::Counter,
                        "s2",
                        "Bob",
                        vec![make_arg(
                            "a-2",
                            "OECD shows broad wage growth",
                            ArgumentType::Evidence,
                            "s1",
                            "Alice",
                            vec![],
                        )],
                    )],
                )],
            }],
        };
        let md = map.to_markdown("AI Debate", &HashSet::new());
        assert!(md.contains("- \u{2705} Alice: Increases productivity\n"));
        assert!(md.contains("  - \u{274C} Bob: Most gains go to corporations\n"));
        assert!(md.contains("    - \u{1F4CA} Alice: OECD shows broad wage growth\n"));
    }

    #[test]
    fn test_arguments_count_recursive() {
        let map = ArgumentMap {
            theses: vec![ThesisNode {
                id: "t-0".into(),
                label: "Thesis".into(),
                speaker_id: "s1".into(),
                speaker_name: "A".into(),
                arguments: vec![make_arg(
                    "a-0",
                    "Arg",
                    ArgumentType::Support,
                    "s1",
                    "A",
                    vec![
                        make_arg("a-1", "Child 1", ArgumentType::Counter, "s2", "B", vec![]),
                        make_arg("a-2", "Child 2", ArgumentType::Evidence, "s1", "A", vec![]),
                    ],
                )],
            }],
        };
        assert_eq!(map.arguments_count(), 3); // 1 parent + 2 children
    }

    #[test]
    fn test_find_depth_by_label() {
        let arg = make_arg(
            "a-0",
            "Top level argument",
            ArgumentType::Support,
            "s1",
            "A",
            vec![make_arg(
                "a-1",
                "Nested counter",
                ArgumentType::Counter,
                "s2",
                "B",
                vec![make_arg(
                    "a-2",
                    "Deep refutation",
                    ArgumentType::Support,
                    "s1",
                    "A",
                    vec![],
                )],
            )],
        );

        // Exact match at depth 1
        assert_eq!(arg.find_depth_by_label("top level argument", 1), Some(1));
        // Nested at depth 2
        assert_eq!(arg.find_depth_by_label("nested counter", 1), Some(2));
        // Deep at depth 3
        assert_eq!(arg.find_depth_by_label("deep refutation", 1), Some(3));
        // Not found
        assert_eq!(arg.find_depth_by_label("nonexistent", 1), None);
    }

    #[test]
    fn test_find_by_label_mut() {
        let mut arg = make_arg(
            "a-0",
            "Top level argument",
            ArgumentType::Support,
            "s1",
            "A",
            vec![make_arg(
                "a-1",
                "Nested counter",
                ArgumentType::Counter,
                "s2",
                "B",
                vec![],
            )],
        );

        // Find nested child
        let found = arg.find_by_label_mut("nested counter");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "a-1");

        // Find self
        let found = arg.find_by_label_mut("top level argument");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "a-0");
    }

    #[test]
    fn test_by_speaker_includes_argument_only_speakers_and_new_markers() {
        let map = ArgumentMap {
            theses: vec![ThesisNode {
                id: "t-0".into(),
                label: "AI is beneficial".into(),
                speaker_id: "s1".into(),
                speaker_name: "Alice".into(),
                arguments: vec![make_arg(
                    "a-0",
                    "Increases productivity",
                    ArgumentType::Support,
                    "s1",
                    "Alice",
                    vec![make_arg("a-1", "Gains go to shareholders", ArgumentType::Counter, "s2", "Bob", vec![])],
                )],
            }],
        };
        let md = map.to_markdown_by_speaker("AI Debate", &HashSet::new(), "fr");
        assert!(md.contains("## Bob\n### Arguments\n- \u{274C} Gains go to shareholders (→ AI is beneficial)\n"), "{md}");
        let zh = map.to_markdown_by_speaker("AI Debate", &HashSet::new(), "zh");
        assert!(zh.contains("### 论据\n"));

        let new_ids: HashSet<String> = ["t-0".to_string(), "a-1".to_string()].into_iter().collect();
        let marked = map.to_markdown("AI Debate", &new_ids);
        assert!(marked.contains("## ✨ AI is beneficial (Alice)\n"), "{marked}");
        assert!(marked.contains("  - \u{274C} ✨ Bob: Gains go to shareholders\n"), "{marked}");
        assert!(marked.contains("- \u{2705} Alice: Increases productivity\n"), "unchanged node has no marker");
        let by_speaker = map.to_markdown_by_speaker("AI Debate", &new_ids, "en");
        assert!(by_speaker.contains("### ✨ AI is beneficial\n"));
        assert!(by_speaker.contains("- \u{274C} ✨ Gains go to shareholders (→ AI is beneficial)\n"), "{by_speaker}");
    }

    #[test]
    fn test_to_markdown_by_speaker_recursive() {
        let map = ArgumentMap {
            theses: vec![ThesisNode {
                id: "t-0".into(),
                label: "AI is beneficial".into(),
                speaker_id: "s1".into(),
                speaker_name: "Alice".into(),
                arguments: vec![make_arg(
                    "a-0",
                    "Increases productivity",
                    ArgumentType::Support,
                    "s1",
                    "Alice",
                    vec![make_arg(
                        "a-1",
                        "Job displacement risk",
                        ArgumentType::Counter,
                        "s2",
                        "Bob",
                        vec![],
                    )],
                )],
            }],
        };
        let md = map.to_markdown_by_speaker("AI Debate", &HashSet::new(), "fr");
        assert!(md.starts_with("# AI Debate\n"));
        // Speaker-centric: speaker at depth-2, thesis at depth-3
        assert!(md.contains("## Alice\n"));
        assert!(md.contains("### AI is beneficial\n"));
        // Arguments with recursive nesting
        assert!(md.contains("- \u{2705} Alice: Increases productivity\n"));
        assert!(md.contains("  - \u{274C} Bob: Job displacement risk\n"));
    }
}
