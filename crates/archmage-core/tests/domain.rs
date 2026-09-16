use archmage_core::domain::{CompletionOutcome, WorkspaceAccess, compatible, completion_exit_code};

#[test]
fn readers_share_but_writer_is_exclusive() {
    use WorkspaceAccess::*;
    for a in [None, Read, Write] {
        for b in [None, Read, Write] {
            assert_eq!(
                compatible(a, b),
                a == None || b == None || (a == Read && b == Read)
            );
        }
    }
}

#[test]
fn review_decision_is_not_execution_success() {
    assert_eq!(completion_exit_code(CompletionOutcome::Approved), 0);
    assert_eq!(
        completion_exit_code(CompletionOutcome::ChangesRequested),
        10
    );
}
