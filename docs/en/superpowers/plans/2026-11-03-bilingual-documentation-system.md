# Bilingual Documentation System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every Archmage document into mirrored English and Simplified Chinese trees, make the repository landing page Chinese-first with an English peer, and verify complete bilingual coverage and valid links.

**Architecture:** `docs/en/` and `docs/zh-CN/` contain identical Markdown paths and same-language internal links. Existing documents are mirrored first, then each peer is translated without changing code, commands, status, or historical evidence. Root `README.md` and `README.en.md` form a separate bilingual pair.

**Tech Stack:** Markdown, Git, shell utilities (`rg`, `comm`, `find`), repository source code as the authority for current behavior.

---

### Task 1: Establish the mirrored directory layout and root README pair

**Files:**
- Move: every current `docs/**/*.md` file to `docs/en/<same-relative-path>`
- Create: every matching `docs/zh-CN/<same-relative-path>`
- Create: `README.en.md`
- Modify: `README.md`

- [ ] **Step 1: Capture the pre-migration inventory**

```bash
rg --files docs -g '*.md' | sort > /tmp/archmage-docs-before.txt
wc -l /tmp/archmage-docs-before.txt
```

Expected: `72` Markdown files after this plan is added.

- [ ] **Step 2: Move the current tree into `docs/en/` and mirror it**

```bash
mkdir -p docs/en docs/zh-CN
while IFS= read -r path; do
  relative=${path#docs/}
  mkdir -p "docs/en/$(dirname "$relative")"
  git mv "$path" "docs/en/$relative"
done < /tmp/archmage-docs-before.txt
cp -R docs/en/. docs/zh-CN/
```

Expected: both commands below produce identical path lists, and no Markdown remains directly under `docs/`.

```bash
(cd docs/en && rg --files -g '*.md' | sort) > /tmp/archmage-en.txt
(cd docs/zh-CN && rg --files -g '*.md' | sort) > /tmp/archmage-zh.txt
comm -3 /tmp/archmage-en.txt /tmp/archmage-zh.txt
find docs -mindepth 1 -maxdepth 1 -type f -name '*.md'
```

- [ ] **Step 3: Split and translate the root README**

Copy the current English project page to `README.en.md`. Add `[简体中文](README.md) | English` above its title and link it to `docs/en/README.md`. Translate the complete root `README.md` into Simplified Chinese, add `简体中文 | [English](README.en.md)` above its title, and link it to `docs/zh-CN/README.md`. Preserve all commands and current product qualifications.

- [ ] **Step 4: Verify and commit the layout**

```bash
test -f README.en.md
test -f docs/en/README.md
test -f docs/zh-CN/README.md
git diff --check
git add README.md README.en.md docs/en docs/zh-CN
git commit -m "docs: establish bilingual documentation layout"
```

### Task 2: Translate indexes and current user-facing documentation

**Files:**
- Modify: `docs/{en,zh-CN}/README.md`
- Modify: `docs/{en,zh-CN}/cli-reference.md`
- Modify: `docs/{en,zh-CN}/guide.md`
- Modify: `docs/{en,zh-CN}/guide-online-doctor.md`
- Modify: `docs/{en,zh-CN}/product/*.md`
- Modify: `docs/{en,zh-CN}/concepts/*.md`
- Modify: `docs/{en,zh-CN}/architecture/*.md`
- Modify: `docs/{en,zh-CN}/roadmap/**/*.md`
- Modify: `docs/{en,zh-CN}/runbooks/*.md`

- [ ] **Step 1: Preserve source-language copies and translate their peers**

Preserve existing Chinese prose in `zh-CN` for the CLI/current guides and desktop runbook. Preserve existing English prose in `en` for product, concepts, architecture, roadmap, and the remaining runbooks. Translate every peer completely; do not summarize.

- [ ] **Step 2: Add a language switcher before every title**

Use the same pattern at the correct nesting depth. For example:

```markdown
# docs/en/architecture/overview.md
[简体中文](../../zh-CN/architecture/overview.md) | English

# docs/zh-CN/architecture/overview.md
简体中文 | [English](../../en/architecture/overview.md)
```

- [ ] **Step 3: Repair links for the added language directory level**

Keep documentation links inside the active language tree. Add one parent level to links that target repository files outside `docs/`. Preserve commands, flags, environment variables, JSON fields, event names, file paths, dates, and verification results verbatim.

- [ ] **Step 4: Verify and commit current documentation**

```bash
for root in docs/en docs/zh-CN; do
  rg --files "$root" -g '*.md' -g '!superpowers/**' -g '!adr/**' -g '!acceptance/**' | while IFS= read -r file; do
    head -n 1 "$file"
  done
done
git diff --check
git add docs/en docs/zh-CN
git commit -m "docs: translate current product documentation"
```

Expected: every printed first line is a language switcher and formatting succeeds.

### Task 3: Translate ADRs and acceptance records

**Files:**
- Modify: `docs/{en,zh-CN}/adr/0001-deliver-vertical-slices.md`
- Modify: `docs/{en,zh-CN}/adr/0002-separate-provider-and-capability-agent.md`
- Modify: `docs/{en,zh-CN}/adr/0003-use-explicit-context-handoffs.md`
- Modify: `docs/{en,zh-CN}/adr/0004-use-per-run-git-worktrees.md`
- Modify: `docs/{en,zh-CN}/adr/0005-use-real-codex-only-in-product-flow.md`
- Modify: `docs/{en,zh-CN}/acceptance/s0/batch-a-progress.md`
- Modify: `docs/{en,zh-CN}/acceptance/s0/batch-b-progress.md`
- Modify: `docs/{en,zh-CN}/acceptance/s0/batch-c-progress.md`

- [ ] **Step 1: Translate all ADR pairs**

Preserve decision status, context, consequences, supersession relationships, and provider names. Do not modernize earlier decisions while translating them.

- [ ] **Step 2: Translate all acceptance pairs**

Preserve dates, commands, pass/fail wording, limitations, skipped checks, and partial completion. Translation must not upgrade evidence claims.

- [ ] **Step 3: Add peer switchers, verify counts, and commit**

```bash
test "$(rg --files docs/en/adr -g '*.md' | wc -l | tr -d ' ')" = 5
test "$(rg --files docs/zh-CN/adr -g '*.md' | wc -l | tr -d ' ')" = 5
test "$(rg --files docs/en/acceptance -g '*.md' | wc -l | tr -d ' ')" = 3
test "$(rg --files docs/zh-CN/acceptance -g '*.md' | wc -l | tr -d ' ')" = 3
git diff --check
git add docs/en/adr docs/zh-CN/adr docs/en/acceptance docs/zh-CN/acceptance
git commit -m "docs: translate decisions and acceptance records"
```

### Task 4: Translate all design specifications

**Files:**
- Modify: every mirrored `docs/{en,zh-CN}/superpowers/specs/*.md`

- [ ] **Step 1: Translate all September specification pairs**

Cover every `2026-09-*.md` pair. Preserve proposed/approved status, boundaries, schemas, UI labels, and explicitly deferred work.

- [ ] **Step 2: Translate all October specification pairs**

Cover every `2026-10-*.md` pair. Preserve Team compatibility, legacy/flow semantics, provider constraints, UI details, and the difference between design approval and implementation evidence.

- [ ] **Step 3: Translate all November specification pairs**

Translate the cross-provider handoff design and bilingual documentation design completely. Keep paths, JSON fields, commands, and verification rules identical.

- [ ] **Step 4: Add depth-correct switchers and verify parity**

Each English spec links to the same filename below `../../../zh-CN/superpowers/specs/`; each Chinese spec links to the same filename below `../../../en/superpowers/specs/`. For example, the English `2026-09-16-real-codex-vertical-slice-design.md` links to `../../../zh-CN/superpowers/specs/2026-09-16-real-codex-vertical-slice-design.md`.

```bash
test "$(rg --files docs/en/superpowers/specs -g '*.md' | wc -l | tr -d ' ')" = "$(rg --files docs/zh-CN/superpowers/specs -g '*.md' | wc -l | tr -d ' ')"
git diff --check
git add docs/en/superpowers/specs docs/zh-CN/superpowers/specs
git commit -m "docs: translate design specifications"
```

### Task 5: Translate all implementation plans

**Files:**
- Modify: every mirrored `docs/{en,zh-CN}/superpowers/plans/*.md`

- [ ] **Step 1: Translate all September plan pairs**

Preserve checkbox state, commands, expected results, commit messages, file paths, code blocks, dates, and recorded progress. Do not mark unchecked work complete.

- [ ] **Step 2: Translate October 14 and 15 Team plan pairs**

Preserve compatibility constraints, tests, expected failures, and progress notes.

- [ ] **Step 3: Translate October 28 through 31 plan pairs**

Preserve UI selectors, CSS class names, SDK/runtime package names, provider routes, test fixtures, acceptance evidence, and completion qualifications.

- [ ] **Step 4: Translate this November migration plan**

Keep the English plan intact apart from its switcher and migrated paths. Translate every checkbox, command, expected result, commit message, and explanation into the Chinese peer.

- [ ] **Step 5: Add peer switchers, verify parity, and commit**

```bash
test "$(rg --files docs/en/superpowers/plans -g '*.md' | wc -l | tr -d ' ')" = "$(rg --files docs/zh-CN/superpowers/plans -g '*.md' | wc -l | tr -d ' ')"
git diff --check
git add docs/en/superpowers/plans docs/zh-CN/superpowers/plans
git commit -m "docs: translate implementation plans"
```

### Task 6: Add the bilingual cross-provider handoff guide

**Files:**
- Create: `docs/en/architecture/cross-provider-handoff.md`
- Create: `docs/zh-CN/architecture/cross-provider-handoff.md`
- Modify: `docs/{en,zh-CN}/README.md`
- Modify: `docs/{en,zh-CN}/architecture/context-and-contracts.md`
- Modify: `docs/{en,zh-CN}/superpowers/specs/2026-10-14-message-flow-teams-design.md`
- Modify: `docs/{en,zh-CN}/runbooks/desktop-s0.md`

- [ ] **Step 1: Write the English guide from current code**

Document the Archmage orchestration boundary, JSON/JSON Schema common language, delivery envelope, persisted FlowDelivery, receiver context, Coder/Codex → Reviewer/DSH example, provider adapter differences, conversation/workspace boundaries, validation failures, compatibility, and code sources.

- [ ] **Step 2: Write the semantically equivalent Chinese guide**

Use identical headings, JSON field names, examples, constraints, and source links. Translate prose and human-readable values without changing normative meaning.

- [ ] **Step 3: Apply approved consistency corrections**

Mark Message-Flow Teams as implemented and point to its implementation plan for verification history. Replace the desktop guide's “every delivery executes real Codex” wording with target-Capability provider-neutral wording. Add same-language links to the new guide.

- [ ] **Step 4: Verify and commit the guide pair**

```bash
git diff --check
git add docs/en docs/zh-CN
git commit -m "docs: add bilingual cross-provider handoff guide"
```

### Task 7: Repair repository-wide documentation links

**Files:**
- Modify: any tracked Markdown file that links to a removed `docs/<old-path>.md`
- Modify: `README.md`, `README.en.md`, and both documentation indexes as required

- [ ] **Step 1: Find stale old-layout links**

```bash
rg -n '\]\(docs/(?!en/|zh-CN/)|\]\((?:\.\./)*docs/(?!en/|zh-CN/)' . -g '*.md' --pcre2
```

- [ ] **Step 2: Route each stale link by prose language**

Chinese prose links to `docs/zh-CN/...`; English prose links to `docs/en/...`. Do not create compatibility stubs at old paths.

- [ ] **Step 3: Verify no stale links remain and commit**

Run the Step 1 search again; expected output is empty.

```bash
git diff --check
git add -u README.md README.en.md docs
git commit -m "docs: repair bilingual documentation links"
```

### Task 8: Run final parity, switcher, link, and scope verification

**Files:**
- Modify: only documentation files requiring corrections discovered by verification

- [ ] **Step 1: Prove mirror parity**

```bash
(cd docs/en && rg --files -g '*.md' | sort) > /tmp/archmage-en-final.txt
(cd docs/zh-CN && rg --files -g '*.md' | sort) > /tmp/archmage-zh-final.txt
comm -3 /tmp/archmage-en-final.txt /tmp/archmage-zh-final.txt
```

Expected: no output.

- [ ] **Step 2: Prove every language switcher resolves**

For every relative path in the parity list, calculate its nesting depth, confirm each first line contains the required peer link, and resolve that link to an existing file. Correct every missing or malformed switcher.

- [ ] **Step 3: Validate all local Markdown targets and anchors**

Extract every non-HTTP, non-mailto Markdown link. Resolve it relative to its source file, confirm the target exists, and for Markdown fragments confirm the normalized heading anchor exists. Correct every broken link.

- [ ] **Step 4: Check semantic and structural parity**

Compare each pair's heading count, fenced-code count, table structure, dates, status markers, commands, paths, JSON fields, and checkbox counts. Manually review current runbooks, architecture contracts, and cross-provider guides for equivalent claims.

- [ ] **Step 5: Run formatting and scope checks**

```bash
git diff --check origin/main...HEAD
git status --short
git diff --name-only 6554406...HEAD | rg -v '^(README\.md|README\.en\.md|docs/)'
```

Expected: formatting succeeds, the worktree is clean after commits, and this migration introduces no Runtime, schema, provider, or UI source changes.

- [ ] **Step 6: Commit verification corrections if needed**

```bash
git add README.md README.en.md docs/en docs/zh-CN
git commit -m "docs: verify bilingual documentation mirror"
```

Do not create an empty commit when verification requires no correction.
