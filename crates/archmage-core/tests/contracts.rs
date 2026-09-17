use archmage_core::{
    capability::CapabilityKind,
    contract::{ContractContext, RequiredCheck, validate_builtin_schemas, validate_output},
};
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
fn tester_must_supply_exact_canonical_evidence() {
    let context = ContractContext {
        required_checks: vec![RequiredCheck {
            check_id: "unit".into(),
            evidence_id: "artifact-1".into(),
            passed: false,
        }],
        ..Default::default()
    };
    let valid = json!({"schema_version":"1","summary_markdown":"failed as expected","status":"failed","checks":[{"check_id":"unit","evidence_id":"artifact-1","status":"failed"}]});
    assert!(
        validate_output(
            CapabilityKind::Tester,
            &serde_json::to_vec(&valid).unwrap(),
            &context
        )
        .is_ok()
    );
    let forged = json!({"schema_version":"1","summary_markdown":"x","status":"passed","checks":[{"check_id":"unit","evidence_id":"fake","status":"passed"}]});
    assert!(
        validate_output(
            CapabilityKind::Tester,
            &serde_json::to_vec(&forged).unwrap(),
            &context
        )
        .is_err()
    );
}

#[test]
fn reviewer_cannot_approve_failed_checks_or_unsafe_paths() {
    let approved =
        json!({"schema_version":"1","summary_markdown":"ok","status":"approved","issues":[]});
    let failed = ContractContext {
        checks_failed: true,
        ..Default::default()
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
            &ContractContext::default()
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
