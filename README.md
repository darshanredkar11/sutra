# Sutra

**Predict production software failures before they happen.**

[![Rust](https://img.shields.io/badge/rust-1.86%2B-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-green)](https://www.apache.org/licenses/LICENSE-2.0)
[![SARIF](https://img.shields.io/badge/SARIF-2.1.0-orange)](https://docs.oasis-open.org/sarif/sarif/v2.1.0/)

Sutra is a deterministic, math-first framework that estimates production failure risk from source code. It fuses five analysis engines — complexity metrics, dependency analysis, process/change mining, logistic regression, and LLM validation — into a single composable pipeline.

---

## Quick Start

```bash
# Analyze a repository with all engines
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
┌──────────────────────────────────────────────────┐
│                   sutra analyze                    │
├──────────────────────────────────────────────────┤
│  mgtg        → complexity metrics (McCabe, etc.)  │
│  dependency  → import graph + Tarjan SCC cycles   │
│  process     → git history + entropy + JIT feats  │
│  ml          → logistic regression (14 features)   │
│  llm         → Ollama-based finding validation     │
│  hitl        → human feedback precision tracking   │
├──────────────────────────────────────────────────┤
│  Output: unified risk score + findings + SARIF     │
└──────────────────────────────────────────────────┘
```

Each engine implements the `AnalysisEngine` trait and can run independently or orchestrated:

```rust
let mut orchestrator = Orchestrator::new();
orchestrator.register(Engine::Mgtg, Box::new(MgtgEngine::new()));
orchestrator.register(Engine::Dependency, Box::new(DependencyEngine::new()));
orchestrator.register(Engine::Process, Box::new(ProcessEngine::new()));

let result = orchestrator.analyze(&request)?;
```

---

## Engines

| Engine | Crate | What It Does | Deterministic |
|--------|-------|-------------|:---:|
| **mgtg** (7 tests) | `sutra-mgtg` | Cyclomatic/cognitive complexity via tree-sitter | Yes |
| **dependency** (121 tests) | `sutra-dependency` | Import graph, circular dep detection (Tarjan SCC), architecture rules | Yes |
| **process** (66 tests) | `sutra-process` | Git history mining, Hassan entropy, co-change graph, 14 JIT features | Yes |
| **ml** (70 tests) | `sutra-ml` | Logistic regression with SGD + L2, AUC evaluation, model persistence | No* |
| **llm** (45 tests) | `sutra-llm` | Finding validation via Ollama (optional, disabled by default) | No |
| **hitl** (62 tests) | `sutra-hitl` | Human feedback collection, per-engine precision, auto-adjust findings | Yes |

\* ML is deterministic for a given trained model; training itself is not deterministic.

---

## CLI Reference

```bash
# General
sutra --help
sutra --version

# Run analysis
sutra analyze <path>
sutra analyze <path> --engine mgtg        # single engine
sutra analyze <path> --engine all         # all 5 engines (default)
sutra analyze <path> --format json        # JSON output
sutra analyze <path> --format sarif       # SARIF 2.1.0 output
sutra analyze <path> --output results.sarif
sutra analyze <path> --commit abc123      # specific commit
sutra analyze <path> --arch arch.toml     # dependency architecture rules
sutra analyze <path> --llm                # enable LLM validation
sutra analyze <path> --llm-model llama3.1
sutra analyze <path> --ollama-url http://localhost:11434

# Health check
sutra health <path>

# HTTP server
sutra server --port 8080
```

### API Endpoints (server mode)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `POST /v1/analyze` | POST | Run analysis with JSON request |
| `GET /v1/health` | GET | Component health check |
| `GET /v1/status` | GET | Server and engine status |

---

## Project Structure

```
sutra/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── sutra-schema/           # Core types & serde
│   ├── sutra-common/           # Shared traits & errors
│   ├── sutra-mgtg/             # Complexity engine
│   ├── sutra-dependency/       # Dependency analysis
│   ├── sutra-process/          # Change mining
│   ├── sutra-ml/               # Logistic regression
│   ├── sutra-llm/              # LLM validation
│   ├── sutra-hitl/             # Human feedback
│   ├── sutra-orchestrator/     # Engine coordinator
│   ├── sutra-ci/               # SARIF & PR comments
│   └── sutra-cli/              # CLI entry point
├── WHITEPAPER.md               # Technical white paper
├── CHANGELOG.md                # Release history
└── README.md                   # This file
```

---

## Configuration

### Architecture Rules (Dependency Engine)

Create an `arch.toml` file to define layer constraints:

```toml
[layer.presentation]
allowed_deps = ["application"]

[layer.application]
allowed_deps = ["domain", "infrastructure"]

[layer.domain]
allowed_deps = []

[layer.infrastructure]
allowed_deps = []
```

Pass it with `sutra analyze . --arch arch.toml`.

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
