[简体中文](../../../zh-CN/superpowers/specs/2026-09-30-capability-guidance-design.md) | English

# Capability Guidance Design

**Status:** Approved for implementation on `main`.

## Boundary

Each built-in Capability (Designer, Coder, Tester, Reviewer) keeps its fixed Runtime persona and JSON output contract. Its editable `extra_instructions` is the role-specific supplemental prompt. A Capability also owns manually configured Skill and Tool guidance entries, each with a name and purpose. These are instructions for its bound real Agent, not proof that a Skill or Tool is installed, nor a permission or execution allowlist.

## Data and execution

SQLite stores `skills_json` and `tools_json` on `capability_configurations`, defaulting to empty arrays during migration. Saves validate names, purposes, list sizes, duplicate names, and the existing revision. Existing configurations, databases, and historical Run definitions deserialize with empty arrays. New Run definitions freeze the complete Capability configuration; subsequent edits affect only new Runs.

The primary node context includes the frozen role supplement and Skill/Tool guidance. A read-only consultation uses the *target* Capability's frozen supplement and guidance alongside its built-in persona. Both prompts explicitly state that the lists are advisory and do not imply availability or restrict other tools. The Runtime does not attempt to install Skills, enable tools, or change Codex CLI permissions.

## Desktop

Capability management remains a list/detail page, without topology. The selected Capability shows its built-in role description and editable supplemental prompt, Skill entries, Tool entries, and bound Agent. A user can add, edit, and remove manual entries and save all changes atomically. The list summarizes configured Skill and Tool counts. Dirty-state navigation, refresh confirmation, optimistic revision conflicts, and new-Run-only messaging cover the expanded editor. The browser-only preview remains non-authoritative.

## Verification

Tests cover migration/defaults, validation and revision conflict, saved configuration in new versus existing Runs, main and consultation prompt contents, and editor draft serialization/dirty behavior. Run focused Rust and frontend tests, the full Rust workspace suite, frontend build, and formatting/diff checks.
