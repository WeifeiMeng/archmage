use archmage_core::provider::codex::validate_help;

#[test]
fn all_required_flags_must_be_present() {
    let flags = "--cd --sandbox --json --output-schema --output-last-message";
    assert!(validate_help(flags).is_ok());
    let error = validate_help("--json").unwrap_err().to_string();
    assert!(error.contains("--output-schema"));
}
