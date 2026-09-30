use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const DEFAULT_TEAM_ID: &str = "default";
pub const CAPABILITIES: [&str; 4] = ["designer", "coder", "tester", "reviewer"];
pub const REQUIRED_EDGES: [(&str, &str); 3] = [
    ("designer", "coder"),
    ("coder", "tester"),
    ("tester", "reviewer"),
];
pub const OPTIONAL_EDGES: [(&str, &str); 3] = [
    ("designer", "tester"),
    ("designer", "reviewer"),
    ("coder", "reviewer"),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamNode {
    pub capability: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamEdge {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamGraph {
    pub nodes: Vec<TeamNode>,
    pub edges: Vec<TeamEdge>,
}

impl TeamGraph {
    pub fn builtin() -> Self {
        Self {
            nodes: CAPABILITIES
                .into_iter()
                .enumerate()
                .map(|(index, capability)| TeamNode {
                    capability: capability.into(),
                    x: 100 + index as i32 * 240,
                    y: 120,
                })
                .collect(),
            edges: REQUIRED_EDGES
                .into_iter()
                .chain(OPTIONAL_EDGES)
                .map(|(from, to)| TeamEdge {
                    from: from.into(),
                    to: to.into(),
                })
                .collect(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        let keys: BTreeSet<_> = self.nodes.iter().map(|node| node.capability.as_str()).collect();
        if self.nodes.len() != CAPABILITIES.len()
            || keys != CAPABILITIES.into_iter().collect()
            || self
                .nodes
                .iter()
                .any(|node| !(0..=2000).contains(&node.x) || !(0..=2000).contains(&node.y))
        {
            return Err(Error::Preflight("team nodes or positions are invalid".into()));
        }
        let edges: BTreeSet<_> = self
            .edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();
        if edges.len() != self.edges.len()
            || !REQUIRED_EDGES.into_iter().all(|edge| edges.contains(&edge))
            || !edges
                .iter()
                .all(|edge| REQUIRED_EDGES.contains(edge) || OPTIONAL_EDGES.contains(edge))
        {
            return Err(Error::Preflight("team edges are invalid".into()));
        }
        Ok(())
    }

    pub fn outgoing<'a>(&'a self, capability: &'a str) -> impl Iterator<Item = &'a str> {
        self.edges
            .iter()
            .filter(move |edge| edge.from == capability)
            .map(|edge| edge.to.as_str())
    }
}
