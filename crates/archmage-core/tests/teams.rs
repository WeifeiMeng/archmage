use archmage_core::store::Store;
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
        ..TeamGraph::default()
    }
}

#[test]
fn graph_accepts_required_spine_and_forward_consultations() {
    graph().validate().unwrap();
}

#[test]
fn graph_accepts_connected_coder_reviewer_subset() {
    let graph = TeamGraph {
        nodes: vec![
            TeamNode {
                capability: "coder".into(),
                x: 120,
                y: 120,
            },
            TeamNode {
                capability: "reviewer".into(),
                x: 620,
                y: 120,
            },
        ],
        edges: vec![TeamEdge {
            from: "coder".into(),
            to: "reviewer".into(),
        }],
        ..TeamGraph::default()
    };
    graph.validate().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let saved = store.create_team("Code Review", &graph).unwrap();
    assert_eq!(store.team(&saved.id).unwrap().graph, graph);
}

#[test]
fn flow_graph_accepts_feedback_cycle_and_preserves_legacy_mode() {
    let legacy: TeamGraph = serde_json::from_value(serde_json::json!({
        "nodes":[{"capability":"coder","x":120,"y":120},{"capability":"reviewer","x":620,"y":120}],
        "edges":[{"from":"coder","to":"reviewer"}]
    }))
    .unwrap();
    legacy.validate().unwrap();
    let flow: TeamGraph = serde_json::from_value(serde_json::json!({
        "mode":"flow", "entry":"coder", "terminal":"reviewer",
        "nodes":[{"capability":"coder","x":120,"y":120},{"capability":"reviewer","x":620,"y":120}],
        "edges":[{"from":"coder","to":"reviewer"},{"from":"reviewer","to":"coder"}]
    }))
    .unwrap();
    flow.validate().unwrap();
    assert_eq!(serde_json::to_value(&flow).unwrap()["mode"], "flow");
}

#[test]
fn graph_rejects_singleton_and_disconnected_members() {
    let mut graph = TeamGraph {
        nodes: vec![TeamNode {
            capability: "coder".into(),
            x: 0,
            y: 0,
        }],
        edges: vec![],
        ..TeamGraph::default()
    };
    assert!(graph.validate().is_err());
    graph.nodes.push(TeamNode {
        capability: "reviewer".into(),
        x: 200,
        y: 0,
    });
    assert!(graph.validate().is_err());
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

#[test]
fn schema_ten_seeds_default_team_and_preserves_existing_data() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("archmage.db");
    let store = Store::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 15);
    assert_eq!(store.teams().unwrap().len(), 1);
    assert_eq!(store.teams().unwrap()[0].id, "default");
    assert_eq!(store.capability_bindings().unwrap().len(), 4);
}

#[test]
fn team_save_is_atomic_and_revision_guarded() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("archmage.db")).unwrap();
    let created = store.create_team("Review Team", &graph()).unwrap();
    assert_eq!(created.revision, 0);
    let changed = store
        .save_team(&created.id, 0, "New Name", &graph())
        .unwrap();
    assert_eq!(changed.revision, 1);
    assert!(store.save_team(&created.id, 0, "Stale", &graph()).is_err());
    assert!(store.create_team("New Name", &graph()).is_err());
    let mut invalid = graph();
    invalid.edges.clear();
    assert!(
        store
            .save_team(&created.id, 1, "Invalid", &invalid)
            .is_err()
    );
    assert_eq!(store.team(&created.id).unwrap().name, "New Name");
}
