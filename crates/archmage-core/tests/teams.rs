use archmage_core::team::{TeamEdge, TeamGraph, TeamNode};

fn graph() -> TeamGraph {
    TeamGraph {
        nodes: ["designer", "coder", "tester", "reviewer"]
            .into_iter()
            .enumerate()
            .map(|(index, capability)| TeamNode {
                capability: capability.into(),
                x: 100 + index as i32 * 240,
                y: 120,
            })
            .collect(),
        edges: [
            ("designer", "coder"),
            ("coder", "tester"),
            ("tester", "reviewer"),
            ("designer", "reviewer"),
        ]
        .into_iter()
        .map(|(from, to)| TeamEdge {
            from: from.into(),
            to: to.into(),
        })
        .collect(),
    }
}

#[test]
fn graph_accepts_required_spine_and_forward_consultations() {
    graph().validate().unwrap();
}

#[test]
fn graph_rejects_missing_spine_or_reverse_edge() {
    let mut missing = graph();
    missing.edges.remove(0);
    assert!(missing.validate().is_err());
    let mut reverse = graph();
    reverse.edges.push(TeamEdge {
        from: "reviewer".into(),
        to: "designer".into(),
    });
    assert!(reverse.validate().is_err());
}

#[test]
fn graph_rejects_duplicate_nodes_edges_and_out_of_bounds_positions() {
    let mut duplicate_node = graph();
    duplicate_node.nodes[1].capability = "designer".into();
    assert!(duplicate_node.validate().is_err());
    let mut duplicate_edge = graph();
    duplicate_edge.edges.push(duplicate_edge.edges[0].clone());
    assert!(duplicate_edge.validate().is_err());
    let mut outside = graph();
    outside.nodes[0].x = -1;
    assert!(outside.validate().is_err());
}
