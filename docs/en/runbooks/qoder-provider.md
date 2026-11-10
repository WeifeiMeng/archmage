[简体中文](../../zh-CN/runbooks/qoder-provider.md) | English

# Qoder ACP Provider

Archmage can run any Designer, Coder, Tester, or Reviewer Capability through a local Qoder CLI process using ACP v1. This integration controls the CLI process and its ACP session; it does not automate the Qoder IDE or WorkBuddy GUI.

## Install and authenticate

Install Qoder CLI with the [official installation guide](https://docs.qoder.com/cli/installation):

```sh
# macOS or Linux
curl -fsSL https://qoder.com/install | bash

# Windows PowerShell
irm https://qoder.com/install.ps1 | iex
```

Verify `qoder --version`, then run `qoder` and sign in. Archmage reuses the existing Qoder CLI authentication or `QODER_PERSONAL_ACCESS_TOKEN` from its launch environment. It never stores or displays the token.

## Configure an instance

In **Agent Management**, add a Qoder instance and configure:

- an optional explicit executable path; blank searches `PATH`;
- an optional model and Qoder Agent name;
- an explicit JSON object for extensions, initially `{ "mcpServers": [] }`;
- optional `pluginDirs`, `allowedTools`, `disallowedTools`, `agents`, and `settings` values in that object.

Archmage starts Qoder with ACP, strict MCP configuration, and no implicit setting sources. Authentication is reused, but personal settings, Memory, the default Agent, MCP servers, plugins, and skills are not inherited unless explicitly frozen in the instance configuration.

Use **Check CLI** for a local version check. **Verify connection** creates a temporary read-only Git repository, negotiates ACP v1, creates a session, makes one real model request, and accepts only the exact requested JSON object.

## Sessions and permissions

Each live `Run + Capability` owns one Qoder process and ACP session. Later deliveries to the same Capability reuse it; other Capabilities and Runs never share it. Consultations use temporary sessions.

- Designer and Reviewer are read-only.
- Coder and Tester may write only inside the Run worktree.
- ACP file paths are canonicalized; symlink escapes and outside paths are rejected.
- Terminal cwd must stay inside the worktree. Privilege escalation and protected destructive targets are rejected.
- Final assistant output must be exactly one bounded JSON object. Markdown fences and repair heuristics are rejected.

Run completion closes sessions. Cancellation sends `session/cancel` and then terminates the registered process group after a grace period. A process lost during restart leaves the running Attempt `interrupted`; Archmage does not silently manufacture continuity.

## Diagnostics and verification

Bounded, private diagnostics are stored below the relevant Attempt or probe directory. They do not contain tokens, the complete environment, unbounded conversation text, or thought content.

Deterministic tests use the repository fake ACP process. To run the authenticated live smoke test:

```sh
cargo test -p archmage-core --test qoder_live -- --ignored --nocapture
```

The live test requires an installed, authenticated Qoder CLI, network access, and available model quota.

## Troubleshooting

- **Executable inspection failed:** run `qoder --version` in the environment that launches Archmage or set the executable path explicitly.
- **Connection verification failed:** run Qoder interactively and sign in, or provide `QODER_PERSONAL_ACCESS_TOKEN` in the Archmage launch environment.
- **ACP or contract error:** confirm the installed Qoder version supports ACP v1 and that the Agent returns one plain JSON object.
- **Configuration rejected:** the extension value must be a JSON object; `mcpServers`, when present, must be an array.

See also [cross-provider Capability handoff](../architecture/cross-provider-handoff.md).
