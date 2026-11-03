# DeepSeek Harness Provider Support Design

[简体中文](../../../zh-CN/superpowers/specs/2026-10-30-deepseek-harness-provider-design.md) | English

**Status:** Adjusted and confirmed according to the plan on 2026-10-30; this design replaces the previous plan to modify the DSH headless CLI.

## target

Add a real-executable DeepSeek Harness Provider to Archmage. Users can create, inspect, and verify DeepSeek Harness instances in agent management, bind Designers, Coders, Testers, or Reviewers to the instances, and complete real runs through existing Linear Teams, Advisory Edges, and Message Flow Teams.

Only DeepSeek Harness is implemented at this stage. Hermes, Claude, and OpenClaw continue to show as not yet supported.

## integration boundary

Archmage does not modify, fork, or require upstream merging of DeepSeek Harness source code. The integration only relies on the Python SDK `deepseek-harness-sdk` released by DeepSeek Harness and its same version bundled runtime `deepseek-harness-runtime-bin`.

Archmage provides a self-maintained lightweight Python bridge.bridge imports the official `deepseek_harness` module, uses the SDK's high-level owned-run API to start the bundled JSON-RPC runtime, and holds the same named session between multiple calls. The Rust Runtime only exchanges well-bounded JSONL messages with the bridge, does not import DSH internal TypeScript packages, and does not rely on unpublished headless parameters.

The bundled default config of the SDK only guarantees the basic runtime and is not equivalent to the complete set of coding tools required by Archmage. Archmage therefore ships a Cordis configuration with itself, passed into the bundled runtime via the SDK's public `cordis` parameter. This configuration only references the runtime wheel's packaged public plug-ins, explicitly combining the JSON-RPC server, Agent, model adapter, persistence, sandbox-bound shell and file tools, automatic rejection approval policy, and necessary contextual components; the DSH source code is not copied or modified.

The bridge process inherits the current Archmage environment, including `DSH_HOME`, Provider credentials, endpoint, and other environment variables supported by the SDK.The official SDK's `initialize` requires the Provider route and model to be provided explicitly, so these two items are configured by the Agent instance rather than guessed from the DSH profile.

## Agent instances and storage

- Added Provider type `deepseek_harness` and marked it to support real execution.
- DeepSeek Harness Agent instances save the Python executable path, Provider route, model, display name, and enabled status. Looks for `python3` from `PATH` when the Python path is left blank.
- The DSH instance's Provider route and model must be non-null; the Codex instance does not use the Provider route and continues to allow its own model override semantics.
- Database migration extends the `agent_instances.provider_kind` constraint and adds nullable `provider_route`. All existing instances and capability bindings are retained after upgrading.
- The default DSH instance with the guessed model is not created. The user explicitly adds and verifies the instance before binding Capability.
- Run is created with the instance ID, display name, provider type, resolved Python path, provider route, and model frozen. Modifying the global Agent configuration later does not change the existing Run.

## Sidecar and session ownership

Archmage lazily starts a separate bridge sidecar for each `Run + Capability`. Sidecar has an official SDK client and a stable named session ID internally:

- In the same run, when the same Capability is executed again due to Reviewer feedback, retry, or message flow, the same sidecar and the same DSH session will continue to be used.
- Different Runs or Capabilities never share sidecars or sessions, even if they are bound to the same Agent instance.
- Sidecar survives from the first call to Run completion, failure, cancellation, or Runtime recycling; the final state must close the SDK client and wait for the bridge and bundled runtime to exit.
- The sidecar enters a permanent failure state after exiting unexpectedly. This Run does not silently create a replacement session, so as not to disguise the lost history as a continuous conversation.
- The consultation call uses an independent temporary bridge and temporary session, and is closed immediately after completion without entering the main dialog of the target capability.

The first version only guarantees continuous dialogue within the life cycle of Archmage Run. After the Archmage or desktop client is restarted, the DSH session is not restored; the existing recovery logic terminates the legacy process and converges the corresponding Run to `interrupted`. Graphical resume is not in the current product scope, so no cross-restart commitment is added.

## Bridge protocol

Archmage publishes the bridge script as its own resource and launches it through Python. The startup parameters or first control message include the working directory, provider route, model, session ID, and output limit. After the process starts:

1. bridge constructs the official SDK client and passes in the Cordis configuration released by Archmage; the SDK parses it by itself and starts the bundled runtime of the same version.
2. Rust sends a single-line JSON request with the invocation ID, task text, and JSON Schema for each task.
3. Bridge calls the SDK on a fixed named session and waits for the current activity interval to the next idle.
4. bridge returns the final committed assistant text, session ID, and bounded status summary for this interval.
5. Rust strictly parses the complete text into a JSON object and then enters the existing Capability contract verification.

bridge stdout only allows protocol frames; SDK/runtime stderr is saved as a bounded diagnostic stream. Invalid JSON, unknown invocation, duplicate completion, out-of-bounds output, or protocol order errors terminate the sidecar and fail the current attempt. The implementation does not extract JSON loosely from description text or Markdown.

## Runtime integration

The runtime holds an in-process, shareable Provider sidecar pool with `(run_id, capability)` as the primary key. Ordinary nodes and message flow execution obtain or create sidecars through the unified Provider executor; the consulting path explicitly requests temporary sidecars.

Each DSH task still uses the existing frozen Context Snapshot, Capability directive, and target JSON Schema.bridge only provides model conversation continuity within the same Capability and is not a replacement for Archmage's explicit Handoff, Artifact, Changeset, or output contracts.

The SDK runtime process is hosted by the process group of the bridge. Archmage registers and verifies bridge PID, PGID and startup identity:

- Designer, Reviewer, and Consulting are launched via `DSH_PERMISSION_MODE=read-only`; Coder, Tester use `DSH_PERMISSION_MODE=workspace-write`. The Cordis configuration must combine a sandbox policy, a constrained shell/file tool, and an approval policy that automatically denies without interaction, and cannot rely solely on prompt words to declare permissions.
- There is currently no prompt-level cancellation for the SDK protocol. When an active attempt is canceled, Archmage terminates the entire sidecar process group for that capability; independent sidecars for other capabilities are not affected.
- When Run enters any final state, it closes and recycles all its sidecars; recycling failures are recorded but failed Runs cannot be rewritten as successful.
- Timeouts, process exits, and protocol corruption are all Provider failures that cannot publish successful results or Changesets.

## Discovery and connection verification

"Check SDK" parses the Python executable path and confirms it in an isolated child process:

- Python can be started;
- `deepseek_harness` can be imported;
- SDK and bundled runtime packages can be resolved to compatible versions;
- The Agent's Provider route and model configurations are not empty.

"Verify connection" results in a real model call. Archmage starts a temporary read-only bridge in a temporary Git working directory, uses the configured Provider route and model, requires a fixed JSON object to be returned, and then completely shuts down the SDK runtime. Success indicates that the Python SDK, bundled runtime, credentials, model routing, and JSONL bridge are currently available.

Changing the Python path, Provider route, model, or enablement status will invalidate existing verification status. Authentication failure only returns a security digest without credentials to the interface, and the original stderr is saved in the Archmage private diagnostic directory.

## Agent and Capability Management

- The new instance operation of agent management supports the selection of Codex or DeepSeek Harness.
- The DeepSeek Harness card edits the instance name, Python path, Provider route, model, and activation status, and displays the SDK/runtime version, connection status, and last verification time.
- The page explains that credentials and endpoints are inherited from the environment when Archmage is launched; provider route and model are explicitly configured by the instance.
- Page metrics count all supported Provider instances, no longer just Codex.
- Capability management allows selection of any enabled Codex or DeepSeek Harness instance and displays the real Provider name.
- Hermes, Claude, and OpenClaw retain "Not yet supported" cards and cannot create runnable instances or bind capabilities.

## Failure and safety boundaries

Python/SDK Missing, version incompatible, initialization failed, model call failed, non-zero exit, timeout, cancellation, output out of bounds, invalid UTF-8, invalid JSON, and Capability contract mismatch each produce an explicit failure. The diagnostic catalog uses private permissions and does not log provider credentials, full environments, or unrestricted session content.

The Agent configuration and management API only returns the Python path, Provider route, model, version and security status, and does not return the API key, endpoint secret or `DSH_HOME` content. The bridge environment is inherited as a whole, but the diagnostic metadata only records the allowed fields.

The official SDK JSON-RPC protocol is currently a pre-release interface and there is no protocol version negotiation. Archmage centrally isolates this dependency in the bridge and verifies the known server identity/version; incompatible versions fail explicitly and do not propagate protocol details to runtime scheduling and UI.

## Minimum verification scope

- The bridge test uses a controllable Python SDK stub, covering startup, two rounds of execution of the same named session, strict result boundaries, abnormal exit and shutdown.
- Provider testing uses a controllable JSONL bridge, covering discovery, real configuration freezes, output caps, timeouts, and process group cancellation.
- The runtime test proves that the same `Run + Capability` reuses the same sidecar/session. Different capabilities are isolated from each other. Temporary sidecars are used for consultation. The message flow follows the same rules as ordinary execution.
- Front-end testing covers Python paths, Provider routes, models, new instances, indicator statistics and Capability selection.
- Finally, use the installed official `deepseek-harness-sdk` to run a small real smoke run; if the SDK, bundled runtime or credentials are not available, the missing preconditions are clearly reported and are not replaced by simulation results.

## non-target

- Modify, fork or publish the DeepSeek Harness repository;
- Hermes Provider implementation;
- Archmage manages DeepSeek credentials, accounts, or `DSH_HOME` content;
- DSH session recovery across Archmage restarts;
- Provider automatic selection, failover or load balancing;
- DSH real-time token UI, reasoning process or tool event display;
- Share DSH conversations between multiple capabilities.
