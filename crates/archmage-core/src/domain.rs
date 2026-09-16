#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceAccess {
    None,
    Read,
    Write,
}

pub fn compatible(a: WorkspaceAccess, b: WorkspaceAccess) -> bool {
    use WorkspaceAccess::*;
    matches!((a, b), (None, _) | (_, None) | (Read, Read))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionOutcome {
    Approved,
    ChangesRequested,
}

pub fn completion_exit_code(outcome: CompletionOutcome) -> i32 {
    match outcome {
        CompletionOutcome::Approved => 0,
        CompletionOutcome::ChangesRequested => 10,
    }
}
