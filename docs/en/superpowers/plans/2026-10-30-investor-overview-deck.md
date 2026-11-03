[简体中文](../../../zh-CN/superpowers/plans/2026-10-30-investor-overview-deck.md) | English

# Archmage Investor Overview Deck Implementation Plan


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a validated five-slide Chinese investor presentation that explains Archmage with an extremely minimal visual language.

**Architecture:** Build the deck from editable presentation objects using the bundled Artifact Tool. Keep authoring files and rendered previews in a private build directory, then run the presentation finalizer to create a separate deliverable in `output/`.

**Tech Stack:** JavaScript ES modules, `@oai/artifact-tool`, bundled presentation finalizer, PPTX, PNG preview rendering

---

### Task 1: Prepare the authoring workspace

**Files:**
- Create: `.codex-build/archmage-investor-deck/build.mjs`
- Create: `.codex-build/archmage-investor-deck/previews/`
- Create: `output/`

- [ ] **Step 1: Load the bundled workspace runtime**

Run the Codex workspace dependency loader and record the returned Node.js executable, module directory, binary directory, and Python executable for the authoring command.

- [ ] **Step 2: Mark the presentation operation**

Run:

```bash
node container_tools/mark_artifact_operation_started.mjs --operation-kind create --expected-output-count 1 --output-format pptx
```

from the installed presentation skill directory. Expected: the operation marker succeeds without errors.

- [ ] **Step 3: Create private build and output directories**

Create `.codex-build/archmage-investor-deck/previews` and `output`, and link the bundled `node_modules` directory inside the private build directory. Expected: the script can import `@oai/artifact-tool` without installing packages.

### Task 2: Author the five-slide deck

**Files:**
- Create: `.codex-build/archmage-investor-deck/build.mjs`
- Create: `.codex-build/archmage-investor-deck/previews/slide-01.png`
- Create: `.codex-build/archmage-investor-deck/previews/slide-02.png`
- Create: `.codex-build/archmage-investor-deck/previews/slide-03.png`
- Create: `.codex-build/archmage-investor-deck/previews/slide-04.png`
- Create: `.codex-build/archmage-investor-deck/previews/slide-05.png`
- Create: `output/Archmage-投资人概览-5页.pptx`

- [ ] **Step 1: Implement the shared visual system**

Use a 1280×720 canvas, warm off-white background `F7F6F2`, near-black text `171A17`, muted text `666B66`, and sage accent `5F7A68`. Use a Chinese-capable sans-serif font resolved from the bundled runtime. Keep each slide to one dominant message, with no gradients, card grids, stock images, decorative stripes, or ornamental icons.

- [ ] **Step 2: Implement the five-slide narrative**

Create exactly these slides:

1. `Archmage` — subtitle `让本地 AI Agent 围绕同一代码库协作`.
2. `AI 编程进入协作瓶颈` — show four short pain points: manual context transfer, opaque decisions, workspace conflicts, fragmented evidence.
3. `把 Agent 组织成能力系统` — show Capability, Context, Contract, and Workspace as the four product principles.
4. `一次目标，形成研发闭环` — show the editable flow `设计 / 编码 / 测试 / 评审 / 修复`, plus concise investor value labels.
5. `本地优先的协作运行时` — show editable layers for desktop and CLI, Rust runtime services, provider adapter, Git worktree, and local persistence; label multi-provider support as an architectural direction.

- [ ] **Step 3: Add source notes**

Add speaker notes identifying `README.md`, `docs/product/vision.md`, `docs/product/mvp-definition.md`, `docs/architecture/overview.md`, and `docs/concepts/collaboration-model.md` as the product sources used for the deck.

- [ ] **Step 4: Export previews and finalize the PPTX**

Render all five slides to full-size PNG files. Export a private candidate PPTX and invoke the bundled finalizer with `explicitTotalSlideCount: 5`, empty native table and chart requirements, a design font policy, and the expected 16:9 slide size. Expected: the finalizer writes `output/Archmage-投资人概览-5页.pptx`.

### Task 3: Verify and repair

**Files:**
- Modify: `.codex-build/archmage-investor-deck/build.mjs`
- Replace with a new finalized revision if needed: `output/Archmage-投资人概览-5页.pptx`

- [ ] **Step 1: Inspect every slide preview**

Check all five full-size PNGs for clipping, overlaps, unintended wrapping, weak contrast, margins below 48 px, crowded diagrams, or loss of the minimal visual direction.

- [ ] **Step 2: Check the content and package**

Extract presentation text and confirm exactly five slides, correct Chinese wording, correct order, and no placeholders. Confirm the validation receipt reports a successful package, layout, font, and Artifact Tool import check.

- [ ] **Step 3: Repair and regenerate**

If any defect appears, update the authoring script, regenerate every affected preview, finalize to a new output filename, and repeat inspection. Expected: no visible clipping, overlap, placeholder, or unsupported claim remains.
