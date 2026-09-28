use super::NodeSnapshot;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalDecision {
    Continue,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

fn dependencies(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "designer" => Some(&[]),
        "coder" => Some(&["designer"]),
        "tester" => Some(&["coder"]),
        "reviewer" => Some(&["tester"]),
        _ => None,
    }
}

pub fn ready_keys(nodes: &[NodeSnapshot]) -> Vec<&'static str> {
    let states: BTreeMap<_, _> = nodes
        .iter()
        .map(|node| (node.key.as_str(), node.state.as_str()))
        .collect();
    ["designer", "coder", "tester", "reviewer"]
        .into_iter()
        .filter(|key| {
            states.get(key).copied() == Some("pending")
                && dependencies(key).is_some_and(|required| {
                    required
                        .iter()
                        .all(|dependency| states.get(dependency).copied() == Some("succeeded"))
                })
        })
        .collect()
}

pub fn terminal_decision(nodes: &[NodeSnapshot]) -> TerminalDecision {
    if nodes.iter().any(|node| node.state == "cancelled") {
        TerminalDecision::Cancelled
    } else if nodes.iter().any(|node| node.state == "failed") {
        TerminalDecision::Failed
    } else if nodes.iter().any(|node| node.state == "interrupted") {
        TerminalDecision::Interrupted
    } else if nodes.len() == 4 && nodes.iter().all(|node| node.state == "succeeded") {
        TerminalDecision::Completed
    } else {
        TerminalDecision::Continue
    }
}
