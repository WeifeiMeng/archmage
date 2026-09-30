use archmage_core::runtime::consultation::render_consultation_prompt;
use serde_json::json;

#[test]
fn consultation_prompt_uses_target_persona_and_source_attempt() {
    let rendered = render_consultation_prompt(
        "tester",
        "designer",
        "attempt-1",
        "build feature",
        &json!({"design":"spec"}),
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&rendered).unwrap();
    assert_eq!(value["target_capability"], "tester");
    assert_eq!(value["source_attempt_id"], "attempt-1");
    assert!(
        value["persona_for_perspective_only"]
            .as_str()
            .unwrap()
            .contains("Tester")
    );
    assert_eq!(value["source_output"]["design"], "spec");
    assert!(
        value["stage_rule"]
            .as_str()
            .unwrap()
            .contains("ChangeSet, test execution")
    );
    let reviewer =
        render_consultation_prompt("reviewer", "designer", "attempt-2", "goal", &json!({}))
            .unwrap();
    let reviewer: serde_json::Value = serde_json::from_slice(&reviewer).unwrap();
    assert!(
        reviewer["stage_rule"]
            .as_str()
            .unwrap()
            .contains("NEVER a reason to reject")
    );
}
