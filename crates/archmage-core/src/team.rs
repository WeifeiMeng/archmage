use crate::{
    error::{Error, Result},
    store::Store,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub name: String,
    pub revision: i64,
    pub graph: TeamGraph,
}

#[derive(Clone)]
pub struct TeamService {
    data_root: PathBuf,
}

impl TeamService {
    pub fn new(data_root: PathBuf) -> Self {
        Self { data_root }
    }
    fn store(&self) -> Result<Store> {
        Store::open(&self.data_root.join("archmage.db"))
    }
    pub fn list(&self) -> Result<Vec<Team>> {
        self.store()?.teams()
    }
    pub fn create(&self, name: &str, graph: &TeamGraph) -> Result<Team> {
        self.store()?.create_team(name, graph)
    }
    pub fn save(&self, id: &str, revision: i64, name: &str, graph: &TeamGraph) -> Result<Team> {
        self.store()?.save_team(id, revision, name, graph)
    }
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
        let keys: BTreeSet<_> = self
            .nodes
            .iter()
            .map(|node| node.capability.as_str())
            .collect();
        if !(2..=CAPABILITIES.len()).contains(&self.nodes.len())
            || keys.len() != self.nodes.len()
            || !keys.iter().all(|key| CAPABILITIES.contains(key))
            || self
                .nodes
                .iter()
                .any(|node| !(0..=2000).contains(&node.x) || !(0..=2000).contains(&node.y))
        {
            return Err(Error::Preflight(
                "team nodes or positions are invalid".into(),
            ));
        }
        let edges: BTreeSet<_> = self
            .edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();
        if edges.len() != self.edges.len()
            || !edges.iter().all(|(from, to)| {
                let from_order = CAPABILITIES.iter().position(|key| key == from);
                let to_order = CAPABILITIES.iter().position(|key| key == to);
                keys.contains(from)
                    && keys.contains(to)
                    && from_order.zip(to_order).is_some_and(|(a, b)| a < b)
            })
        {
            return Err(Error::Preflight("team edges are invalid".into()));
        }
        let roots: Vec<_> = keys
            .iter()
            .filter(|key| !edges.iter().any(|(_, to)| to == *key))
            .copied()
            .collect();
        if roots.len() != 1 {
            return Err(Error::Preflight(
                "team must have one connected entry".into(),
            ));
        }
        let mut reached = BTreeSet::from([roots[0]]);
        for key in CAPABILITIES {
            if reached.contains(key) {
                for (_, to) in edges.iter().filter(|(from, _)| *from == key) {
                    reached.insert(*to);
                }
            }
        }
        if reached != keys {
            return Err(Error::Preflight(
                "team contains disconnected capabilities".into(),
            ));
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
