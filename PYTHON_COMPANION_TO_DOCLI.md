# Python Companion To `docli`

## Purpose

`docli` should stay optimized for fast startup, atomic package operations, and
structured CLI output. A Python companion should handle the expensive work that
benefits from Python's broader ecosystem and where startup cost is acceptable.

The split is simple:

- `docli` is the fast path for inspect, patch, package transactions, and
  machine-readable summaries.
- Python is the slow path for heavyweight validation, semantic checks, final
  QA, and integration with rich analysis libraries.

This keeps the Rust tool small, predictable, and agent-friendly while still
letting us use the best available libraries for deep verification.

## Design Rule

Python should complement `docli`, not replace it.

- `docli` remains the source of truth for reading and mutating `.docx` packages.
- Python should operate after `docli` has produced a candidate document or a
  structured report.
- Python should be invoked for boundary steps:
  preflight, final check, CI verification, release gating, or batch audit.
- Python should avoid editing OOXML directly unless there is a compelling,
  isolated reason. The primary edit engine should remain in `docli`.

## Why This Split Works

Rust and Python have different strengths.

`docli` is better for:

- sub-second cold starts
- atomic single-shot CLI invocations
- package-safe read/modify/write flows
- deterministic JSON envelopes for agents and automation
- shipping as a single binary with minimal environment assumptions

Python is better for:

- richer validation libraries
- heavy parsing and reporting pipelines
- visual QA and document comparison workflows
- data-science-style heuristics over extracted structure
- integration with existing internal scripts and notebooks

The startup penalty is acceptable because Python should only run when we need a
deeper answer than `docli` alone should provide.

## Recommended Responsibilities

### `docli`

- open, inspect, and mutate `.docx` packages
- perform fast structural validation
- emit normalized JSON and YAML envelopes
- expose stable commands for extract, validate, render, diff, and finalize
- provide the low-latency primitive that agents can call repeatedly

### Python Companion

- run heavyweight validation passes over the candidate output
- aggregate multiple checks into a single final report
- perform semantic and policy validation using the wider Python ecosystem
- generate rich HTML or Markdown QA artifacts
- act as a CI gate or "final check" runner

## Best Use Cases For The Python Companion

### 1. Final Validation Gate

Run Python once after all `docli` edits complete.

Examples:

- compare `docli inspect` output against project-specific rules
- verify required sections, heading order, references, and appendix presence
- check table captions, figure numbering, and citation formatting
- confirm that tracked changes or comments are absent before release

### 2. Deep Content Policy Checks

Python is a good place for rules that are expensive or domain-specific.

Examples:

- style-guide enforcement
- glossary consistency
- prohibited language detection
- metadata completeness checks
- cross-document consistency across a batch of files

### 3. Visual and Rendering QA

Python can orchestrate tools and compare their outputs more effectively than a
thin CLI alone.

Examples:

- render DOCX to PDF, then compare page images
- detect layout drift using image diff or OCR
- inspect generated PDFs for missing fonts, blank pages, or broken tables

### 4. Batch Audits And Reporting

Python is a natural fit for repository-wide analysis.

Examples:

- validate hundreds of reports overnight
- build a dashboard of failures by category
- summarize recurring template defects
- export CSV, JSON, or HTML reports for humans

## Recommended Interface Between `docli` And Python

Keep the interface narrow and stable.

### Contract

- Python consumes document paths and `docli` JSON output.
- `docli` does not need to know Python internals.
- Python should treat `docli` as an external command with a stable schema.
- The handoff format should be JSON files or stdout, not ad hoc text parsing.

### Suggested Flow

1. `docli` produces or validates a candidate `.docx`.
2. `docli inspect`, `docli validate`, `docli read`, or `docli diff` emits JSON.
3. Python consumes those envelopes plus the file path.
4. Python runs deeper checks and produces a consolidated report.
5. CI or the agent decides whether the document is acceptable.

### Good Boundary Objects

- source document path
- candidate output path
- `docli inspect` JSON envelope
- `docli validate` JSON envelope
- render artifacts such as PDF or PNG pages
- a final Python report in JSON and Markdown

## Example Workflow

### Fast Iteration Loop

Use only `docli` during editing:

```text
docli inspect report.docx
docli edit.replace report.docx ...
docli validate report.docx
```

This keeps agent loops fast.

### Slow Final Check

Run Python only when the document is ready for review:

```text
docli inspect report.docx --format json > /tmp/report.inspect.json
docli validate report.docx --format json > /tmp/report.validate.json
python tools/docli_companion.py final-check \
  --doc report.docx \
  --inspect /tmp/report.inspect.json \
  --validate /tmp/report.validate.json
```

That gives us a cheap inner loop and an expensive outer gate.

## Python Ecosystem To Leverage

The companion should use Python where the ecosystem is clearly stronger.

### Document and XML Handling

- `lxml` for powerful XML and XPath inspection
- `zipfile` for package inspection when raw archive access is enough
- `python-docx` for high-level read-only convenience checks
- `docx2python` for text and table extraction shortcuts
- `mammoth` for semantic DOCX-to-HTML inspection

### Validation and Data Modeling

- `pydantic` for typed reports and configuration
- `jsonschema` for validating `docli` output contracts
- `schemathesis` or similar tools if API-style schema testing becomes useful

### Data and Heuristic Analysis

- `pandas` for large batch reports
- `rapidfuzz` for approximate heading or phrase matching
- `regex` for advanced style or policy scanning
- `networkx` if document structure graphs become useful

### Rendering and Final QA

- `pypdf` and `pdfplumber` for PDF inspection
- `Pillow` and `opencv-python` for page image comparison
- `pytesseract` if OCR-based checks are needed
- `matplotlib` or `plotly` for QA dashboards

## Suggested Python Companion Modes

A single script can expose a few high-value subcommands.

### `final-check`

Runs the full release gate for one document.

Checks could include:

- `docli` structural validity
- required sections
- policy rules
- citation or appendix conventions
- rendered PDF sanity checks
- report generation

### `batch-audit`

Runs the same logic across many documents and produces a summary table.

### `visual-check`

Focuses on PDF conversion, page rasterization, and visual drift detection.

### `policy-check`

Runs domain-specific rules that should not be embedded into the Rust binary.

## What Should Stay Out Of Python

To avoid architectural drift, the companion should not become the real engine.

Avoid using Python for:

- the default edit path
- repeated small mutations during agent loops
- package commit semantics
- low-level OOXML rewrite logic that `docli` already owns
- core CLI contracts that should stay in one place

If Python becomes responsible for core document mutation, startup cost and
behavioral drift will undo the main point of `docli`.

## Implementation Guidance

If we build this, the first version should stay narrow.

### Phase 1

- create `tools/docli_companion.py`
- shell out to `docli`
- read `docli` JSON envelopes
- emit a combined JSON and Markdown report
- focus on `final-check` for one document

### Phase 2

- add PDF-based visual checks
- add policy packs loaded from YAML
- add batch processing and summary artifacts

### Phase 3

- integrate into CI
- store historical QA reports
- compare document quality across revisions

## Recommended Operating Model

Use this rule of thumb:

- call `docli` many times during authoring and editing
- call Python once per milestone, review, or release boundary

That gives us:

- fast agent feedback loops
- richer final assurance
- better separation of concerns
- freedom to use the Python ecosystem without paying its startup cost on every
  small operation

## Bottom Line

The best complement is not "Python instead of Rust." It is "Python after
Rust."

`docli` should remain the fast transactional core. A Python companion should be
the heavyweight verifier, auditor, and final-check layer that runs less often
but goes much deeper.
