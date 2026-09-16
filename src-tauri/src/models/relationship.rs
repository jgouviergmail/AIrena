use serde::{Deserialize, Serialize};

/// One undirected edge of the participants' reaction graph, with the counts
/// in both directions (`a → b` and `b → a`) and the classified relationship
/// when it is strong enough ("ally" | "rival" | "tense").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipEdge {
    pub a: String,
    pub b: String,
    pub ab_likes: u32,
    pub ab_dislikes: u32,
    pub ba_likes: u32,
    pub ba_dislikes: u32,
    pub kind: Option<String>,
}
