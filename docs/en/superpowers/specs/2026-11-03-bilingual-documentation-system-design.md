# Bilingual Documentation System Design

**Status:** Approved for implementation planning.

## Purpose

Make Simplified Chinese and English first-class, complete documentation languages for Archmage. Every Markdown document currently under `docs/` must have one English version and one Simplified Chinese version with matching paths, structure, examples, constraints, and source references.

The repository landing page remains Chinese-first. English readers receive an explicit entry at the top of the root README.

## Target layout

```text
README.md                 # Chinese project landing page
README.en.md              # English project landing page

docs/
├── en/
│   ├── README.md
│   ├── acceptance/
│   ├── adr/
│   ├── architecture/
│   ├── concepts/
│   ├── product/
│   ├── roadmap/
│   ├── runbooks/
│   └── superpowers/
└── zh-CN/
    ├── README.md
    └── the same relative paths as docs/en/
```

After migration, language-bearing Markdown files must not remain directly under `docs/`. The two language trees are complete mirrors: for every `docs/en/<relative-path>.md`, exactly one `docs/zh-CN/<relative-path>.md` exists, and vice versa.

## Root README behavior

`README.md` becomes the complete Simplified Chinese project landing page and starts with a visible link to `README.en.md`. `README.en.md` preserves the complete English project landing page and starts with a reciprocal link to `README.md`.

Both pages describe the same current product behavior and link to their matching documentation index:

- `README.md` → `docs/zh-CN/README.md`;
- `README.en.md` → `docs/en/README.md`.

The root README pair is outside the `docs/` mirror but follows the same semantic-parity rule.

## Migration and translation rules

### Source selection

For an existing English document, move the original into `docs/en/` and create a faithful Simplified Chinese translation under `docs/zh-CN/`. For an existing Chinese document, move the original into `docs/zh-CN/` and create a faithful English translation under `docs/en/`.

Mixed-language documents must be normalized according to their dominant prose language, then translated completely. Existing document history, status wording, dates, qualifications, and known limitations must be preserved; translation must not turn a design into an implementation claim or a partial verification into a completed acceptance claim.

### Content that remains unchanged

Keep these values verbatim in both languages unless the example explicitly represents human-authored prose:

- shell commands and command output;
- code and configuration snippets;
- JSON field names and enum values;
- database columns, event names, API names, CLI flags, environment variables, file paths, identifiers, and error codes;
- Markdown link destinations, except where migration requires a new relative path;
- product names such as Archmage, Codex, DeepSeek Harness, MCP, and ACP.

Translate headings, explanations, table prose, callouts, human-readable example values, and prose comments where doing so does not change executable meaning.

### Language switcher

Every mirrored document starts with a compact language switcher before the title. The current language is plain text and the peer language is a link:

```markdown
# In an English document
[简体中文](<relative path to zh-CN peer>) | English

# In a Simplified Chinese document
简体中文 | [English](<relative path to en peer>)
```

Relative paths must account for the document's nesting depth.

### Internal links

Links between documentation pages stay within the reader's selected language tree. Because both trees use identical relative paths, most intra-documentation links can remain structurally identical after the initial move.

Links from documentation to repository files must gain the additional parent-directory level introduced by `docs/en/` or `docs/zh-CN/`. Links from source files, the root READMEs, and other repository content to documentation must be updated to target the correct language path.

No compatibility stub is required at each old `docs/<path>.md` location. Repository-owned links must be migrated in the same change so the final tree has no intentional references to removed paths.

## Coverage

Coverage includes every Markdown document currently under `docs/`, including:

- documentation indexes and current user guides;
- product, concept, architecture, roadmap, and runbook documents;
- ADRs and acceptance records;
- historical `superpowers/specs` and `superpowers/plans` documents.

Historical material remains historical. Its translation must preserve original status, chronology, scope, and unchecked checklist state rather than rewriting it as current guidance.

The previously approved cross-provider handoff guide is implemented inside the new mirrored architecture directories:

- `docs/en/architecture/cross-provider-handoff.md`;
- `docs/zh-CN/architecture/cross-provider-handoff.md`.

The requirements in `2026-11-03-cross-provider-handoff-guide-design.md` remain applicable to those two files.

## Documentation indexes

`docs/en/README.md` and `docs/zh-CN/README.md` are equivalent navigation indexes. Each index must:

- explain the documentation layers;
- identify current and historical material clearly;
- link only to pages in its own language tree, except for the language switcher;
- expose the same recommended reading order;
- include the cross-provider handoff guide in the architecture section.

## Delivery strategy

Implement the migration in reviewable batches while keeping the final result complete:

1. **Layout and landing pages** — create both language trees, move existing files to their source-language side, create `README.en.md`, and update root landing-page links.
2. **Current product documentation** — complete both languages for product, concepts, architecture, roadmap, runbooks, CLI/current guides, and documentation indexes.
3. **Historical and governance documentation** — complete both languages for ADRs, acceptance records, and all `superpowers/specs` and `superpowers/plans`.
4. **Cross-provider guide and repository-wide link repair** — add both guide versions, repair all remaining repository references, and run full parity and link verification.

Intermediate commits may contain incomplete translation coverage, but each commit must state its scope accurately. Completion may be claimed only after the final mirror and link checks pass.

## Consistency policy

English and Simplified Chinese documents are equally supported artifacts. A later documentation change that affects behavior, constraints, commands, status, or examples must update both language versions in the same change.

If translation wording conflicts, current implementation and authoritative schemas determine behavior. Neither language is allowed to silently introduce a stricter or looser protocol.

## Verification

Final verification must include:

1. **Mirror parity** — compare relative `.md` paths under `docs/en/` and `docs/zh-CN/`; both sets must be identical.
2. **Language switchers** — ensure every mirrored document links to its peer and that the link resolves.
3. **Relative links** — extract local Markdown links from both trees, confirm every target file exists, and validate anchors for Markdown targets.
4. **Repository references** — search outside `docs/en/` and `docs/zh-CN/` for stale links to removed `docs/<old-path>` locations.
5. **Semantic sampling** — compare headings, code blocks, tables, status statements, dates, warnings, and examples across both languages, with special attention to current runbooks and architecture contracts.
6. **Formatting** — run `git diff --check` and scan for placeholders such as `TODO`, `TBD`, or untranslated temporary markers introduced by the migration.
7. **Scope** — confirm that no Runtime, schema, provider, or UI behavior changed as part of the documentation migration.
