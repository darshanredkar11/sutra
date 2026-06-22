# Changelog

## 0.1.0 (2026-06-22)

Initial release. Sutra is a deterministic, math-first framework for predicting production software failures.

### Added

#### Core Infrastructure
- **Workspace scaffold** — 11 Rust crates with shared dependency management
- **sutra-schema** — Core data types: Engine, Severity, Finding, AnalysisResult, MetricsSummary, AnalyzeRequest, serde support for JSON/YAML/TOML (54 tests)
- **sutra-common** — AnalysisEngine trait, SutraError with typed variants, ComponentHealth, Prometheus-compatible metrics, tracing setup (66 tests)
- **Feature flags** — LLM disabled by default, enabled via `--llm` flag; all engines composable via CLI

#### Phase 1: Complexity Engine
- **sutra-mgtg** — Bridge to mgtg tree-sitter analyzer for McCabe cyclomatic complexity, cognitive complexity, and nesting depth (7 tests)
- Path dependency to external `mgtg` crate at `/Users/darshanredkar/darshan/spantest`

#### Phase 2: Dependency Engine
- **sutra-dependency** — 8 modules: types, extractor, graph, architecture, persist, output, engine, shared
- **Import extraction** — Regex-based import parsing for multiple languages
- **Tarjan SCC algorithm** — Hand-implemented strongly connected components for cycle detection (O(V+E))
- **Fan-in/fan-out analysis** — Per-module dependency fan measurements
- **Architecture rules** — TOML-defined layered architecture validation (105 tests)

#### Phase 3: Process/Change Mining
- **sutra-process** — 8 modules: types, gitwalk, cochange, entropy, jitfeatures, persist, output, engine
- **Git history walker** — git2-based traversal of commit history
- **Hassan entropy** — Code change entropy with sliding window support
- **Co-change graph** — Files modified together across commits
- **14 JIT features** — Revisions, distinct committers, lines added/deleted, entropy, directories, age, owner contribution, minor contributors, etc.
- **SQLite persistence** — Feature storage with rusqlite (53 tests)

#### Phase 4: CLI, Orchestrator & CI
- **sutra-cli** — Clap-based CLI with `analyze`, `health`, `server` subcommands
- **sutra-orchestrator** — Engine coordinator with multi-engine result merging (max risk, union findings, per-field max metrics)
- **Axum HTTP server** — `POST /v1/analyze`, `GET /v1/health`, `GET /v1/status` with CORS (14 tests)
- **sutra-ci** — SARIF 2.1.0 converter with severity mapping (Critical/Error → error, Warning → warning, Info → note)
- **PR comment formatter** — Markdown table sorted by severity with emoji/risk badges (23 tests)

#### Phase 5: ML Engine
- **sutra-ml** — Logistic regression with SGD + L2 regularization
- **Standard scaling** — Z-score normalization with learned means/stds
- **AUC computation** — Trapezoidal rule for ROC curve area
- **Evaluation metrics** — Precision, recall, F1, accuracy from confusion matrix
- **Model persistence** — JSON serialization of weights, bias, means, stds
- **jit_features_to_fvs converter** — Bridges ProcessEngine feature maps to FeatureVector (49 tests)

#### Phase 6: LLM Validation
- **sutra-llm** — Ollama HTTP client using ureq 3.x blocking requests
- **JSON response parser** — Code fence stripping, fallback defaults, confidence clamping
- **In-memory cache** — Deduplicates per-finding LLM validation
- **apply_llm_validation** — Post-processing pipeline: filter + suggested fixes merge
- **Disabled by default** — No network calls unless explicitly enabled (34 tests)

#### Phase 7: HITL Engine
- **sutra-hitl** — Human-in-the-loop feedback collection and analysis
- **FeedbackStore trait** — Abstract over in-memory/SQLite/PostgreSQL backends
- **Feedback adjustment** — Auto-confirm (≥80% correct) / auto-reject (≥60% incorrect) with severity downgrade
- **Precision analysis** — Per-engine reliability reporting (43 tests)

### Engine Registration
- All 5 engines (mgtg, dependency, process, ml, hitl) registered in orchestrator + CLI
- LLM validation as optional post-processing via `--llm` flag

### Test Count
- 448+ tests across all crates, all passing
- Serde roundtrips, edge cases, fixtures, engine isolation

### Configuration
- TOML-based architecture rules for dependency engine
- CLI flags for all options (engine selection, output format, commit, architecture, LLM config)

---

## Future

See [WHITEPAPER.md](./WHITEPAPER.md#11-future-work) for the roadmap.
