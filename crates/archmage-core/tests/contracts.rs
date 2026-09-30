use archmage_core::{
    capability::CapabilityKind,
    contract::{ContractContext, validate_builtin_schemas, validate_output},
};

#[test]
fn consultation_contract_requires_actionable_rejection() {
    use archmage_core::contract::validate_consultation;
    assert!(validate_consultation(br#"{"schema_version":"1","decision":"changes_requested","summary":"Needs work","suggestions":[]}"#).is_err());
    assert!(validate_consultation(br#"{"schema_version":"1","decision":"changes_requested","summary":"Needs work","suggestions":["Add error cases"]}"#).is_ok());
    assert!(validate_consultation(br#"{"schema_version":"1","decision":"approved","summary":"Looks good","suggestions":["Change this"]}"#).is_err());
}
use serde_json::json;

#[test]
fn builtin_schemas_are_valid_and_strict() {
    validate_builtin_schemas().unwrap();
    let extra = br#"{"schema_version":"1","summary_markdown":"s","design_markdown":"d","acceptance_criteria":["a"],"implementation_steps":["i"],"extra":true}"#;
    assert!(validate_output(CapabilityKind::Designer, extra, &ContractContext::default()).is_err());
    assert!(
        validate_output(
            CapabilityKind::Designer,
            b"not json",
            &ContractContext::default()
        )
        .is_err()
    );
}

#[test]
fn designer_requires_nonempty_design_and_criteria() {
    let valid = json!({"schema_version":"1","summary_markdown":"summary","design_markdown":"design","acceptance_criteria":["works"],"implementation_steps":["code"]});
    assert!(
        validate_output(
            CapabilityKind::Designer,
            &serde_json::to_vec(&valid).unwrap(),
            &ContractContext::default()
        )
        .is_ok()
    );
    let empty = json!({"schema_version":"1","summary_markdown":"s","design_markdown":" ","acceptance_criteria":[],"implementation_steps":[]});
    assert!(
        validate_output(
            CapabilityKind::Designer,
            &serde_json::to_vec(&empty).unwrap(),
            &ContractContext::default()
        )
        .is_err()
    );
}

#[test]
fn tester_reports_commands_it_selected_without_required_check_ids() {
    let valid = json!({
        "schema_version":"1",
        "summary_markdown":"Tests passed",
        "status":"passed",
        "commands":[{
            "command":"cargo test",
            "exit_code":0,
            "status":"passed",
            "summary":"12 passed"
        }],
        "limitations":[]
    });
    assert!(
        validate_output(
            CapabilityKind::Tester,
            &serde_json::to_vec(&valid).unwrap(),
            &ContractContext::default()
        )
        .is_ok()
    );
    let contradictory = json!({
        "schema_version":"1",
        "summary_markdown":"Claims success",
        "status":"passed",
        "commands":[{"command":"cargo test","exit_code":1,"status":"failed","summary":"failed"}],
        "limitations":[]
    });
    assert!(
        validate_output(
            CapabilityKind::Tester,
            &serde_json::to_vec(&contradictory).unwrap(),
            &ContractContext::default()
        )
        .is_err()
    );
}

#[test]
fn reviewer_cannot_approve_failed_or_blocked_testing_or_unsafe_paths() {
    let approved =
        json!({"schema_version":"1","summary_markdown":"ok","status":"approved","issues":[]});
    let failed = ContractContext {
        tester_accepted: false,
    };
    assert!(
        validate_output(
            CapabilityKind::Reviewer,
            &serde_json::to_vec(&approved).unwrap(),
            &failed
        )
        .is_err()
    );
    let unsafe_path = json!({"schema_version":"1","summary_markdown":"issue","status":"changes_requested","issues":[{"severity":"P1","path":"../outside","message":"bad"}]});
    assert!(
        validate_output(
            CapabilityKind::Reviewer,
            &serde_json::to_vec(&unsafe_path).unwrap(),
            &ContractContext {
                tester_accepted: true,
            }
        )
        .is_err()
    );
}

#[test]
fn coder_uses_fixed_status_enum() {
    let valid = json!({"schema_version":"1","summary_markdown":"done","status":"implemented","changed_paths":["src/lib.rs"],"notes":[]});
    assert!(
        validate_output(
            CapabilityKind::Coder,
            &serde_json::to_vec(&valid).unwrap(),
            &ContractContext::default()
        )
        .is_ok()
    );
    let wrong = json!({"schema_version":"1","summary_markdown":"done","status":"success","changed_paths":[],"notes":[]});
    assert!(
        validate_output(
            CapabilityKind::Coder,
            &serde_json::to_vec(&wrong).unwrap(),
            &ContractContext::default()
        )
        .is_err()
    );
}
