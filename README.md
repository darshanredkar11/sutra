# Sutra

**Predict production software failures before they happen.**

[![Rust](https://img.shields.io/badge/rust-1.86%2B-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](https://www.apache.org/licenses/LICENSE-2.0)
[![SARIF](https://img.shields.io/badge/SARIF-2.1.0-orange)](https://docs.oasis-open.org/sarif/sarif/v2.1.0/)

Sutra is a deterministic, math-first framework that estimates production failure risk from source code and proposes concrete fixes. It fuses **7 analysis engines** (complexity, dependency, runtime survivability, process mining, ML, LLM, human feedback) + **5 repair engines** (refactoring, coupling, performance, testing, debt prioritization) into a single composable pipeline. **18K+ lines of Rust, 693 tests, 13 crates, zero external runtime deps. Parallel execution, OpenAPI docs, E2E integration tests, CI security auditing.**

---

## Quick Start

```bash
# Analyze a repository with all 7 engines
cargo run -- analyze /path/to/repo

# Start the HTTP API server
cargo run -- server --port 8080

# Health check
cargo run -- health /path/to/repo

# Analyze with LLM validation (requires Ollama)
cargo run -- analyze /path/to/repo --llm

# Output SARIF for CI integration
cargo run -- analyze /path/to/repo --format sarif --output results.sarif
```

---

## How It Works

```
┌──────────────────────────────────────────────────────┐
│                     sutra analyze                      │
├──────────────────────────────────────────────────────┤
│  mgtg        → complexity metrics (McCabe, Cognitive)  │
│  dependency  → import graph + Tarjan SCC cycles        │
│  rse         → runtime survivability (queueing, mem)   │
│  process     → git history + Hassan entropy + 14 JIT   │
│  ml          → logistic regression (SGD+L2, 14 feats)  │
│  llm         → Ollama-based finding validation (opt)   │
│  hitl        → human feedback precision tracking       │
├──────────────────────────────────────────────────────┤
│  Output: unified risk score + findings + SARIF          │
└──────────────────────────────────────────────────────┘
```

Each engine implements the `AnalysisEngine` trait:

```rust
let mut orchestrator = Orchestrator::new();
orchestrator.register(Engine::Mgtg, Box::new(MgtgEngine::new()));
orchestrator.register(Engine::Dependency, Box::new(DependencyEngine::new()));
orchestrator.register(Engine::Rse, Box::new(RseEngine::new()));
orchestrator.register(Engine::Process, Box::new(ProcessEngine::new()));
orchestrator.register(Engine::Ml, Box::new(MlEngine::new()));
orchestrator.register(Engine::Llm, Box::new(LlmEngine::new()));
orchestrator.register(Engine::Hitl, Box::new(HitlEngine::new()));

let result = orchestrator.analyze(&request)?;
```

---

## Engines

| Engine | Crate | Tests | What It Does | Deterministic |
|--------|-------|-------|-------------|:---:|
| **mgtg** | `sutra-mgtg` | 7 | Cyclomatic/cognitive complexity via regex patterns | Yes |
| **dependency** | `sutra-dependency` | 121 | Import graph, Tarjan SCC cycles, architecture layer rules, fan-in/out | Yes |
| **rse** | `sutra-rse` | 123 | Runtime survivability: M/M/1 queueing, CPU/memory/GC/thread/latency risk | Yes |
| **process** | `sutra-process` | 66 | Git history mining, Hassan entropy, co-change matrix, 14 JIT features | Yes |
| **ml** | `sutra-ml` | 70 | Logistic regression (SGD+L2), AUC evaluation, model persistence | No\* |
| **llm** | `sutra-llm` | 45 | Finding validation via Ollama (optional, disabled by default) | No |
| **hitl** | `sutra-hitl` | 62 | Human feedback, per-engine precision tracking, auto-adjust findings | Yes |
| **orchestrator** | `sutra-orchestrator` | 33 | Engine registration, risk fusion (max), panic safety, HTTP API, parallel rayon, 11 E2E tests | Yes |
| **schema** | `sutra-schema` | 68 | Core types, serde JSON/YAML/TOML, proptest fuzzing | — |
| **common** | `sutra-common` | 66 | Shared traits, errors, config, health, metrics | — |
| **ci** | `sutra-ci` | 32 | SARIF 2.1.0 output, markdown PR comments | — |

\* ML is deterministic for a given trained model; training itself is not deterministic.

### Repair Engines (Actionable Specifications)

Beyond finding problems, Sutra proposes structured solutions. The **5 repair engines** generate detailed specifications with quantified impact:

| Engine | Crate | Purpose | Outputs |
|--------|-------|---------|---------|
| **refactoring** | `sutra-repair-refactoring` | Propose class/method extractions | `RefactoringSpec`: before/after complexity, effort (hrs), bug prevention ROI |
| **coupling** | `sutra-repair-coupling` | Propose architectural decoupling | `CouplingSpec`: throughput/latency gain, migration phases, RPS increase |
| **performance** | `sutra-repair-performance` | Fix runtime bottlenecks | `PerformanceSpec`: latency reduction %, optimization strategies ranked by ROI |
| **testing_gap** | `sutra-repair-testing-gap` | Plan test coverage improvements | `TestingGapSpec`: coverage gain %, test patterns, effort estimates |
| **debt_roi** | `sutra-repair-debt-roi` | Prioritize all findings by ROI | `DebtRoiSpec`: ranked payoff timeline (months), annual savings per fix |

Each spec is machine-readable JSON with:
- ✅ **Quantified impact**: before/after numbers, confidence scores, edge cases
- ✅ **Effort breakdown**: design, implementation, testing, buffer hours
- ✅ **ROI analysis**: incident prevention value, payoff period
- ✅ **Validation strategy**: how to verify the fix works
- ✅ **Zero hallucination**: all data computed, not invented

**Example:** Coupling engine on observalog's triage subsystem:
```json
{
  "id": "COUP-001",
  "type": "async_message_queue",
  "impact": {
    "throughput": {"before_rps": 8, "after_rps": 45, "improvement_percent": 462.5},
    "latency_p95": {"before_ms": 500, "after_ms": 180},
    "coupling_score": {"before": 0.82, "after": 0.15}
  },
  "effort_hours": 40,
  "roi_months": 0.69,  // Pays for itself in < 1 month
  "confidence": 0.88
}
```

See [REPAIR_ENGINES.md](REPAIR_ENGINES.md) for detailed specification examples.

---

## CLI Reference

```bash
sutra --help
sutra --version

# Run analysis
sutra analyze <path>
sutra analyze <path> --engine mgtg         # single engine
sutra analyze <path> --engine all          # all 7 engines (default)
sutra analyze <path> --format json         # JSON output
sutra analyze <path> --format sarif        # SARIF 2.1.0 output
sutra analyze <path> --output results.sarif
sutra analyze <path> --commit abc123       # specific commit
sutra analyze <path> --arch arch.toml      # dependency architecture rules
sutra analyze <path> --llm                 # enable LLM validation
sutra analyze <path> --llm-model llama3.2
sutra analyze <path> --ollama-url http://localhost:11434

# Health check
sutra health <path>

# HTTP server
sutra server --port 8080
```

### API Endpoints (server mode)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `POST /v1/analyze` | POST | Run analysis with JSON request body |
| `POST /v1/demo` | POST | Clone any public GitHub repo and run all 7 engines |
| `GET /v1/health` | GET | Component health check |
| `GET /v1/status` | GET | Server and engine status |
| `GET /v1/report` | GET | HTML report page with interactive analysis form |
| `GET /v1/openapi.json` | GET | OpenAPI 3.1 specification |
| `GET /v1/docs` | GET | Interactive Swagger UI docs |

---

## Project Structure

```
sutra/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── sutra-schema/             # Core types & serde (68 tests)
│   ├── sutra-common/             # Shared traits & errors (66 tests)
│   ├── sutra-mgtg/               # Complexity metrics (7 tests)
│   ├── sutra-dependency/         # Dependency analysis (121 tests)
│   ├── sutra-rse/                # Runtime survivability (123 tests)
│   ├── sutra-process/            # Change mining (66 tests)
│   ├── sutra-ml/                 # Logistic regression (70 tests)
│   ├── sutra-llm/                # LLM validation (45 tests)
│   ├── sutra-hitl/               # Human feedback (62 tests)
│   ├── sutra-orchestrator/       # Engine coordinator (22 unit + 11 E2E tests)
│   ├── sutra-ci/                 # SARIF & PR comments (32 tests)
│   └── sutra-cli/                # CLI entry point
├── docs/
│   ├── book.md                   # Complete mathematical reference
│   ├── example.py                # End-to-end Python example
│   └── arch.md                   # Architecture documentation
├── website/
│   ├── index.html                # Landing page
│   ├── book.html                 # Rendered reference book
│   ├── example.html              # Rendered example
│   ├── style.css                 # Styles
│   ├── demo.js                   # Interactive risk calculator
│   └── live-demo.js              # GitHub repo analysis
├── WHITEPAPER.md                 # Technical white paper
└── CHANGELOG.md                  # Release history
```

---

## Requirements

- **Rust**: 1.86+ (stable)
- **OS**: Linux, macOS
- **Git**: 2.x (for process engine)
- **Ollama** (optional): For LLM validation engine
- **Dependencies**: None external — all engines use pure Rust libraries

---

## License

Apache 2.0. See `LICENSE` for details.
