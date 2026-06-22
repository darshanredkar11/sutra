# Sutra

**Predict production software failures before they happen.**

[![Rust](https://img.shields.io/badge/rust-1.86%2B-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](https://www.apache.org/licenses/LICENSE-2.0)
[![SARIF](https://img.shields.io/badge/SARIF-2.1.0-orange)](https://docs.oasis-open.org/sarif/sarif/v2.1.0/)

Sutra is a deterministic, math-first framework that estimates production failure risk from source code. It fuses **7 analysis engines** — complexity, dependency, runtime survivability (RSE), process/change mining, logistic regression, LLM validation, and human feedback — into a single composable pipeline. **18K+ lines of Rust, 682 tests, 13 crates, zero external runtime deps.**

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
| **orchestrator** | `sutra-orchestrator` | 22 | Engine registration, risk fusion (max), panic safety, HTTP API | Yes |
| **schema** | `sutra-schema` | 68 | Core types, serde JSON/YAML/TOML, proptest fuzzing | — |
| **common** | `sutra-common` | 66 | Shared traits, errors, config, health, metrics | — |
| **ci** | `sutra-ci` | 32 | SARIF 2.1.0 output, markdown PR comments | — |

\* ML is deterministic for a given trained model; training itself is not deterministic.

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
| `GET /v1/health` | GET | Component health check |
| `GET /v1/status` | GET | Server and engine status |
| `GET /v1/report` | GET | HTML report page with interactive analysis form |
| `POST /v1/demo` | POST | Clone any public GitHub repo and run all 7 engines |

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
│   ├── sutra-orchestrator/       # Engine coordinator (22 tests)
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
