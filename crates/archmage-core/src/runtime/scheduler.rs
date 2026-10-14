use super::NodeSnapshot;
use crate::team::TeamGraph;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalDecision {
    Continue,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewGate {
    Awaiting(Vec<String>),
    ChangesRequested(Vec<String>),
    Approved,
}

pub fn review_gate(outgoing: &[&str], decisions: &[(&str, &str, Vec<String>)]) -> ReviewGate {
    let missing: Vec<_> = outgoing
        .iter()
        .filter(|target| !decisions.iter().any(|(reviewer, _, _)| reviewer == *target))
        .map(|target| (*target).to_owned())
        .collect();
    if !missing.is_empty() {
        return ReviewGate::Awaiting(missing);
    }
    let suggestions = decisions
        .iter()
        .filter(|(_, decision, _)| *decision == "changes_requested")
        .flat_map(|(_, _, suggestions)| suggestions.iter().cloned())
        .collect::<Vec<_>>();
    if suggestions.is_empty() {
        ReviewGate::Approved
    } else {
        ReviewGate::ChangesRequested(suggestions)
    }
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

pub fn ready_keys_for_team<'a>(nodes: &[NodeSnapshot], graph: &'a TeamGraph) -> Vec<&'a str> {
    let states: BTreeMap<_, _> = nodes
        .iter()
        .map(|node| (node.key.as_str(), node.state.as_str()))
        .collect();
    crate::team::CAPABILITIES
        .into_iter()
        .filter(|key| {
            states.get(key).copied() == Some("pending")
                && graph
                    .edges
                    .iter()
                    .filter(|edge| edge.to == *key)
                    .all(|edge| states.get(edge.from.as_str()).copied() == Some("succeeded"))
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
    } else if !nodes.is_empty() && nodes.iter().all(|node| node.state == "succeeded") {
        TerminalDecision::Completed
    } else {
        TerminalDecision::Continue
    }
}
