# Batch A implementation checkpoint

## 2026-09-17 follow-up

Online doctor now invokes real Codex, incrementally decodes stdout, and retains private diagnostics including failures.
Live CLI integration test passed, verifying final JSON and 0600 diagnostic files.
Manual real invocation: Codex 0.154.0, thread 01a0ad29-6623-7d92-a80d-b78e28c404f6, exit 0, result {"ok":true}.
Sanitized observed event shapes are checked in as a parser fixture; no fake Provider was added.
The remaining list below records the earlier checkpoint: online wiring, streaming decoding and diagnostic retention are now implemented.
Database and owned process-group recovery remain batch B; no full S0 completion is claimed.

Date: 2026-09-16. Branch: feat/s0-codex.

## Completed

- Task 01: two-crate Rust workspace, CLI argument validation, domain access compatibility.
- Task 02 local portion: data-root validation, private directory creation, executable/version/flag checks, offline diagnostics.
- Task 03 partial: JSONL decoder, bounded subprocess capture, real read-only Codex probe and explicit opt-in test.

## Verification

- cargo test --workspace: 20 passed; 1 real-Codex test explicitly ignored by default.
- Explicit real_readonly_probe: failed (Codex exit 1).
- codex login status: logged in using ChatGPT.
- Installed Codex: 0.142.4.
- Direct CLI reproduction returned HTTP 400 invalid_request_error:
  The configured gpt-5.6-sol model requires a newer version of Codex.

The initial failure used no model override, permission bypass, or fake Provider.

## Authorized upgrade and retest

With user approval, the existing npm installation at
`/Users/mengweifei/.nvm/versions/node/v22.11.0` was upgraded from 0.142.4 to 0.154.0.
No model or login configuration was changed; login status remained ChatGPT.

Explicit real_readonly_probe retest: PASS (1 passed, 0 failed, 0 ignored).
Codex exited 0 and returned exactly `{"ok":true}`.
Observed thread ID: `01a0a9a5-034a-7451-91bf-6b6c8f467c39`.
CLI/model compatibility is no longer blocking the probe.

## Remaining before A completion

- Installed CLI/model compatibility and real read-only probe are now verified; complete the remaining integration work below.
- Wire the online probe into doctor (online doctor currently explicitly reports not implemented).
- Integrate incremental event decoding into live subprocess draining and persist diagnostic logs; current probe captures bounded output then decodes it.
- Capture a sanitized real event fixture and successful invocation evidence.
- Full process-group ownership/cancellation is Task 07; current capture controls only the direct child.
- Database diagnostics remain explicitly deferred to batch B.

Do not mark batch A or S0 accepted on the basis of offline tests.
