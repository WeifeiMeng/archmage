[简体中文](../zh-CN/guide-online-doctor.md) | English

# Online Doctor: 2026-09-17 Update

This update was developed in the `codex/s0-runtime-fencing` worktree and had not yet been merged into main at the time of writing. The primary checkout still contained the previous version.

```bash
cd /Users/mengweifei/Desktop/code/github/archmage/.worktrees/s0-runtime-fencing
ARCHMAGE_HOME="$HOME/Library/Application Support/archmage-dev" cargo run -p archmage-cli -- doctor
```

This makes a real Codex call using the current local model configuration and login, consumes quota, uses a read-only sandbox, and has a 120-second deadline. stderr warns first; on success stdout prints JSON containing:

- `authentication=verified`
- `probe.thread_id`: real session ID
- `probe.exit_code=0`
- `probe.result={"ok":true}`
- `probe.diagnostics_dir`: diagnostics directory for this probe
- `database`: the real database inspection object, including `schema_version=5`, `foreign_keys=true`, `synchronous=2`, `journal_mode=wal`, and `busy_timeout_ms=5000`. This does not mean the complete workflow is available.

Every online call creates a separate `probe-*` directory containing `events.jsonl`, `stderr.log`, `result.json`, `metadata.json`, the Schema, and the probe repository. Events are decoded incrementally while the process runs; failures retain received logs and print the directory path. On Unix, directories use mode 0700 and log files use 0600. Diagnostics may contain sensitive material; do not upload or commit them directly.

Continue using `doctor --offline` when no model call is wanted. Online and offline diagnostics both create or open `archmage.db` in the data root and run migrations; neither creates a Workflow Run. At the time of this record, `run`, `status`, `logs`, `resume`, and `cancel` were still unimplemented.

## Verification

```bash
cargo test --workspace
cargo test -p archmage-cli --test doctor_live -- --ignored --nocapture
```

The second command is a real CLI integration and log-permission test, not a Fake Agent. The Core `codex_live` test remains available and cleans up its temporary files. CLI Doctor diagnostic directories are not deleted automatically.

## Known boundaries

Unix subprocesses already use separate process groups; timeout and explicit cancellation validate process identity and execute TERM/KILL. Database owner generation and complete crash-recovery orchestration were deferred to the later Runtime. Core already had real Git worktree provisioning, precise recovery reconciliation, ChangeSet Artifacts, and content-level Snapshots, but these were not yet wired into `run`. The read-only probe was not a general Agent invocation and did not prove four-capability collaboration.
