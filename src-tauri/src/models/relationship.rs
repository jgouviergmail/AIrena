use serde::{Deserialize, Serialize};

/// Classified relationship between two participants, once strong enough.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationshipKind {
    Ally,
    Rival,
    Tense,
}

impl RelationshipKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ally => "ally",
            Self::Rival => "rival",
            Self::Tense => "tense",
        }
    }
}

/// How an edge's net warmth moved since the previous emission (v1.17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationshipTrend {
    Warming,
    Cooling,
    Stable,
}

/// One undirected edge of the participants' reaction graph, with the counts
/// in both directions (`a → b` and `b → a`) and the classified relationship
/// when it is strong enough ("ally" | "rival" | "tense").
///
/// `score` is the net weighted warmth (approvals − disapprovals, both ways,
/// decayed each turn) and `trend` its move since the previous emission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipEdge {
    pub a: String,
    pub b: String,
    pub ab_likes: u32,
    pub ab_dislikes: u32,
    pub ba_likes: u32,
    pub ba_dislikes: u32,
    pub kind: Option<String>,
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub trend: Option<RelationshipTrend>,
}
