use archmage_core::provider::codex::validate_help;
use archmage_core::provider::codex_event::Decoder;

#[test]
fn fragmented_utf8_unknown_and_unterminated_events_are_preserved() {
    let input = "{\"type\":\"thread.started\",\"thread_id\":\"abc\"}\n{\"type\":\"future.event\",\"text\":\"中文\"}\n{\"type\":\"turn.completed\"}";
    let mut d = Decoder::new(1024);
    let mut events = Vec::new();
    for b in input.as_bytes() {
        events.extend(d.push(&[*b]).unwrap());
    }
    events.extend(d.finish().unwrap());
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["thread_id"], "abc");
    assert_eq!(events[1]["text"], "中文");
    assert!(events[2].get("thread_id").is_none());
}

#[test]
fn invalid_and_oversized_lines_are_rejected() {
    assert!(Decoder::new(4).push(b"12345").is_err());
    assert!(Decoder::new(1024).push(b"not json\n").is_err());
    assert!(Decoder::new(1024).push(b"[]\n").is_err());
}

#[test]
fn failure_events_are_preserved() {
    let events = Decoder::new(1024)
        .push(b"{\"type\":\"turn.failed\"}\n{\"type\":\"error\"}\n")
        .unwrap();
    assert_eq!(events[0]["type"], "turn.failed");
    assert_eq!(events[1]["type"], "error");
}

#[test]
fn all_required_flags_must_be_present() {
    let flags = "--cd --sandbox --json --output-schema --output-last-message";
    assert!(validate_help(flags).is_ok());
    let error = validate_help("--json").unwrap_err().to_string();
    assert!(error.contains("--output-schema"));
}
