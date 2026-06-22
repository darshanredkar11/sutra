# Sutra: A Deterministic, Math-First Framework for Predicting Production Software Failures

**Version 0.1.0 | June 2026**

---

## Abstract

Sutra is a multi-engine static analysis and prediction framework that estimates the probability of production software failures from source code alone. Unlike existing tools that provide isolated lint-style checks, Sutra fuses five analysis engines—deterministic complexity metrics, dependency graph analysis, process/change mining, logistic regression, and LLM-based validation—into a single, composable pipeline. The system produces a unified risk score, actionable findings, SARIF-compatible output, and human-in-the-loop feedback refinement. This white paper describes the architecture, mathematical foundations, engine specifications, and design rationale for each component.

---

## 1. Introduction

### 1.1 Problem Statement

Production software failures cost the global economy an estimated $1.7 trillion annually in lost revenue, recovery costs, and reputational damage [1]. Existing approaches to predicting failures fall into three categories, each with fundamental limitations:

1. **Static analysis linters (ESLint, Pylint, clippy)** — Fast and deterministic, but focused on style and trivial bugs. They do not rank findings by operational risk or model historical failure patterns.

2. **Code quality platforms (SonarQube, CodeClimate)** — Aggregate metrics like cyclomatic complexity and code duplication, but produce a single "quality gate" indicator that correlates poorly with production incidents.

3. **ML-based defect prediction** — Powerful but fragile. Models trained on one team's data fail to generalize. They require labeled defect data that most organizations do not have.

Sutra bridges these approaches by combining deterministic analysis (complexity, dependency, process mining) with optional statistical and LLM enrichment, all coordinated through a unified risk model.

### 1.2 Design Principles

- **Math-first, ML-optional** — The core risk score is a deterministic function of code metrics. ML and LLM components enrich, never replace, the foundational analysis.
- **Composable engines** — Each analysis engine implements the `AnalysisEngine` trait and can be run independently, in orchestrated batches, or as part of a CI/CD pipeline.
- **Deterministic by default** — Given the same repository and commit hash, Sutra produces identical results (ML and LLM engines excluded).
- **Minimal dependencies** — The core stack (mgtg, dependency, process engines) requires only a Rust toolchain, git, and a filesystem. No databases, queues, or external services required.
- **Pluggable persistence** — Feedback and model storage use trait-based abstractions, enabling swap from in-memory to SQLite/Postgres without engine changes.

---

## 2. Architecture

### 2.1 System Overview

```
┌──────────────────────────────────────────────────────────┐
│                        sutra-cli                          │
│              (clap CLI: analyze, health, server)          │
└───────────┬──────────────────────────────────────────────┘
            │
┌───────────▼──────────────────────────────────────────────┐
│                   sutra-orchestrator                      │
│          Engine registry + merge coordinator              │
│          Axum HTTP server (POST /v1/analyze)              │
└────┬──────┬──────┬──────┬──────┬──────┬──────────────────┘
     │      │      │      │      │      │
┌────▼┐ ┌──▼──┐ ┌─▼───┐ ┌─▼──┐ ┌─▼───┐ ┌──────┐
│mgtg │ │dep  │ │proc │ │ ml │ │llm  │ │hitl  │
│engine│ │engine│ │engine│ │engine│ │engine│ │engine│
└─────┘ └─────┘ └─────┘ └────┘ └─────┘ └──────┘
              │                          │
         sutra-common           sutra-schema
         (trait + error)        (types + serde)
```

### 2.2 Crate Structure

| Crate | Lines | Tests | Description |
|-------|-------|-------|-------------|
| `sutra-schema` | ~960 | 54 | Core types: Engine, Severity, Finding, AnalysisResult, serde |
| `sutra-common` | ~140 | 66 | AnalysisEngine trait, SutraError, config, health, tracing |
| `sutra-mgtg` | ~80 | 7 | Bridge to mgtg (tree-sitter-based complexity) |
| `sutra-dependency` | ~1200 | 105 | Import extraction, Tarjan SCC, architecture rules |
| `sutra-process` | ~1400 | 53 | Git walker, Hassan entropy, co-change, 14 JIT features |
| `sutra-ml` | ~600 | 49 | Logistic regression, standard scaling, AUC, persistence |
| `sutra-llm` | ~350 | 34 | Ollama client, JSON parser, cache, validation pipeline |
| `sutra-hitl` | ~450 | 43 | Feedback store, precision analysis, auto-adjust |
| `sutra-orchestrator` | ~260 | 14 | Engine coordinator, merge logic, Axum HTTP server |
| `sutra-ci` | ~200 | 23 | SARIF 2.1.0 converter, PR comment formatter |
| `sutra-cli` | ~210 | 0 | Clap CLI, engine registration |

### 2.3 The AnalysisEngine Trait

Every analysis component implements a single interface:

```rust
pub trait AnalysisEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult>;
}
```

This uniform interface enables the orchestrator to register any engine, run them in parallel or sequence, and merge results without knowing engine internals.

---

## 3. Deterministic Engines

### 3.1 Mgtg Engine (Complexity)

The Mgtg Engine wraps the `mgtg` crate, which uses tree-sitter to parse source files and compute:

- **Cyclomatic complexity** — McCabe's metric: number of linearly independent paths through a function. `M = E - N + 2P` where E = edges, N = nodes, P = connected components.
- **Cognitive complexity** — How hard code is to understand: nesting depth, logical operator breaks, recursion.
- **Nesting depth** — Maximum nesting level of control structures.

Source-level parsing ensures accuracy across languages supported by tree-sitter (JavaScript, TypeScript, Python, Rust, Go, C, C++, Java, and more).

### 3.2 Dependency Engine (Graph Analysis)

The Dependency Engine extracts import/require statements using language-specific regex patterns and builds a directed graph where nodes represent source files and edges represent imports.

**Core algorithms:**

**Tarjan's Strongly Connected Components (SCC):** Detects circular dependencies by finding all SCCs with more than one node. Time complexity: O(V + E).

```
Tarjan(node v):
  v.index = v.lowlink = index; index++
  push v onto stack
  for each edge (v -> w):
    if w not visited: recurse Tarjan(w); v.lowlink = min(v.lowlink, w.lowlink)
    else if w on stack: v.lowlink = min(v.lowlink, w.index)
  if v.lowlink == v.index:
    pop SCC from stack
```

**Fan-in / fan-out analysis:** For each module, count incoming dependencies (fan-in) and outgoing dependencies (fan-out). High fan-in indicates a core utility module. High fan-out indicates a hub or potential architectural violation.

**Architecture rules engine:** Validates imports against a TOML-defined layered architecture (e.g., "presentation" → "application" → "domain" → "infrastructure"). Produces findings for layer violations.

### 3.3 Process Engine (Change Mining)

The Process Engine analyzes git history to model how code evolves. It computes 14 features per file:

**A. Code Change Entropy (Hassan, 2009)**

For a file `f` across `n` commits, entropy measures the distribution of changes:

`H(f) = -Σ p_i · log₂(p_i)`

where `p_i = lines_changed_in_commit_i / total_lines_changed`

A high entropy file has changes spread evenly across many commits (indicating systemic churn). Low entropy means changes concentrate in few commits (targeted edits). A sliding window variant (`compute_entropy_window`) restricts to the most recent N days.

**B. JIT Defect Prediction Features (14 dimensions)**

Based on the Just-In-Time (JIT) defect prediction literature [2, 3]:

| # | Feature | Description |
|---|---------|-------------|
| 1 | `revisions` | Number of times file was modified |
| 2 | `distinct_committers` | Unique authors touching the file |
| 3 | `lines_added` | Total lines added |
| 4 | `lines_deleted` | Total lines deleted |
| 5 | `total_lines_changed` | Sum of added + deleted |
| 6 | `entropy` | Hassan entropy over git history |
| 7 | `num_directories` | Number of distinct directories the file touches |
| 8 | `avg_files_per_commit` | Average file count in commits touching this file |
| 9 | `age_days` | Days since first commit touching this file |
| 10 | `weighted_age_days` | Age weighted by change frequency |
| 11 | `recent_commits` | Commit count in recent window (90 days) |
| 12 | `bug_fix_commits` | Number of commits with bug-fix keywords |
| 13 | `owner_contribution` | Ratio of changes by the primary author |
| 14 | `minor_contributors` | Number of authors with <5% contribution |

These features are stored per-file via SQLite persistence and serve as the primary input to the ML engine.

**C. Co-change Graph**

Files that are frequently modified together form a co-change graph. The engine computes a co-change matrix: for each pair of files `(A, B)`, the count of commits where both were modified. This surfaces hidden coupling not visible in import statements.

---

## 4. ML Engine

### 4.1 Model Architecture

The ML engine implements logistic regression from scratch:

`sigmoid(z) = 1 / (1 + e^(-z))`

`P(y=1 | x) = sigmoid(w^T · x_scaled + b)`

where:
- `w` = weight vector (14 dimensions, one per JIT feature)
- `b` = bias term
- `x_scaled` = standardized feature vector: `x_scaled[i] = (x[i] - μ[i]) / σ[i]`
- `μ[i]` = mean of feature i across training data
- `σ[i]` = standard deviation (floored at 1.0 to prevent division by zero)

### 4.2 Training: SGD with L2 Regularization

Training uses stochastic gradient descent with learning rate decay:

```
for epoch in 0..epochs:
    lr = base_lr / (1 + 0.001 * epoch)
    for each example (x, y):
        y_hat = sigmoid(w^T · x_scaled + b)
        error = y_hat - y
        w[i] -= lr * (error * x_scaled[i] + l2_lambda * w[i])  // L2 penalty
        b -= lr * error
```

The loss function is binary cross-entropy:

`L = -[y · log(y_hat) + (1 - y) · log(1 - y_hat)]`

Clamped at ±1e10 to prevent numerical overflow.

### 4.3 Evaluation

The evaluation module computes a full confusion matrix and derived metrics:

| Metric | Formula |
|--------|---------|
| Accuracy | `(TP + TN) / (TP + FP + TN + FN)` |
| Precision | `TP / (TP + FP)` |
| Recall | `TP / (TP + FN)` |
| F1 Score | `2 · P · R / (P + R)` |

**AUC (Area Under the ROC Curve):** Computed via the trapezoidal rule. Predictions are sorted by probability descending, then the ROC curve is constructed by sweeping the threshold from 0 to 1:

`AUC = Σ (x_{i+1} - x_i) · (y_i + y_{i+1}) / 2`

where `x` = false positive rate, `y` = true positive rate.

### 4.4 Persistence

Models serialize to JSON with `serde`:

```json
{
  "weights": [0.12, -0.05, ...],
  "bias": 0.01,
  "means": [5.3, 2.1, ...],
  "stds": [3.7, 1.9, ...],
  "feature_names": ["revisions", "distinct_committers", ...]
}
```

---

## 5. LLM Validation Engine

### 5.1 Architecture

The LLM engine validates findings by sending each to an Ollama-compatible model:

```
Finding → Prompt Builder → HTTP POST /api/generate → Response Parser → ValidationResult
                          ↓
                      In-Memory Cache
```

### 5.2 Prompt Design

Each finding is wrapped in a structured prompt that instructs the LLM to validate whether the finding represents a genuine production risk:

```
You are a code review expert. Validate the following static analysis finding.
Respond with JSON only — no markdown, no explanation outside the JSON.

File: {path}, Line {line}
Finding: {message}
Severity: {severity}

{
  "is_valid": true|false,
  "confidence": 0.0..1.0,
  "explanation": "...",
  "suggested_fix": "..." or null
}
```

### 5.3 Response Parsing

The parser handles real-world LLM output variations:
- Strips markdown code fences (```json, ```)
- Falls back to default values on parse failure (`is_valid: true`, `confidence: 0.5`)
- Filters empty or "null" suggested fixes
- Clamps confidence to [0, 1]

### 5.4 Pipeline Integration

The `apply_llm_validation()` function integrates LLM validation as a post-processing step after all deterministic engines complete:

```
All Engines → Combined Findings → LLM Validation → Validated Findings
                                                        ↓
                                               Filter by confidence,
                                               apply suggested fixes
```

---

## 6. HITL (Human-in-the-Loop) Engine

### 6.1 Purpose

The HITL engine closes the feedback loop. When developers review findings and mark them as correct or incorrect, the engine learns which engines produce reliable results and adjusts future analyses accordingly.

### 6.2 Feedback Storage

Feedback entries are stored via the `FeedbackStore` trait:

```rust
pub trait FeedbackStore: Send + Sync {
    fn store(&mut self, entry: FeedbackEntry) -> SutraResult<()>;
    fn get_by_finding_id(&self, finding_id: &str) -> SutraResult<Vec<FeedbackEntry>>;
    fn get_by_engine(&self, engine: &Engine) -> SutraResult<Vec<FeedbackEntry>>;
    fn metrics(&self) -> SutraResult<FeedbackMetrics>;
    fn clear(&mut self) -> SutraResult<()>;
    fn len(&self) -> usize;
}
```

The default implementation is `InMemoryFeedbackStore`. Future implementations can back to SQLite or PostgreSQL by implementing this trait.

### 6.3 Feedback Adjustment

Given a finding with sufficient feedback entries (default: ≥3), the engine computes:

`confirm_ratio = correct / (correct + incorrect)`
`reject_ratio = incorrect / (correct + incorrect)`

Decision rules:
- If `confirm_ratio ≥ 0.8`: auto-validates the finding (`validated = true`)
- If `reject_ratio ≥ 0.6`: downgrades severity (Critical→Warning, Error→Info)
- Mixed feedback or insufficient count: no adjustment

### 6.4 Feedback Metrics

The engine reports per-engine precision:

`precision(engine) = correct_feedback / (correct + incorrect)`

These metrics are surfaced as findings (`HITL-001` through `HITL-003`) and recommendations during analysis.

---

## 7. Orchestration

### 7.1 The Orchestrator

The `Orchestrator` manages the lifecycle of multiple engines:

**Registration:** Engines are registered by `Engine` enum variant:

```rust
orchestrator.register(Engine::Mgtg, Box::new(MgtgEngine::new()));
orchestrator.register(Engine::Dependency, Box::new(DependencyEngine::new()));
```

**Analysis:** The `analyze()` method runs requested engines and merges results:

1. Select engines from request (or default set: mgtg, dependency, process)
2. For each engine, call `analyze()` and collect results
3. **Risk:** `total_risk = max(total_risk, result.overall_risk)`
4. **Time:** `total_time += result.processing_time_ms`
5. **Findings:** union of all findings (deduplication by ID)
6. **Recommendations:** union of all recommendations
7. **Metrics:** per-field max across engines
8. **Blocked:** true if any engine blocks

### 7.2 HTTP API

The orchestrator surfaces its analysis capability via an Axum HTTP server:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/analyze` | POST | Run analysis with JSON request body |
| `/v1/health` | GET | Component health check |
| `/v1/status` | GET | Server and engine status |

### 7.3 CLI Interface

The `sutra` CLI provides three subcommands:

```bash
sutra analyze <path>         # Run analysis on a repository
sutra health <path>          # Quick health score
sutra server --port 8080     # Start HTTP API server
```

Output formats: `pretty` (terminal), `json` (machine), `sarif` (CI/CD).

---

## 8. CI/CD Integration

### 8.1 SARIF Output

Sutra generates SARIF 2.1.0 (Static Analysis Results Interchange Format), enabling integration with GitHub Advanced Security, Azure DevOps, and other SARIF-compatible tools.

**Severity mapping:**

| Sutra Severity | SARIF Level |
|----------------|-------------|
| Info | `note` |
| Warning | `warning` |
| Error | `error` |
| Critical | `error` |

### 8.2 PR Comments

The CI output includes a markdown-formatted PR comment with:
- Risk badge (HIGH/MEDIUM/LOW)
- Findings table sorted by severity (highest first) with emoji indicators
- Metric summary
- Recommendations with confidence percentages

---

## 9. Feature Flags and Extensibility

### 9.1 Engine Feature Flags

| Feature | Cargo Flag | Default |
|---------|-----------|---------|
| LLM validation | `--llm` CLI flag | Disabled |
| ml Engine | `--engine ml` | Included |
| hitl Engine | `--engine hitl` | Included |

### 9.2 Pluggable Components

The trait-based architecture enables swapping implementations:

| Trait | Default Implementation | Alternative |
|-------|----------------------|-------------|
| `FeedbackStore` | InMemoryFeedbackStore | SQLiteFeedbackStore, PostgresFeedbackStore |
| `AnalysisEngine` | Per-crate engine impl | Custom domain-specific engines |

---

## 10. Test Strategy

Sutra achieves high confidence through exhaustive testing at every level:

| Crate | Tests | Focus Areas |
|-------|-------|-------------|
| sutra-schema | 68 | Serde roundtrips, validation, edge cases, proptest, NaN/Infinity |
| sutra-common | 66 | Error types, config parsing, health, metrics |
| sutra-dependency | 121 | Import extraction, Tarjan SCC, architecture rules, graph stress |
| sutra-process | 66 | Entropy, co-change, JIT features, SQLite, engine, sliding window |
| sutra-ml | 70 | Sigmoid, scaling, training, evaluation, AUC, proptest, extreme values |
| sutra-llm | 45 | Ollama client, JSON parsing, caching, response edge cases |
| sutra-hitl | 62 | Feedback store, analysis, auto-adjust, 10K entry stress |
| sutra-orchestrator | 22 | Engine registration, merge logic, HTTP API, panic recovery |
| sutra-ci | 32 | SARIF generation, PR comments, 1K findings stress |

Key test patterns:
- **Property-based testing**: Proptest generates random AnalysisResult instances, severity ranks, feature vectors, and sigmoid inputs to verify algebraic properties (symmetry, bounds, roundtrips)
- **Stress testing**: 10K-entry feedback store, 1K-finding SARIF output, 100-node cyclic graphs
- **Panic safety**: Engine panics are caught via `catch_unwind` and converted to error findings
- **NaN/Infinity guards**: Every numeric path validates against non-finite floats
- **Unicode safety**: All string fields tested with emoji, CJK, RTL text, 10K-char notes
- **Integration testing**: Real git repo analysis, SQLite persistence roundtrips, HTTP API endpoint tests

---

## 11. Future Work

### 11.1 Short Term

- **GitHub Action**: Package as `.github/actions/sutra/` with Docker image for one-click CI setup
- **End-to-end integration test**: Run all 5 engines against a real git repository

### 11.2 Medium Term

- **SQLite feedback store**: Persistent feedback storage with schema migrations
- **ML model zoo**: Pre-trained logistic regression models for common languages and frameworks
- **PostgreSQL backend**: For multi-repository deployments at scale

### 11.3 Long Term

- **Kafka integration**: Stream findings for real-time dashboards
- **Valkey/Redis cache**: Distributed result caching
- **ObservaLog**: Runtime incident response system (separate track)
- **Multi-model ML**: Support for XGBoost and neural networks via ONNX
- **Cross-repo learning**: Transfer learning across repositories in the same organization

---

## 12. References

[1] Consortium for Information & Software Quality (CISQ), "The Cost of Poor Software Quality in the US: A 2022 Report," 2022.

[2] A. E. Hassan, "Predicting faults using the complexity of code changes," in *Proceedings of the 31st International Conference on Software Engineering*, 2009, pp. 78–88.

[3] Y. Kamei, E. Shihab, B. Adams, A. E. Hassan, A. Mockus, A. Sinha, and N. Ubayashi, "A large-scale empirical study of just-in-time quality assurance," *IEEE Transactions on Software Engineering*, vol. 39, no. 6, pp. 757–773, 2013.

[4] R. E. Tarjan, "Depth-first search and linear graph algorithms," *SIAM Journal on Computing*, vol. 1, no. 2, pp. 146–160, 1972.

[5] T. J. McCabe, "A complexity measure," *IEEE Transactions on Software Engineering*, vol. SE-2, no. 4, pp. 308–320, 1976.

[6] OASIS, "SARIF Version 2.1.0," OASIS Standard, 2020. [Online]. Available: https://docs.oasis-open.org/sarif/sarif/v2.1.0/

---

*Sutra — Because production failures are predictable.*
