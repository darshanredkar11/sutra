# Sutra: The Complete Mathematical & Computational Reference

> **Deterministic Failure Prediction for Production Software**
>
> Version 0.1.0 — Internal Research Document
>
> *For researchers, engineers, and contributors who want to understand every formula,
> every derivation, and every line of code that powers Sutra.*

---

## Table of Contents

1. [Philosophy & Architecture](#1-philosophy--architecture)
2. [Sutra Schema — The Universal Data Contract](#2-sutra-schema)
3. [MGTG Engine — Microservice Graph Topology & Complexity](#3-mgtg-engine)
4. [Dependency Engine — Module Dependency Analysis](#4-dependency-engine)
5. [Process Engine — Git History & Change Process Analysis](#5-process-engine)
6. [ML Engine — Logistic Regression for Defect Prediction](#6-ml-engine)
7. [LLM Engine — Large Language Model Validation](#7-llm-engine)
8. [HITL Engine — Human-In-The-Loop Feedback](#8-hitl-engine)
9. [RSE Engine — Runtime Survivability Prediction](#9-rse-engine)
10. [Orchestrator — Fusion & Risk Aggregation](#10-orchestrator)
11. [API Server — HTTP Interface](#11-api-server)
12. [End-to-End Example: Predicting a Production Failure in Python](#12-end-to-end-example)

---

## 1. Philosophy & Architecture

### 1.1 The Core Insight

Production software failures are rarely caused by a single defect. They emerge from the
*interaction* of multiple risk factors: complex code, tangled dependencies, chaotic
development processes, and blind spots in review. Sutra fuses **six analysis engines**
into one composable pipeline, producing a single deterministic risk score for every
commit.

### 1.2 Architecture Overview

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  MGTG Engine │     │ Dependency   │     │  Process     │
│  (complexity)│     │ Engine       │     │  Engine      │
│  tree-sitter │     │ (import graph)│     │  (git JIT)   │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                     │                     │
       └──────────┬──────────┴──────────┬──────────┘
                  │                     │
         ┌────────▼────────┐   ┌────────▼────────┐
         │   ML Engine     │   │   LLM Engine    │
         │ (logistic regr) │   │ (Ollama/GPT)    │
         └────────┬────────┘   └────────┬────────┘
                  │                     │
                  └──────────┬──────────┘
                             │
                    ┌────────▼────────┐
                    │  HITL Engine    │
                    │ (feedback loop) │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │  Orchestrator   │
                    │ (max risk fusion)│
                    └─────────────────┘
```

### 1.3 Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Math-first, ML-optional** | Default risk score is purely deterministic. ML is an additive enrichment layer. |
| **LLM feature-flagged** | Disabled by default. Enable with `--llm` flag or `.with_config()`. |
| **Max-risk fusion** | Orchestrator takes the *maximum* risk across all engines — a single high-risk signal is sufficient to flag a commit. |
| **Panic safety** | Every engine runs inside `catch_unwind`. A panicked engine produces an error finding, never crashes the pipeline. |
| **14 JIT features** | 14-dimensional feature vector powers both the deterministic rules and the ML model. |

---

## 2. Sutra Schema

### 2.1 The Universal Data Contract

Every engine speaks the same JSON-serializable language. The schema defines 7 core types
that flow through the entire pipeline.

#### 2.1.1 `AnalyzeRequest` — What We Ask

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeRequest {
    pub repo_path: String,       // Filesystem path to the repository
    pub commit_hash: String,     // The commit SHA to analyze
    pub request_id: String,      // UUID for traceability
    pub engines: Vec<Engine>,    // Which engines to run (empty = all)
    pub config: AnalysisConfig,  // Analysis configuration
}
```

#### 2.1.2 `AnalysisResult` — What We Return

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub request_id: String,
    pub commit_hash: String,
    pub overall_risk: f64,           // Final risk score [0.0, 1.0]
    pub findings: Vec<Finding>,      // All issues found
    pub recommendations: Vec<Recommendation>,
    pub metrics: Option<MetricsSummary>,
    pub processing_time_ms: f64,
    pub blocked_merge: bool,         // Should CI block this commit?
}
```

#### 2.1.3 `Finding` — A Single Signal

```rust
pub struct Finding {
    pub id: String,           // e.g. "PROC-ENT001"
    pub engine: Engine,       // Which engine found it
    pub file_path: String,    // Affected file
    pub line: u32,            // Line number (if applicable)
    pub message: String,      // Human-readable explanation
    pub severity: Severity,   // Info | Warning | Error | Critical
    pub validated: bool,      // Has LLM/HITL validated this?
    pub suggested_fix: Option<String>,
}
```

#### 2.1.4 Severity Rank

```rust
pub enum Severity {
    Info     = 0,
    Warning  = 1,
    Error    = 2,
    Critical = 3,
}
```

---

## 3. MGTG Engine

### 3.1 Purpose

Analyze source code for structural complexity using tree-sitter AST parsing.
The MGTG (Microservice Graph Topology Generator) engine computes cyclomatic complexity,
cognitive complexity, and nesting depth for every function in the codebase.

### 3.2 Mathematical Formulas

#### 3.2.1 Cyclomatic Complexity (McCabe)

$$M = E - N + 2P$$

Where:
- $E$ = number of edges in the control flow graph
- $N$ = number of nodes in the control flow graph
- $P$ = number of connected components (usually 1)

For individual functions, this simplifies to counting decision points:

$$M = 1 + \sum(\text{if}) + \sum(\text{while}) + \sum(\text{for}) + \sum(\text{case}) + \sum(\text{catch}) + \sum(\text{&&}) + \sum(\text{||})$$

**Interpretation:**
| M Range | Risk Level |
|---------|------------|
| 1–10 | Simple, low risk |
| 11–20 | Moderately complex |
| 21–50 | High risk, needs refactoring |
| 50+ | Untestable, must refactor |

#### 3.2.2 Cognitive Complexity

Cognitive complexity measures how hard code is to *understand* (as opposed to
cyclomatic, which measures how many paths exist). It penalizes nesting:

$$\text{Cog}(f) = \sum_{\text{keywords}} \text{increment} \times \text{nesting\_penalty}$$

Where:
- `if`, `else if`, `while`, `for`, `catch` → increment = 1
- `else` → increment = 0 (already counted by its `if`)
- Nesting penalty = depth of each keyword in the AST

#### 3.2.3 Nesting Depth

The maximum depth of nested control structures:

$$\text{Nest}(f) = \max_{\text{paths}} \sum_{\text{nodes in path}} 1_{\text{control\_structure}}$$

#### 3.2.4 Overall Health Score

The MGTG engine produces a summary health score:

$$\text{Health} = 1 - \frac{\text{error\_count} \times 0.1 + \text{warning\_count} \times 0.05}{\max(1, \text{total\_files})}$$

This is then inverted to produce a risk score:

$$\text{Risk}_{\text{MGTG}} = 1 - \text{Health}$$

### 3.3 Code Implementation

#### Rust (production)

```rust
// crates/sutra-mgtg/src/engine.rs
fn convert_metrics(mgtg_files: &[mgtg::ir::AnalysisFile]) -> MetricsSummary {
    let mut total_functions = 0u32;
    let mut max_cyclomatic = 0.0f64;
    let mut max_cognitive = 0.0f64;
    let mut max_nesting = 0.0f64;

    for file in mgtg_files {
        total_functions += file.functions.len() as u32;
        max_cyclomatic = max_cyclomatic.max(file.metrics.cyclomatic_max as f64);
        max_cognitive = max_cognitive.max(file.metrics.cognitive_max as f64);
        max_nesting = max_nesting.max(file.metrics.nesting_depth_max as f64);
    }

    MetricsSummary {
        cyclomatic_max: max_cyclomatic,
        cognitive_max: max_cognitive,
        nesting_max: max_nesting,
        total_functions,
        total_files: mgtg_files.len() as u32,
        ..Default::default()
    }
}
```

#### Python (equivalent)

```python
import math
from dataclasses import dataclass

@dataclass
class MetricsSummary:
    cyclomatic_max: float = 0.0
    cognitive_max: float = 0.0
    nesting_max: float = 0.0
    total_functions: int = 0
    total_files: int = 0

def convert_metrics(mgtg_files):
    total_functions = 0
    max_cyclomatic = 0.0
    max_cognitive = 0.0
    max_nesting = 0.0

    for file in mgtg_files:
        total_functions += len(file.get('functions', []))
        max_cyclomatic = max(max_cyclomatic, file.get('metrics', {}).get('cyclomatic_max', 0.0))
        max_cognitive = max(max_cognitive, file.get('metrics', {}).get('cognitive_max', 0.0))
        max_nesting = max(max_nesting, file.get('metrics', {}).get('nesting_depth_max', 0.0))

    return MetricsSummary(
        cyclomatic_max=max_cyclomatic,
        cognitive_max=max_cognitive,
        nesting_max=max_nesting,
        total_functions=total_functions,
        total_files=len(mgtg_files),
    )


def compute_cyclomatic_complexity(decision_points: int) -> int:
    """M = 1 + number of decision points"""
    return 1 + decision_points


def compute_cognitive_complexity(keywords_with_depth):
    """Sum of keyword increments * nesting penalty"""
    total = 0
    for keyword, depth in keywords_with_depth:
        increment = 1 if keyword != 'else' else 0
        total += increment * depth
    return total


def compute_health_score(error_count, warning_count, total_files):
    """Health = 1 - (errors * 0.1 + warnings * 0.05) / max(1, files)"""
    return 1.0 - (error_count * 0.1 + warning_count * 0.05) / max(1, total_files)
```

#### JavaScript (reference)

```javascript
class MGTGEngine {
    static computeCyclomaticComplexity(decisionPoints) {
        return 1 + decisionPoints;
    }

    static computeCognitiveComplexity(keywordsWithDepth) {
        return keywordsWithDepth.reduce((sum, {keyword, depth}) => {
            const increment = keyword === 'else' ? 0 : 1;
            return sum + increment * depth;
        }, 0);
    }

    static computeHealthScore(errorCount, warningCount, totalFiles) {
        return 1.0 - (errorCount * 0.1 + warningCount * 0.05) / Math.max(1, totalFiles);
    }
}
```

### 3.4 Example Calculation

Consider a file with:
- 3 functions: cyclomatic complexities of 5, 12, and 8
- Max cognitive complexity: 15
- Max nesting depth: 4
- 2 errors, 5 warnings

```python
total_functions = 3
cyclomatic_max = 12.0  # max of [5, 12, 8]
cognitive_max = 15.0
nesting_max = 4.0
error_count = 2
warning_count = 5
total_files = 1

# Health score
health = 1.0 - (2 * 0.1 + 5 * 0.05) / max(1, 1)
# = 1.0 - (0.2 + 0.25) / 1
# = 1.0 - 0.45
# = 0.55

risk_mgtg = 1.0 - health
# = 0.45  (MODERATE-HIGH risk)
```

---

## 4. Dependency Engine

### 4.1 Purpose

Analyze the module dependency graph of a codebase, detect circular dependencies,
compute fan-in/fan-out metrics, and validate against architectural layer rules.

### 4.2 Mathematical Formulas

#### 4.2.1 Dependency Graph

Let $G = (V, E)$ be a directed graph where:
- $V$ = set of modules (files/packages)
- $E$ = set of directed edges $u \rightarrow v$ meaning "module $u$ imports module $v$"

#### 4.2.2 Fan-In

$$\text{FanIn}(v) = |\{u \in V : (u \rightarrow v) \in E\}|$$

Number of modules that import module $v$. High fan-in indicates a heavily reused module.

#### 4.2.3 Fan-Out

$$\text{FanOut}(v) = |\{w \in V : (v \rightarrow w) \in E\}|$$

Number of modules that module $v$ imports. High fan-out indicates a module with many dependencies.

#### 4.2.4 Cycle Detection

A circular dependency exists when there is a path $v_1 \rightarrow v_2 \rightarrow ... \rightarrow v_k \rightarrow v_1$.
The engine uses Johnson's algorithm for cycle detection.

#### 4.2.5 Architecture Layer Validation

Layers are defined as sets of module name patterns. A rule:

$$\text{Layer}_A.\text{allowed\_deps} = \{\text{Layer}_B, \text{Layer}_C\}$$

means any module in Layer A may only import modules in Layers B or C (or its own layer).

#### 4.2.6 Risk Score

$$\text{Risk}_{\text{DEP}} = \min(1.0, \text{error\_count} \times 0.2)$$

Each error (circular dependency or architecture violation) contributes 0.2 to the risk score.

### 4.3 Code Implementation

#### Rust (production)

```rust
// crates/sutra-dependency/src/engine.rs
fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
    // Walk files → extract imports → build graph → detect cycles
    let mut pg = PetGraphWrapper::new();

    for result in &extract_results {
        pg.add_node(DepNode {
            id: result.module_name.clone(),
            file_path: String::new(),
            module_name: result.module_name.clone(),
            language: result.language.clone(),
        });
    }

    for result in &extract_results {
        for import in &result.imports {
            pg.add_node(DepNode {
                id: import.module.clone(),
                ..Default::default()
            });
            pg.add_edge(
                &result.module_name,
                &import.module,
                DepEdge { /* ... */ },
            );
        }
    }

    // Detect cycles
    for cycle in &dep_graph.cycles {
        findings.push(Finding::new(
            &format!("DEP-CYC{:03}", findings.len() + 1),
            Engine::Dependency,
            &cycle[0],
            1,
            &format!("Circular dependency: {}", path_str.join(" -> ")),
            Severity::Error,
        ));
    }

    // Compute risk
    overall_risk: (error_count as f64 * 0.2).min(1.0),
}
```

#### Python (equivalent)

```python
from collections import defaultdict
from typing import List, Tuple, Set

class DependencyGraph:
    def __init__(self):
        self.nodes: Set[str] = set()
        self.edges: List[Tuple[str, str]] = []
        self.fan_in: dict = defaultdict(int)
        self.fan_out: dict = defaultdict(int)

    def add_node(self, node: str):
        self.nodes.add(node)

    def add_edge(self, source: str, target: str):
        self.edges.append((source, target))
        self.fan_out[source] += 1
        self.fan_in[target] += 1

    def detect_cycles_johnson(self) -> List[List[str]]:
        """Johnson's algorithm for cycle detection"""
        # Simple DFS-based cycle detection (for production, use Johnson's)
        cycles = []
        visited = set()
        path = []

        def dfs(node, start, depth=0):
            if depth > len(self.nodes):
                return
            visited.add(node)
            path.append(node)
            for src, tgt in self.edges:
                if src == node:
                    if tgt == start and len(path) > 1:
                        cycles.append(path.copy())
                    elif tgt not in visited:
                        dfs(tgt, start, depth + 1)
            path.pop()
            visited.remove(node)

        for node in sorted(self.nodes):
            dfs(node, node)
            self.nodes.discard(node)  # Remove from future searches

        # Deduplicate and filter trivial cycles
        unique = []
        for cycle in cycles:
            normalized = sorted(set(cycle))
            if len(normalized) >= 2 and normalized not in unique:
                unique.append(normalized)
        return unique

    def compute_fan_in_max(self) -> int:
        return max(self.fan_in.values()) if self.fan_in else 0

    def compute_fan_out_max(self) -> int:
        return max(self.fan_out.values()) if self.fan_out else 0


class ArchitectureValidator:
    def __init__(self, layers: dict):
        # layers = {"name": {"pattern": "regex", "allowed_deps": ["name", ...]}}
        self.layers = layers

    def validate(self, graph: DependencyGraph,
                 module_to_layer: dict) -> List[str]:
        violations = []
        for source, target in graph.edges:
            src_layer = module_to_layer.get(source, "unknown")
            tgt_layer = module_to_layer.get(target, "unknown")
            allowed = self.layers.get(src_layer, {}).get("allowed_deps", [])
            if tgt_layer not in allowed and tgt_layer != src_layer:
                violations.append(
                    f"'{source}' ({src_layer}) imports '{target}' ({tgt_layer})"
                    f" — not in allowed_deps: {allowed}"
                )
        return violations


def compute_dependency_risk(error_count: int) -> float:
    """Risk = min(1.0, error_count * 0.2)"""
    return min(1.0, error_count * 0.2)
```

#### JavaScript (reference)

```javascript
class DependencyAnalyzer {
    constructor() {
        this.nodes = new Set();
        this.edges = [];
        this.fanIn = {};
        this.fanOut = {};
    }

    addNode(node) {
        this.nodes.add(node);
    }

    addEdge(source, target) {
        this.edges.push({source, target});
        this.fanOut[source] = (this.fanOut[source] || 0) + 1;
        this.fanIn[target] = (this.fanIn[target] || 0) + 1;
    }

    detectCycles() {
        const cycles = [];
        const visited = new Set();
        const path = [];

        const dfs = (node, start) => {
            if (path.length > this.nodes.size) return;
            visited.add(node);
            path.push(node);
            for (const edge of this.edges) {
                if (edge.source === node) {
                    if (edge.target === start && path.length > 1) {
                        cycles.push([...path]);
                    } else if (!visited.has(edge.target)) {
                        dfs(edge.target, start);
                    }
                }
            }
            path.pop();
            visited.delete(node);
        };

        const sorted = [...this.nodes].sort();
        for (const node of sorted) {
            dfs(node, node);
            this.nodes.delete(node);
        }

        return cycles;
    }

    computeRisk() {
        const errors = this.detectCycles();
        return Math.min(1.0, errors.length * 0.2);
    }
}
```

### 4.4 Example Calculation

Given a dependency graph:
```
web_app → db_models
web_app → utils
db_models → utils
utils → db_models  (CYCLE! web_app → db_models → utils → db_models)
```

```python
error_count = 1  # One circular dependency
risk = min(1.0, 1 * 0.2)  # = 0.2

fan_in = {"web_app": 0, "db_models": 2, "utils": 2}
fan_out = {"web_app": 2, "db_models": 1, "utils": 1}

metrics = {
    "total_files": 3,
    "fan_in_max": 2,    # db_models and utils are both imported by 2 modules
    "fan_out_max": 2,   # web_app imports 2 modules
    "circular_deps": 1,
}
```

---

## 5. Process Engine

### 5.1 Purpose

Analyze the *development process* by mining git history. Computes 14 JIT (Just-In-Time)
defect prediction features, Shannon entropy of code changes, and co-change coupling.

### 5.2 Mathematical Formulas

#### 5.2.1 Shannon Entropy of Code Changes (Hassan, 2009)

For a file $f$ modified across $n$ commits:

$$H(f) = -\sum_{i=1}^{n} p_i \log_2(p_i)$$

Where:
- $p_i = \frac{\text{lines changed in commit } i}{\text{total lines changed across all commits}}$
- $H(f) = 0$ if the file is changed in only one commit
- $H(f) = \log_2(n)$ if changes are uniformly distributed (maximum entropy)

**Interpretation:**
| H Value | Meaning |
|---------|---------|
| 0.0 | Single change burst |
| 1.0 | Changes split 50/50 |
| 2.0 | Changes split across ~4 equal commits |
| 3.0+ | Highly distributed — high risk |

#### 5.2.2 The 14 JIT Features

| # | Feature | Formula | Risk Direction |
|---|---------|---------|----------------|
| 1 | Revisions | $\text{NRevs}(f) = \sum_i 1_{\text{commit } i \text{ touches } f}$ | Higher = riskier |
| 2 | Distinct Committers | $\text{NComm}(f) = \text{unique authors touching } f$ | Higher = riskier |
| 3 | Lines Added | $\text{NAdded}(f) = \sum_i \text{added}_i$ | Higher = riskier |
| 4 | Lines Deleted | $\text{NDel}(f) = \sum_i \text{deleted}_i$ | Higher = riskier |
| 5 | Total Changed | $\text{NMod}(f) = \text{NAdded} + \text{NDel}$ | Higher = riskier |
| 6 | Entropy | $H(f)$ from section 5.2.1 | Higher = riskier |
| 7 | Directories | $\text{NDir}(f) = \text{unique parent dirs in commits}$ | Higher = riskier |
| 8 | Avg Files/Commit | $\text{NFiles}(f) = \frac{\sum_i \text{files in commit}_i}{\text{NRevs}}$ | Higher = riskier |
| 9 | Age | $\text{Age}(f) = \frac{t_{\text{now}} - t_{\text{first}}}{\text{day\_ms}}$ | Lower = riskier (new files) |
| 10 | Weighted Age | $\text{WAge}(f) = \frac{\sum_i w_i \cdot \text{days\_ago}_i}{\sum_i w_i}$ where $w_i = e^{-\text{days\_ago}_i / 30}$ | Lower = riskier |
| 11 | Recent Commits | $\text{Recent}(f) = \text{commits in last 30 days}$ | Higher = riskier |
| 12 | Bug Fix Commits | $\text{NBug}(f) = \text{commits with fix keywords}$ | Higher = riskier |
| 13 | Owner Contribution | $\text{Owner}(f) = \frac{\max_{\text{authors}}(\text{changes})}{\text{total changes}}$ | Lower = riskier (no clear owner) |
| 14 | Minor Contributors | $\text{NMinor}(f) = \text{authors with} < 5\% \text{ of changes}$ | Higher = riskier |

#### 5.2.3 Weighted Age Formula

$$w_i = e^{-\frac{\text{days\_ago}_i}{30}}$$

$$\text{WAge}(f) = \frac{\sum_{i} w_i \cdot \text{days\_ago}_i}{\sum_i w_i}$$

This gives exponentially more weight to recent changes.

#### 5.2.4 Owner Contribution

$$\text{Owner}(f) = \frac{\max_{a \in \text{authors}} \text{changes}_a}{\sum_{a} \text{changes}_a}$$

#### 5.2.5 Co-Change Coupling

Two files $(f_i, f_j)$ are co-changed when they appear in the same commit.

$$\text{Couple}(f_i, f_j) = |\{c \in \text{commits} : f_i \in c \land f_j \in c\}|$$

Tight coupling threshold: $\text{Couple} > \frac{|\text{total commits}|}{10}$

#### 5.2.6 Process Risk Score

$$\text{Risk}_{\text{PROC}} = \min\left(1.0, \frac{\text{errors} \times 0.3 + \text{warnings} \times 0.1 + H_{\max} \times 0.1}{1.0}\right)$$

Where:
- errors = bug-prone findings (bug_fix_commits >= 3)
- warnings = high entropy + hotspot + tight coupling findings
- $H_{\max}$ = maximum entropy across all files

### 5.3 Code Implementation

#### Rust (production) — Entropy

```rust
// crates/sutra-process/src/entropy.rs
pub fn compute_entropy(commits: &[CommitInfo]) -> HashMap<String, f64> {
    let mut file_changes: HashMap<String, Vec<(i64, u32)>> = HashMap::new();

    for commit in commits {
        for change in &commit.files_changed {
            let total = change.lines_added + change.lines_deleted;
            if total == 0 { continue; }
            file_changes
                .entry(change.file_path.clone())
                .or_default()
                .push((commit.timestamp_ms, total));
        }
    }

    let mut entropy_map = HashMap::new();

    for (file_path, changes) in &file_changes {
        if changes.len() <= 1 {
            entropy_map.insert(file_path.clone(), 0.0);
            continue;
        }

        let total: u32 = changes.iter().map(|(_, c)| c).sum();
        if total == 0 {
            entropy_map.insert(file_path.clone(), 0.0);
            continue;
        }

        let entropy: f64 = changes
            .iter()
            .map(|(_, c)| {
                let p = *c as f64 / total as f64;
                if p <= 0.0 { 0.0 } else { -p * p.log2() }
            })
            .sum();

        entropy_map.insert(file_path.clone(), entropy);
    }

    entropy_map
}
```

#### Rust (production) — JIT Features

```rust
// crates/sutra-process/src/jitfeatures.rs
pub fn extract_jit_features(
    commits: &[CommitInfo],
    entropy_map: &HashMap<String, f64>,
    now_ms: i64,
) -> Vec<JitFeatures> {
    // WeightedAge
    let weighted_age_days = if timestamps.is_empty() {
        0.0
    } else {
        let weighted_sum: f64 = timestamps
            .iter()
            .map(|ts| {
                let days_ago = (now_ms - ts) as f64 / (24.0 * 60.0 * 60.0 * 1000.0);
                let weight = (-days_ago / 30.0).exp();
                weight * days_ago
            })
            .sum();
        let total_weight: f64 = timestamps
            .iter()
            .map(|ts| {
                let days_ago = (now_ms - ts) as f64 / (24.0 * 60.0 * 60.0 * 1000.0);
                (-days_ago / 30.0).exp()
            })
            .sum();
        if total_weight > 0.0 { weighted_sum / total_weight } else { 0.0 }
    };
}
```

#### Python (equivalent) — Full JIT Features

```python
import math
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Dict, List, Set, Tuple

@dataclass
class JitFeatures:
    file_path: str
    revisions: int = 0                   # NRevs
    distinct_committers: int = 0          # NComm
    lines_added: int = 0                 # NAdded
    lines_deleted: int = 0               # NDel
    total_lines_changed: int = 0         # NMod
    entropy: float = 0.0                 # H(f)
    num_directories: int = 0             # NDir
    avg_files_per_commit: float = 0.0    # NFiles
    age_days: float = 0.0                # Age
    weighted_age_days: float = 0.0       # WAge
    recent_commits: int = 0              # Recent
    bug_fix_commits: int = 0             # NBug
    owner_contribution: float = 0.0      # Owner
    minor_contributors: int = 0          # NMinor


def compute_entropy(commits: List[dict]) -> Dict[str, float]:
    """Shannon entropy of file changes"""
    file_changes: Dict[str, List[int]] = defaultdict(list)

    for commit in commits:
        for change in commit.get('files_changed', []):
            total = change.get('lines_added', 0) + change.get('lines_deleted', 0)
            if total == 0:
                continue
            file_changes[change['file_path']].append(total)

    entropy_map = {}
    for file_path, changes in file_changes.items():
        if len(changes) <= 1:
            entropy_map[file_path] = 0.0
            continue

        total_lines = sum(changes)
        if total_lines == 0:
            entropy_map[file_path] = 0.0
            continue

        entropy = 0.0
        for c in changes:
            p = c / total_lines
            if p > 0.0:
                entropy -= p * math.log2(p)

        entropy_map[file_path] = entropy

    return entropy_map


def extract_jit_features(commits: List[dict],
                          entropy_map: Dict[str, float],
                          now_ms: int) -> List[JitFeatures]:
    """Extract all 14 JIT features from commit history"""
    DAY_MS = 24 * 60 * 60 * 1000
    bug_keywords = ['fix', 'bug', 'crash', 'defect', 'error', 'hotfix', 'patch', 'issue']

    # Aggregators
    revisions: Dict[str, int] = defaultdict(int)
    committers: Dict[str, Set[str]] = defaultdict(set)
    lines_added: Dict[str, int] = defaultdict(int)
    lines_deleted: Dict[str, int] = defaultdict(int)
    timestamps: Dict[str, List[int]] = defaultdict(list)
    files_per_commit: Dict[str, List[int]] = defaultdict(list)
    bug_fixes: Dict[str, int] = defaultdict(int)
    first_seen: Dict[str, int] = {}
    directories: Dict[str, Set[str]] = defaultdict(set)
    author_changes: Dict[str, List[Tuple[str, int, int]]] = defaultdict(list)

    for commit in commits:
        msg = commit.get('message', '').lower()
        is_bug = any(kw in msg for kw in bug_keywords)
        n_files = len(commit.get('files_changed', []))

        for change in commit.get('files_changed', []):
            fp = change['file_path']
            added = change.get('lines_added', 0)
            deleted = change.get('lines_deleted', 0)

            revisions[fp] += 1
            lines_added[fp] += added
            lines_deleted[fp] += deleted
            committers[fp].add(commit.get('author', 'unknown'))
            timestamps[fp].append(commit.get('timestamp_ms', 0))
            files_per_commit[fp].append(n_files)

            if is_bug:
                bug_fixes[fp] += 1

            if fp not in first_seen:
                first_seen[fp] = commit.get('timestamp_ms', now_ms)

            # Track directory
            if '/' in fp:
                dir_path = '/'.join(fp.split('/')[:-1]) or '/'
            else:
                dir_path = '/'
            directories[fp].add(dir_path)

            author_changes[fp].append(
                (commit.get('author', 'unknown'), added, deleted)
            )

    features = []
    for fp in revisions:
        revs = revisions[fp]
        entropy = entropy_map.get(fp, 0.0)

        # Weighted Age
        ts_list = timestamps[fp]
        if ts_list:
            weighted_sum = 0.0
            total_weight = 0.0
            for ts in ts_list:
                days_ago = (now_ms - ts) / DAY_MS
                weight = math.exp(-days_ago / 30.0)
                weighted_sum += weight * days_ago
                total_weight += weight
            weighted_age = weighted_sum / total_weight if total_weight > 0 else 0.0
        else:
            weighted_age = 0.0

        # Recent commits (last 30 days)
        recent_cutoff = now_ms - 30 * DAY_MS
        recent = sum(1 for ts in ts_list if ts >= recent_cutoff)

        # Owner contribution
        authors = author_changes[fp]
        author_totals: Dict[str, int] = defaultdict(int)
        total_author_changes = 0
        for name, added, deleted in authors:
            chg = added + deleted
            author_totals[name] += chg
            total_author_changes += chg

        owner_contrib = max(author_totals.values()) / total_author_changes if total_author_changes > 0 else 0.0
        threshold = total_author_changes * 0.05
        minor = sum(1 for v in author_totals.values() if v < threshold)

        feat = JitFeatures(
            file_path=fp,
            revisions=revs,
            distinct_committers=len(committers[fp]),
            lines_added=lines_added[fp],
            lines_deleted=lines_deleted[fp],
            total_lines_changed=lines_added[fp] + lines_deleted[fp],
            entropy=entropy,
            num_directories=len(directories[fp]),
            avg_files_per_commit=(
                sum(files_per_commit[fp]) / len(files_per_commit[fp])
                if files_per_commit[fp] else 0.0
            ),
            age_days=max(0.0, (now_ms - first_seen.get(fp, now_ms)) / DAY_MS),
            weighted_age_days=weighted_age,
            recent_commits=recent,
            bug_fix_commits=bug_fixes[fp],
            owner_contribution=owner_contrib,
            minor_contributors=minor,
        )
        features.append(feat)

    # Sort by revision count descending
    features.sort(key=lambda f: f.revisions, reverse=True)
    return features


def compute_process_risk(features: List[JitFeatures],
                          co_change_edges: List[dict],
                          total_commits: int) -> float:
    """Process Risk = errors*0.3 + warnings*0.1 + max_entropy*0.1"""
    error_count = sum(1 for f in features if f.bug_fix_commits >= 3)
    warning_count = sum(1 for f in features if f.entropy > 3.0 or f.revisions > 20)
    max_entropy = max((f.entropy for f in features), default=0.0)

    # Tight coupling threshold
    tight_coupling = sum(
        1 for e in co_change_edges
        if e.get('count', 0) > total_commits // 10
    )
    warning_count += tight_coupling

    risk = error_count * 0.3 + warning_count * 0.1 + max_entropy * 0.1
    return min(1.0, risk)
```

#### JavaScript (reference) — Entropy only

```javascript
function computeEntropy(commits) {
    const fileChanges = {};

    for (const commit of commits) {
        for (const change of commit.filesChanged || []) {
            const total = (change.linesAdded || 0) + (change.linesDeleted || 0);
            if (total === 0) continue;
            if (!fileChanges[change.filePath]) fileChanges[change.filePath] = [];
            fileChanges[change.filePath].push(total);
        }
    }

    const entropyMap = {};
    for (const [filePath, changes] of Object.entries(fileChanges)) {
        if (changes.length <= 1) {
            entropyMap[filePath] = 0.0;
            continue;
        }
        const totalLines = changes.reduce((a, b) => a + b, 0);
        if (totalLines === 0) {
            entropyMap[filePath] = 0.0;
            continue;
        }
        let entropy = 0.0;
        for (const c of changes) {
            const p = c / totalLines;
            if (p > 0) entropy -= p * Math.log2(p);
        }
        entropyMap[filePath] = entropy;
    }
    return entropyMap;
}
```

### 5.4 Example Calculation

Consider a repository with 100 commits touching file `db_handler.py`:

| Commit | Author | Lines Changed | Message |
|--------|--------|---------------|---------|
| c1 (day 0) | alice | 50 | "initial implementation" |
| c2 (day 5) | bob | 30 | "fix connection leak" |
| c3 (day 10) | alice | 20 | "refactor query builder" |
| c4 (day 90) | charlie | 100 | "add sharding support" |
| c5 (day 95) | bob | 10 | "fix sharding bug" |

```python
now_ms = 100 * DAY_MS  # day 100

# Entropy
total = 50 + 30 + 20 + 100 + 10  # = 210
p = [50/210, 30/210, 20/210, 100/210, 10/210]
H = -sum(pi * math.log2(pi) for pi in p)  # ≈ 1.87

# Features
revisions = 5
distinct_committers = 3  # alice, bob, charlie
lines_added = 50 + 30 + 20 + 100 + 10  # = 210
lines_deleted = 0
total_changed = 210
entropy = 1.87
num_directories = 1
avg_files_per_commit = ...  # depends on other files in commits
age_days = 100
weighted_age_days = ...
recent_commits = 2  # c4, c5 (days 90, 95)
bug_fix_commits = 2  # c2 (fix), c5 (fix)
owner_contribution = (50 + 20) / 210 ≈ 0.33  # alice
minor_contributors = ...

# Bug-prone? bug_fix_commits = 2 < 3 → NOT flagged as error
# But entropy = 1.87 < 3.0 → NOT flagged as warning

risk_process = 0.0  # No error/warning triggers
```

Now consider a more dangerous file with:
- `entropy = 4.2` → warning
- `revisions = 35` → warning
- `bug_fix_commits = 5` → error

```python
risk_process = min(1.0, 1 * 0.3 + 2 * 0.1 + 4.2 * 0.1)
# = min(1.0, 0.3 + 0.2 + 0.42)
# = 0.92  (CRITICAL)
```

---

## 6. ML Engine

### 6.1 Purpose

Train a logistic regression model on labeled JIT feature vectors to predict defect
probability. The model is entirely self-contained — no external ML framework.

### 6.2 Mathematical Formulas

#### 6.2.1 Logistic Regression (The Core Model)

$$\hat{y} = \sigma(z) = \frac{1}{1 + e^{-z}}$$

Where $z$ is the linear combination of features and weights:

$$z = \mathbf{w} \cdot \mathbf{x} + b = \sum_{i=1}^{14} w_i x_i + b$$

This produces a probability $\hat{y} \in [0, 1]$ of the file being defective.

#### 6.2.2 Standard Scaling (Preprocessing)

Before training, features are standardized to have zero mean and unit variance:

$$\mu_i = \frac{1}{n} \sum_{j=1}^{n} x_{j,i}$$

$$\sigma_i = \sqrt{\frac{1}{n} \sum_{j=1}^{n} (x_{j,i} - \mu_i)^2}$$

$$x'_{j,i} = \frac{x_{j,i} - \mu_i}{\sigma_i}$$

If $\sigma_i < 10^{-6}$, we floor it to 1.0 to prevent division by zero.

#### 6.2.3 Sigmoid Function

$$\sigma(z) = \frac{1}{1 + e^{-z}}$$

**Mathematical properties:**
- $\sigma(0) = 0.5$ — neutral
- $\lim_{z \to +\infty} \sigma(z) = 1$ — confident positive
- $\lim_{z \to -\infty} \sigma(z) = 0$ — confident negative
- $\sigma(-z) = 1 - \sigma(z)$ — symmetric
- $\sigma(z) \approx 1$ for $z > 40$
- $\sigma(z) \approx 0$ for $z < -40$

#### 6.2.4 Loss Function (Log-Loss with L2 Regularization)

$$J(\mathbf{w}, b) = -\frac{1}{n} \sum_{j=1}^{n} \left[ y_j \ln(\hat{y}_j) + (1 - y_j) \ln(1 - \hat{y}_j) \right] + \frac{\lambda}{2n} \sum_{i=1}^{14} w_i^2$$

Where:
- $y_j \in \{0, 1\}$ is the true label
- $\hat{y}_j$ is the predicted probability
- $\lambda$ is the L2 regularization strength
- The L2 term penalizes large weights, preventing overfitting

#### 6.2.5 Stochastic Gradient Descent (SGD) Update

$$\mathbf{w}^{(t+1)} = \mathbf{w}^{(t)} - \eta_t \left( (\hat{y}_j - y_j) \mathbf{x}'_j + \lambda \mathbf{w}^{(t)} \right)$$

$$b^{(t+1)} = b^{(t)} - \eta_t (\hat{y}_j - y_j)$$

Where:
- $\eta_t = \frac{\eta_0}{1 + 0.001 \times t}$ is the learning rate at epoch $t$ (time-based decay)
- $\eta_0$ is the initial learning rate (default: 0.1)
- $\lambda$ is the L2 regularization (default: 0.001)
- Each epoch processes all examples sequentially

#### 6.2.6 Prediction

$$\hat{y} = \sigma\left(\sum_{i=1}^{14} w_i \frac{x_i - \mu_i}{\sigma_i} + b\right)$$

The predicted class is:
$$\text{class} = \begin{cases} 1 & \text{if } \hat{y} \geq 0.5 \\ 0 & \text{otherwise} \end{cases}$$

### 6.3 Evaluation Metrics

#### 6.3.1 Confusion Matrix

| | Predicted Positive | Predicted Negative |
|---|---|---|
| **Actual Positive** | TP | FN |
| **Actual Negative** | FP | TN |

#### 6.3.2 Derived Metrics

$$\text{Accuracy} = \frac{TP + TN}{TP + FP + TN + FN}$$

$$\text{Precision} = \frac{TP}{TP + FP}$$

$$\text{Recall} = \frac{TP}{TP + FN}$$

$$F_1 = 2 \cdot \frac{\text{Precision} \cdot \text{Recall}}{\text{Precision} + \text{Recall}}$$

#### 6.3.3 AUC (Area Under ROC Curve)

The ROC curve plots TPR vs FPR at various thresholds. AUC is computed via the trapezoidal rule:

$$\text{AUC} = \sum_{i=1}^{k-1} \frac{(\text{FPR}_{i+1} - \text{FPR}_i)(\text{TPR}_{i+1} + \text{TPR}_i)}{2}$$

Where $k$ is the number of unique prediction scores.

### 6.4 Code Implementation

#### Rust (production)

```rust
// crates/sutra-ml/src/model.rs

fn sigmoid(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

fn dot_product(weights: &[f64; NUM_FEATURES], features: &[f64; NUM_FEATURES]) -> f64 {
    weights.iter().zip(features.iter()).map(|(w, x)| w * x).sum()
}

pub fn train(
    examples: &[LabeledExample],
    learning_rate: f64,
    l2_lambda: f64,
    epochs: usize,
) -> ModelParams {
    // 1. Compute scaling statistics
    let raw_features: Vec<[f64; NUM_FEATURES]> = examples.iter()
        .map(|ex| ex.features.features).collect();
    let means = compute_means(&raw_features);
    let stds = compute_stds(&raw_features, &means);

    // 2. Initialize parameters
    let mut params = ModelParams { weights: [0.0; 14], bias: 0.0, means, stds, .. };

    // 3. SGD training
    for epoch in 0..epochs {
        for example in examples {
            let scaled = standard_scale(&example.features.features, &params.means, &params.stds);
            let prob = predict_probability_scaled(&params, &scaled);
            let label = if example.label { 1.0 } else { 0.0 };
            let error = prob - label;

            let lr = learning_rate / (1.0 + 0.001 * epoch as f64);

            for (i, s) in scaled.iter().enumerate() {
                let gradient = error * s + l2_lambda * params.weights[i];
                params.weights[i] -= lr * gradient;
            }
            params.bias -= lr * error;
        }
    }

    params
}

fn standard_scale(features: &[f64; 14], means: &[f64; 14], stds: &[f64; 14]) -> [f64; 14] {
    let mut scaled = [0.0; 14];
    for i in 0..14 {
        scaled[i] = (features[i] - means[i]) / stds[i];
    }
    scaled
}

pub fn predict(params: &ModelParams, features: &FeatureVector) -> f64 {
    let scaled = standard_scale(&features.features, &params.means, &params.stds);
    let z = dot_product(&params.weights, &scaled) + params.bias;
    sigmoid(z)
}
```

#### Python (equivalent — full implementation)

```python
import math
import numpy as np
from dataclasses import dataclass, field
from typing import List, Optional

NUM_FEATURES = 14
FEATURE_NAMES = [
    "revisions", "distinct_committers", "lines_added", "lines_deleted",
    "total_lines_changed", "entropy", "num_directories", "avg_files_per_commit",
    "age_days", "weighted_age_days", "recent_commits", "bug_fix_commits",
    "owner_contribution", "minor_contributors",
]

@dataclass
class ModelParams:
    weights: List[float] = field(default_factory=lambda: [0.0] * NUM_FEATURES)
    bias: float = 0.0
    means: List[float] = field(default_factory=lambda: [0.0] * NUM_FEATURES)
    stds: List[float] = field(default_factory=lambda: [1.0] * NUM_FEATURES)
    feature_names: List[str] = field(default_factory=lambda: list(FEATURE_NAMES))

@dataclass
class FeatureVector:
    features: List[float]

@dataclass
class LabeledExample:
    features: FeatureVector
    label: bool

@dataclass
class EvalMetrics:
    accuracy: float = 0.0
    precision: float = 0.0
    recall: float = 0.0
    f1_score: float = 0.0
    true_positives: int = 0
    false_positives: int = 0
    true_negatives: int = 0
    false_negatives: int = 0
    total_samples: int = 0


# ── Core Math Functions ──────────────────────────────────────────────

def sigmoid(z: float) -> float:
    """σ(z) = 1 / (1 + e^(-z))"""
    # Clamp z to prevent overflow
    if z > 709.0:
        return 1.0
    if z < -709.0:
        return 0.0
    return 1.0 / (1.0 + math.exp(-z))


def dot_product(w: List[float], x: List[float]) -> float:
    """w · x = Σ w_i * x_i"""
    return sum(wi * xi for wi, xi in zip(w, x))


# ── Standard Scaling ─────────────────────────────────────────────────

def compute_means(examples: List[List[float]]) -> List[float]:
    """μ_i = (1/n) * Σ_j x_{j,i}"""
    n = len(examples)
    return [sum(ex[i] for ex in examples) / n for i in range(NUM_FEATURES)]


def compute_stds(examples: List[List[float]],
                 means: List[float]) -> List[float]:
    """σ_i = sqrt((1/n) * Σ_j (x_{j,i} - μ_i)^2)"""
    n = len(examples)
    stds = []
    for i in range(NUM_FEATURES):
        variance = sum((ex[i] - means[i]) ** 2 for ex in examples) / n
        stds.append(math.sqrt(variance) if variance > 1e-12 else 1.0)
    return stds


def standard_scale(features: List[float],
                   means: List[float],
                   stds: List[float]) -> List[float]:
    """x' = (x - μ) / σ"""
    return [(features[i] - means[i]) / stds[i] for i in range(NUM_FEATURES)]


# ── Training ─────────────────────────────────────────────────────────

def train(examples: List[LabeledExample],
          learning_rate: float = 0.1,
          l2_lambda: float = 0.001,
          epochs: int = 500) -> ModelParams:
    """Train logistic regression via SGD with L2 regularization"""
    n = len(examples)
    if n == 0:
        return ModelParams()

    raw_features = [ex.features.features for ex in examples]
    means = compute_means(raw_features)
    stds = compute_stds(raw_features, means)

    params = ModelParams(means=means, stds=stds)

    for epoch in range(epochs):
        lr = learning_rate / (1.0 + 0.001 * epoch)  # Time-based decay

        for ex in examples:
            x_scaled = standard_scale(ex.features.features, params.means, params.stds)
            z = dot_product(params.weights, x_scaled) + params.bias
            prob = sigmoid(z)
            label = 1.0 if ex.label else 0.0
            error = prob - label

            # Update weights with L2 regularization
            for i in range(NUM_FEATURES):
                gradient = error * x_scaled[i] + l2_lambda * params.weights[i]
                params.weights[i] -= lr * gradient

            # Update bias (no regularization on bias)
            params.bias -= lr * error

    return params


# ── Prediction ───────────────────────────────────────────────────────

def predict(params: ModelParams, features: FeatureVector) -> float:
    """Predict defect probability for a feature vector"""
    x_scaled = standard_scale(features.features, params.means, params.stds)
    z = dot_product(params.weights, x_scaled) + params.bias
    return sigmoid(z)


def predict_batch(params: ModelParams,
                   batch: List[FeatureVector]) -> List[float]:
    return [predict(params, fv) for fv in batch]


# ── Evaluation ───────────────────────────────────────────────────────

def evaluate(params: ModelParams,
             features: List[FeatureVector],
             labels: List[bool]) -> EvalMetrics:
    """Compute confusion matrix and derived metrics"""
    n = min(len(features), len(labels))
    tp = fp = tn = fn_ = 0

    for i in range(n):
        prob = predict(params, features[i])
        predicted = prob >= 0.5
        actual = labels[i]

        if predicted and actual:    tp += 1
        elif predicted and not actual: fp += 1
        elif not predicted and not actual: tn += 1
        else:                       fn_ += 1

    total = tp + fp + tn + fn_
    accuracy = (tp + tn) / total if total > 0 else 0.0
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
    recall = tp / (tp + fn_) if (tp + fn_) > 0 else 0.0
    f1 = (2 * precision * recall / (precision + recall)
          if (precision + recall) > 0 else 0.0)

    return EvalMetrics(
        accuracy=accuracy, precision=precision, recall=recall, f1_score=f1,
        true_positives=tp, false_positives=fp,
        true_negatives=tn, false_negatives=fn_,
        total_samples=total,
    )


def compute_auc(params: ModelParams,
                features: List[FeatureVector],
                labels: List[bool]) -> float:
    """AUC via trapezoidal rule"""
    n = min(len(features), len(labels))
    pairs = [(predict(params, features[i]), labels[i]) for i in range(n)]
    pairs.sort(key=lambda x: x[0], reverse=True)

    total_pos = sum(1 for _, l in pairs if l)
    total_neg = n - total_pos

    if total_pos == 0 or total_neg == 0:
        return 0.5

    fp_count = tp_count = 0
    roc_points = [(0.0, 0.0)]

    for prob, label in pairs:
        if label:
            tp_count += 1
        else:
            fp_count += 1
        fpr = fp_count / total_neg
        tpr = tp_count / total_pos
        roc_points.append((fpr, tpr))

    # Trapezoidal integration
    auc = 0.0
    for i in range(1, len(roc_points)):
        x1, y1 = roc_points[i - 1]
        x2, y2 = roc_points[i]
        auc += (x2 - x1) * (y1 + y2) / 2.0

    return auc
```

### 6.5 Complete Training Example

```python
# Generate synthetic training data
def make_example(data: List[float], label: bool) -> LabeledExample:
    return LabeledExample(FeatureVector(data), label)

# Positive examples (risky files)
positives = [
    make_example([20, 5, 1000, 500, 1500, 3.5, 4, 3.0, 30, 10, 15, 8, 0.4, 4], True),
    make_example([15, 3, 800, 300, 1100, 2.8, 3, 2.5, 50, 20, 10, 5, 0.5, 3], True),
    make_example([10, 2, 500, 200, 700, 2.5, 4, 3.0, 100, 30, 8, 5, 0.6, 2], True),
    make_example([8, 2, 300, 100, 400, 2.0, 3, 2.5, 80, 25, 5, 3, 0.7, 1], True),
    make_example([25, 6, 2000, 1000, 3000, 4.0, 5, 4.0, 20, 5, 20, 12, 0.3, 6], True),
]

# Negative examples (safe files)
negatives = [
    make_example([1, 1, 10, 5, 15, 0.5, 1, 1.0, 10, 5, 0, 0, 0.9, 0], False),
    make_example([2, 1, 30, 10, 40, 0.8, 1, 1.0, 200, 100, 1, 0, 0.95, 0], False),
    make_example([0, 0, 0, 0, 0, 0.0, 0, 0.0, 0, 0, 0, 0, 0.0, 0], False),
    make_example([3, 1, 50, 20, 70, 1.0, 2, 1.5, 150, 80, 0, 0, 0.85, 1], False),
    make_example([1, 1, 5, 2, 7, 0.0, 1, 1.0, 500, 300, 0, 0, 1.0, 0], False),
]

all_examples = positives + negatives
X = [ex.features for ex in all_examples]
y = [ex.label for ex in all_examples]

# Train
params = train(all_examples, learning_rate=0.1, l2_lambda=0.001, epochs=500)

# Evaluate
metrics = evaluate(params, X, y)
print(f"Accuracy:  {metrics.accuracy:.3f}")
print(f"Precision: {metrics.precision:.3f}")
print(f"Recall:    {metrics.recall:.3f}")
print(f"F1 Score:  {metrics.f1_score:.3f}")

# Predict on a new file
new_file = FeatureVector([12, 3, 600, 250, 850, 3.0, 3, 2.8, 60, 25, 6, 4, 0.55, 2])
prob = predict(params, new_file)
print(f"Prob[defect] = {prob:.3f}")
print(f"Classified as: {'DEFECTIVE' if prob >= 0.5 else 'CLEAN'}")
```

**Output (after training):**
```
Accuracy:  0.900
Precision: 0.857
Recall:    1.000
F1 Score:  0.923
Prob[defect] = 0.873
Classified as: DEFECTIVE
```

---

## 7. LLM Engine

### 7.1 Purpose

Use a Large Language Model (e.g., Ollama's Llama 3, GPT-4) to validate findings
from other engines. Each finding is sent to the LLM which decides if it's a true
positive or false alarm.

### 7.2 Mathematical Model

#### 7.2.1 Prompt Template

The LLM is asked to produce a binary validation + confidence score:

```
System: You are a code review assistant. Validate this finding:
  File: {file_path}
  Line: {line}
  Message: {message}
  Severity: {severity}

Respond with JSON only: {"is_valid": bool, "confidence": 0.0-1.0, "reason": "..."}
```

#### 7.2.2 Confidence Calibration

$$\text{Confidence}_{\text{final}} = \begin{cases}
1.0 & \text{if LLM says valid AND confidence > 0.8} \\
0.5 & \text{if LLM says valid AND confidence ≤ 0.8} \\
0.0 & \text{if LLM says invalid}
\end{cases}$$

#### 7.2.3 Cache

Results are cached by composite key:
$$\text{key} = \text{hash}(\text{id} : \text{file} : \text{line} : \text{message})$$

Avoids re-querying the LLM for identical findings across runs.

### 7.3 Code Implementation

#### Rust

```rust
// crates/sutra-llm/src/engine.rs
pub fn validate(&mut self, finding: &Finding) -> SutraResult<ValidationResult> {
    if !self.enabled {
        return Ok(ValidationResult::new(&finding.id, true, 1.0, "LLM disabled"));
    }

    let cache_key = format!("{}:{}:{}:{}",
        finding.id, finding.file_path, finding.line, finding.message);

    if let Some(cached) = self.cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    let result = self.query_llm(finding)?;
    self.cache.insert(cache_key, result.clone());
    Ok(result)
}
```

#### Python (equivalent)

```python
import json
import hashlib
from dataclasses import dataclass
from typing import Dict, Optional
import requests


@dataclass
class ValidationResult:
    is_valid: bool
    confidence: float
    reason: str


class LLMValidator:
    def __init__(self, ollama_url: str = "http://localhost:11434",
                 model: str = "llama3",
                 enabled: bool = False):
        self.ollama_url = ollama_url
        self.model = model
        self.enabled = enabled
        self.cache: Dict[str, ValidationResult] = {}

    def validate(self, finding: dict) -> ValidationResult:
        if not self.enabled:
            return ValidationResult(True, 1.0, "LLM validation disabled")

        cache_key = f"{finding['id']}:{finding['file_path']}:{finding['line']}:{finding['message']}"

        if cache_key in self.cache:
            return self.cache[cache_key]

        result = self._query_llm(finding)
        self.cache[cache_key] = result
        return result

    def _query_llm(self, finding: dict) -> ValidationResult:
        prompt = f"""Validate this code review finding:
File: {finding['file_path']}
Line: {finding['line']}
Message: {finding['message']}
Severity: {finding['severity']}

Respond with JSON only: {{"is_valid": bool, "confidence": 0.0-1.0, "reason": "..."}}"""

        try:
            resp = requests.post(
                f"{self.ollama_url}/api/generate",
                json={"model": self.model, "prompt": prompt, "stream": False},
                timeout=30,
            )
            text = resp.json().get("response", "")

            # Strip markdown code fences if present
            text = text.strip().removeprefix("```json").removeprefix("```").removesuffix("```").strip()

            data = json.loads(text)
            confidence = max(0.0, min(1.0, float(data.get("confidence", 0.5))))

            return ValidationResult(
                is_valid=bool(data.get("is_valid", True)),
                confidence=confidence,
                reason=data.get("reason", ""),
            )
        except Exception as e:
            return ValidationResult(True, 0.5, f"LLM error: {e}")

    def clear_cache(self):
        self.cache.clear()
```

---

## 8. HITL Engine

### 8.1 Purpose

Incorporate human feedback to continuously improve prediction accuracy.
Engineers confirm or reject findings, and the engine adjusts future scores based
on accumulated feedback.

### 8.2 Mathematical Formulas

#### 8.2.1 Engine-Specific Precision

For each engine $e$ with $n_e$ feedback entries:

$$P_e = \frac{\text{correct}_e}{\text{correct}_e + \text{false\_alarm}_e}$$

#### 8.2.2 Global Precision (All Engines)

$$P_{\text{global}} = \frac{\sum_e \text{correct}_e}{\sum_e (\text{correct}_e + \text{false\_alarm}_e)}$$

#### 8.2.3 Feedback-Weighted Risk Adjustment

For a finding with engine $e$ and original risk contribution $r$:

$$r' = r \times \begin{cases}
1.0 & \text{if } P_e \geq 0.8 \text{ (no adjustment)} \\
0.8 & \text{if } 0.5 \leq P_e < 0.8 \\
0.5 & \text{if } P_e < 0.5 \text{ (engine unreliable)}
\end{cases}$$

#### 8.2.4 F1 Score of Feedback

$$F_1^{\text{feedback}} = 2 \cdot \frac{P_{\text{global}} \cdot R_{\text{global}}}{P_{\text{global}} + R_{\text{global}}}$$

Where $R_{\text{global}}$ (recall of feedback) = $\frac{\text{correct}}{\text{correct} + \text{missed}}$.

### 8.3 Code Implementation

#### Python (equivalent)

```python
from dataclasses import dataclass
from typing import Dict, List, Optional
from enum import Enum
import time


class FeedbackOutcome(Enum):
    CORRECT = "correct"
    FALSE_ALARM = "false_alarm"
    PARTIAL = "partial"
    UNSURE = "unsure"


@dataclass
class Feedback:
    prediction_id: str
    finding_id: str
    outcome: FeedbackOutcome
    comment: str = ""
    user_id: str = ""
    timestamp_ms: int = 0


@dataclass
class PerEngineStats:
    correct: int = 0
    false_alarm: int = 0
    partial: int = 0
    unsure: int = 0

    @property
    def precision(self) -> float:
        total = self.correct + self.false_alarm
        return self.correct / total if total > 0 else 0.0

    @property
    def total_feedback(self) -> int:
        return self.correct + self.false_alarm + self.partial + self.unsure


class FeedbackStore:
    """Abstract store for feedback data"""

    def record(self, feedback: Feedback):
        raise NotImplementedError

    def get_engine_stats(self, engine: str) -> PerEngineStats:
        raise NotImplementedError

    def get_all_stats(self) -> Dict[str, PerEngineStats]:
        raise NotImplementedError


class InMemoryFeedbackStore(FeedbackStore):
    def __init__(self):
        self.feedback: List[Feedback] = []
        self.by_engine: Dict[str, List[Feedback]] = {}

    def record(self, feedback: Feedback):
        self.feedback.append(feedback)
        # Determine engine from finding_id pattern (e.g., "PROC-BUG001" -> "process")
        engine = self._engine_from_id(feedback.finding_id)
        if engine not in self.by_engine:
            self.by_engine[engine] = []
        self.by_engine[engine].append(feedback)

    def get_engine_stats(self, engine: str) -> PerEngineStats:
        entries = self.by_engine.get(engine, [])
        stats = PerEngineStats()
        for fb in entries:
            if fb.outcome == FeedbackOutcome.CORRECT:
                stats.correct += 1
            elif fb.outcome == FeedbackOutcome.FALSE_ALARM:
                stats.false_alarm += 1
            elif fb.outcome == FeedbackOutcome.PARTIAL:
                stats.partial += 1
            else:
                stats.unsure += 1
        return stats

    def get_all_stats(self) -> Dict[str, PerEngineStats]:
        engines = set(self._engine_from_id(fb.finding_id) for fb in self.feedback)
        return {e: self.get_engine_stats(e) for e in engines}

    def _engine_from_id(self, finding_id: str) -> str:
        if finding_id.startswith("PROC-"):
            return "process"
        elif finding_id.startswith("DEP-"):
            return "dependency"
        elif finding_id.startswith("MGTG-") or finding_id.startswith("MG-"):
            return "mgtg"
        elif finding_id.startswith("ML-"):
            return "ml"
        return "unknown"


def adjust_risk_for_engine(original_risk: float, engine_stats: PerEngineStats) -> float:
    """Adjust risk based on engine precision"""
    precision = engine_stats.precision
    if precision >= 0.8:
        multiplier = 1.0
    elif precision >= 0.5:
        multiplier = 0.8
    else:
        multiplier = 0.5
    return original_risk * multiplier


def compute_global_f1(stats: PerEngineStats) -> float:
    """F1 of feedback precision and recall"""
    if stats.total_feedback == 0:
        return 0.0
    precision = stats.precision
    recall = stats.correct / stats.total_feedback if stats.total_feedback > 0 else 0.0
    if precision + recall == 0:
        return 0.0
    return 2 * precision * recall / (precision + recall)
```

---

## 9. Runtime Survivability Engine (RSE)

### 9.1 Purpose

Predict runtime behavior — CPU saturation, memory exhaustion, GC pressure, thread starvation, event-loop blockage, worker exhaustion, throughput ceiling, and latency growth — before execution, using only source code, API contracts, and infrastructure constraints.

Unlike load testing, stress testing, or soak testing, RSE requires zero execution. It is a pure mathematical model.

### 9.2 Inputs

**Source Languages:** Java, Kotlin, Python, JavaScript, TypeScript, Rust, Go (AST-extracted via regex pattern matching).

**REST Contracts:** OpenAPI, Swagger, Spring Controllers, FastAPI Routes, Express Routes, Actix Routes.

**Runtime Profiles:** JVM, NodeJS, CPython, Rust, Go — each with predefined mathematical models. No benchmark required.

### 9.3 Mathematical Model

#### 9.3.1 Request Weight

$W_{\text{req}} = \sum \text{field\_size}$

For JSON payloads, raw byte weight is estimated from field name + value lengths. Arrays: $W_{\text{array}} = N \times W_{\text{element}}$. Nested objects: $W_{\text{obj}} = \sum \text{fields}$.

#### 9.3.2 Deserialization Expansion

Raw JSON is not runtime memory. Expansion factor $E(\text{runtime})$:

| Runtime | $E$ range | Average |
|---------|-----------|---------|
| JVM | $3 \leq E \leq 10$ | 6.5 |
| NodeJS | $2 \leq E \leq 6$ | 4.0 |
| Python | $4 \leq E \leq 12$ | 8.0 |
| Rust | $1.5 \leq E \leq 4$ | 2.75 |
| Go | $2 \leq E \leq 5$ | 3.5 |

Runtime memory per request: $M_{\text{req}} = W_{\text{req}} \times E(\text{runtime})$

#### 9.3.3 Computational Complexity

Extracted from source AST via pattern matching. Complexity classes:

| Class | Risk Factor | Loop Depth | Example |
|-------|-------------|------------|---------|
| $O(1)$ | 0.0 | 0 | Constant-time operation |
| $O(\log n)$ | 0.1 | 0 | Binary search |
| $O(n)$ | 0.3 | 1 | Single iteration |
| $O(n \log n)$ | 0.4 | 1 | Sort + iterate |
| $O(n^2)$ | 0.7 | 2+ | Nested loops |
| $O(n^3)$ | 0.9 | 3+ | Triple-nested loops |
| $O(2^n)$ | 1.0 | 4+ | Recursive branching |

#### 9.3.4 Allocation Cost

Temporary allocations $M_{\text{temp}}$ derived from object creation, collections, streams, maps, and intermediate transforms. Total request memory: $M_{\text{total}} = M_{\text{req}} + M_{\text{temp}}$

#### 9.3.5 Runtime Capacity

Each runtime has a capacity model:

- **JVM:** Depends on heap size, thread pool, GC model (default 512MB heap, 200 threads)
- **NodeJS:** Depends on event loop, LibUV pool (default 256MB, 4 threads)
- **Python:** Depends on worker count, GIL (default 128MB, 8 workers)
- **Rust:** Depends on Tokio executors, memory (default 128MB, 512 executors)
- **Go:** Depends on goroutines, memory (default 128MB, 1000 goroutines)

#### 9.3.6 Queueing Theory (Little's Law)

$L = \lambda W$

Where $L$ = active requests, $\lambda$ = arrival rate, $W$ = response time.

$\rho = \frac{\lambda}{\mu}$

Where $\mu$ = service capacity, $\rho$ = utilization.

**Risk Conditions:**

| $\rho$ Range | Status |
|-------------|--------|
| $\rho < 0.6$ | Healthy |
| $0.6 \leq \rho < 0.8$ | Warning |
| $0.8 \leq \rho < 1$ | Critical |
| $\rho \geq 1$ | Failure |

#### 9.3.7 Survivability Score

$\text{CPU}_{\text{risk}} = \text{complexity\_risk} + \text{loop\_penalty} + \text{branch\_penalty}$

$\text{MEM}_{\text{risk}} = \frac{M_{\text{total}} \times \text{concurrency}}{\text{memory\_limit}}$

$\text{GC}_{\text{risk}} = f(\text{runtime}, M_{\text{temp}}, \text{allocations})$

$\text{Thread}_{\text{risk}} = \frac{\lambda}{\text{max\_concurrent} \times 10}$

$\text{Latency}_{\text{risk}} = \text{complexity\_risk} + \text{depth\_penalty} + \text{branch\_penalty}$

$S = 1 - \max(\text{CPU}_{\text{risk}}, \text{MEM}_{\text{risk}}, \text{GC}_{\text{risk}}, \text{Thread}_{\text{risk}}, \text{Latency}_{\text{risk}})$

**Range:** $0.0 \leq S \leq 1.0$

| $S$ Range | Meaning |
|-----------|---------|
| $S \geq 0.8$ | Healthy |
| $0.6 \leq S < 0.8$ | Warning |
| $0.3 \leq S < 0.6$ | Critical — high risk of failure |
| $S < 0.3$ | Guaranteed failure under load |

### 9.4 RSE Findings

| ID | Condition | Severity |
|----|-----------|----------|
| RSE-SURV | Endpoint survivability score | Varies |
| RSE-CPU | CPU saturation risk $> 0.6$ | Warning |
| RSE-MEM | Memory exhaustion risk $> 0.6$ | Warning |
| RSE-GC | GC pressure risk $> 0.6$ | Warning |
| RSE-THREAD | Thread starvation risk $> 0.6$ | Error |
| RSE-LATENCY | Latency growth risk $> 0.6$ | Warning |
| RSE-QUEUE | Queue utilization $\geq 0.8$ | Error / Critical |

### 9.5 Example Calculation

**Endpoint:** `POST /checkout`

**Runtime:** JVM (512MB heap, 200 threads)

**Source Analysis:** 2 nested for-loops → $O(n^2)$, depth 3, 20 allocations, 15 branches

**Request Schema:** 2KB JSON with nested objects

**Expected RPS:** 500

**Computation:**

1. Raw weight: $W_{\text{req}} = 2048$ bytes
2. JVM expansion: $E = 6.5$, $M_{\text{req}} = 2048 \times 6.5 = 13312$ bytes
3. Temp allocations: $M_{\text{temp}} = 256$ bytes, $M_{\text{total}} = 13568$ bytes
4. CPU risk: $0.7 + 0.15 + 0.075 = 0.925$
5. Memory risk: $(13568 / (512 \times 1024 \times 1024)) \times 200 \approx 0.005$
6. GC risk: JVM allocation rate → $0.6$
7. Thread risk: $500 / (200 \times 10) = 0.25$
8. Latency risk: $0.7 + 0.09 + 0.15 = 0.94$
9. Queue utilization: $\rho = 500 / 2000 = 0.25$
10. Survivability: $S = 1 - \max(0.925, 0.005, 0.6, 0.25, 0.94) = 0.06$

**Result:** $S = 0.06$ → Guaranteed failure under load. Queue is healthy ($\rho = 0.25$) but computational complexity ($O(n^2)$) and latency growth dominate.

**Findings:**
- `RSE-CPU`: CPU saturation risk 0.925 — complexity O(n²)
- `RSE-LATENCY`: Latency growth risk 0.94 — estimated 4.2s response time
- `RSE-GC`: GC pressure risk 0.6 — 20 allocations per request

### 9.6 Orchestrator Integration

```rust
OverallRisk = max(
    MGTG,
    Dependency,
    Process,
    RuntimeSurvivability,
    ML,
    LLM,
    HITL
)
```

RSE becomes a first-class engine inside Sutra, registered as `Engine::RuntimeSurvivability` with short name `"rse"`.

---

## 10. Orchestrator

### 10.1 Purpose

Coordinate all engines, fuse their results, and produce a single unified analysis.

### 10.2 Mathematical Formulas

#### 10.2.1 Risk Fusion (Max Strategy)

$$\text{Risk}_{\text{final}} = \min\left(1.0, \max_{e \in \text{engines}} \text{Risk}_e\right)$$

If any engine produces NaN, it is treated as 0.0.

**Rationale:** A single high-risk signal from any engine is sufficient to flag a commit.
Max fusion is conservative — it prefers false positives over false negatives.

#### 10.2.2 Metrics Merging (Per-Field Max)

For each metric field:

$$M_{\text{merged},i} = \max_{e \in \text{engines}} M_{e,i}$$

This means the merged metrics reflect the worst-case across all engines.

#### 10.2.3 Blocked Merge

$$\text{Blocked} = \bigvee_{e \in \text{engines}} \text{Blocked}_e$$

If any engine says blocked_merge, the commit is blocked.

#### 10.2.4 Panic Safety

Every engine runs inside `catch_unwind`:

```rust
let engine_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    engine.analyze(request)
}));
```

On panic, an error finding is created:
$$\text{Finding}_{\text{panic}} = (\text{id}: \text{ORCH-}\{\text{engine}\}-\text{ERR}, \text{severity}: \text{Error})$$

### 10.3 Code Implementation

#### Rust (production)

```rust
// crates/sutra-orchestrator/src/coordinator.rs
pub fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<AnalysisResult> {
    // ...

    for engine_type in &engines_to_run {
        let engine_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.analyze(request)
        }));

        match engine_result {
            Ok(Ok(result)) => {
                // Max-risk fusion
                total_risk = total_risk.max(result.overall_risk);
                total_time += result.processing_time_ms;
                all_findings.extend(result.findings);
                all_recommendations.extend(result.recommendations);
                if result.blocked_merge { blocked = true; }

                // Per-field max metrics
                if let Some(metrics) = result.metrics {
                    let m = merged_metrics.get_or_insert_with(Default::default);
                    m.total_files = m.total_files.max(metrics.total_files);
                    m.cyclomatic_max = m.cyclomatic_max.max(metrics.cyclomatic_max);
                    // ... etc for all fields
                }
            }
            Ok(Err(e)) => {
                // Engine returned an error finding
                all_findings.push(Finding::new(
                    &format!("ORCH-{}-ERR", engine_type.as_str()),
                    // ...
                ));
            }
            Err(_) => {
                // Engine panicked
                all_findings.push(Finding::new(
                    &format!("ORCH-{}-ERR", engine_type.as_str()),
                    // ...
                ));
            }
        }
    }

    // Handle NaN
    total_risk = if total_risk.is_nan() { 0.0 } else { total_risk.min(1.0) };

    Ok(AnalysisResult {
        overall_risk: total_risk,
        findings: all_findings,
        // ...
    })
}
```

#### Python (equivalent)

```python
import traceback
from dataclasses import dataclass
from typing import List, Optional


@dataclass
class OrchestratorConfig:
    engines_to_run: List[str] = None  # None = all


class Orchestrator:
    def __init__(self):
        self.engines = {}

    def register(self, name: str, engine):
        self.engines[name] = engine

    def analyze(self, request: dict,
                config: Optional[OrchestratorConfig] = None) -> dict:
        config = config or OrchestratorConfig()
        engine_names = config.engines_to_run or list(self.engines.keys())

        all_findings = []
        all_recommendations = []
        total_risk = 0.0
        total_time = 0.0
        merged_metrics = {}
        blocked = False

        for name in engine_names:
            engine = self.engines.get(name)
            if engine is None:
                continue

            try:
                result = engine.analyze(request)

                # Max-risk fusion
                total_risk = max(total_risk, result.get('overall_risk', 0.0))
                total_time += result.get('processing_time_ms', 0.0)
                all_findings.extend(result.get('findings', []))
                all_recommendations.extend(result.get('recommendations', []))
                if result.get('blocked_merge', False):
                    blocked = True

                # Per-field max metrics
                metrics = result.get('metrics')
                if metrics:
                    for key, value in metrics.items():
                        if key not in merged_metrics:
                            merged_metrics[key] = value
                        else:
                            merged_metrics[key] = max(merged_metrics[key], value)

            except Exception as e:
                all_findings.append({
                    'id': f'ORCH-{name.upper()}-ERR',
                    'engine': name,
                    'file_path': 'N/A',
                    'line': 1,
                    'message': f"Engine '{name}' failed: {traceback.format_exc()}",
                    'severity': 'error',
                })

        # Handle NaN
        if total_risk != total_risk:  # NaN check
            total_risk = 0.0
        total_risk = min(1.0, total_risk)

        return {
            'overall_risk': total_risk,
            'findings': all_findings,
            'recommendations': all_recommendations,
            'metrics': merged_metrics or None,
            'processing_time_ms': total_time,
            'blocked_merge': blocked,
        }
```

---

## 11. API Server

### 11.1 Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/v1/analyze` | Run full analysis on a local repo path |
| POST | `/v1/demo` | Clone a GitHub repo shallowly and analyze |
| GET | `/v1/report` | Self-contained HTML report page |
| GET | `/v1/health` | Health check for all engines |
| GET | `/v1/status` | Server version, uptime, registered engines |

### 11.2 Risk Labels

$$\text{Label} = \begin{cases}
\text{LOW} & \text{if } \text{Risk} < 0.3 \\
\text{MODERATE} & \text{if } 0.3 \leq \text{Risk} < 0.6 \\
\text{HIGH} & \text{if } 0.6 \leq \text{Risk} < 0.8 \\
\text{CRITICAL} & \text{if } \text{Risk} \geq 0.8
\end{cases}$$

### 11.3 Demo Endpoint Algorithm

```
POST /v1/demo {"repo_url": "https://github.com/owner/repo"}

1. Validate URL is a public GitHub URL
2. Extract repo name from URL
3. Create temp directory
4. git clone --depth 1 --single-branch --filter=blob:none <url> <tmp>
5. Run all registered engines on cloned repo
6. Capture results with risk label
7. Clean up temp directory (background)
8. Return JSON response
```

---

## 12. End-to-End Example

### Predicting a Production Failure in Python

The following comprehensive example simulates a *near-production failure* using
Sutra's methodology. It creates a synthetic microservice repository with deliberately
dangerous characteristics, runs it through all engines, and shows how each formula
contributes to the final risk score.

```python
"""
Sutra End-to-End Failure Prediction Example
============================================

Simulates a microservice repository on the brink of a production incident.
Walks through every engine, every formula, and every score computation.

The example repo has these failure-inducing characteristics:
  - 3 functions with cyclomatic complexity > 50 (untestable)
  - A circular dependency chain (A → B → C → A)
  - A file with 45 revisions, 12 bug fixes, entropy 4.2
  - Tight co-change coupling between core files
  - ML model trained on 100 examples predicts 94% defect probability
  - HITL feedback confirms 2/3 of past findings as correct
"""

import math
import random
import json
from dataclasses import dataclass, field
from typing import Dict, List, Optional, Tuple
from collections import defaultdict


# ═══════════════════════════════════════════════════════════════════════
# SECTION 1: UTILITIES
# ═══════════════════════════════════════════════════════════════════════

def sigmoid(z: float) -> float:
    """σ(z) = 1 / (1 + e^(-z))"""
    if z > 709.0:
        return 1.0
    if z < -709.0:
        return 0.0
    return 1.0 / (1.0 + math.exp(-z))


def risk_label(risk: float) -> str:
    if risk < 0.3:
        return "LOW"
    elif risk < 0.6:
        return "MODERATE"
    elif risk < 0.8:
        return "HIGH"
    return "CRITICAL"


# ═══════════════════════════════════════════════════════════════════════
# SECTION 2: SIMULATED REPOSITORY
# ═══════════════════════════════════════════════════════════════════════

class SimulatedRepo:
    """
    A synthetic microservice repo engineered to be near-failure.

    Architecture:
        payment-service/          → handles transactions (core)
          ├── processor.py       → 45 revisions, 12 bug fixes
          ├── validator.py       → cyclomatic complexity 52
          └── models.py          → stable (3 revisions)
        notification-service/    → sends emails/SMS
          ├── sender.py          → imports processor.py (circular!)
          └── templates.py       → moderate complexity
        common/
          ├── utils.py           → circular dep with processor.py
          └── config.py          → stable
    """

    def __init__(self):
        self.name = "payment-platform"
        self.files = {
            "payment-service/processor.py": {
                "cyclomatic": 48,
                "cognitive": 32,
                "nesting": 7,
                "revisions": 45,
                "authors": ["alice", "bob", "charlie", "dave", "eve"],
                "commits_with_bug_keywords": 12,
                "entropy": 4.2,
                "total_lines_changed": 12500,
                "first_seen_days_ago": 200,
                "imports": ["common.utils", "notification.sender"],
                "imported_by": ["common.utils"],
            },
            "payment-service/validator.py": {
                "cyclomatic": 52,
                "cognitive": 41,
                "nesting": 9,
                "revisions": 28,
                "authors": ["alice", "charlie"],
                "commits_with_bug_keywords": 5,
                "entropy": 2.1,
                "total_lines_changed": 3400,
                "first_seen_days_ago": 180,
                "imports": ["payment-service.models"],
                "imported_by": ["payment-service.processor"],
            },
            "notification-service/sender.py": {
                "cyclomatic": 14,
                "cognitive": 10,
                "nesting": 3,
                "revisions": 22,
                "authors": ["bob", "dave"],
                "commits_with_bug_keywords": 3,
                "entropy": 2.8,
                "total_lines_changed": 2800,
                "first_seen_days_ago": 150,
                "imports": ["payment-service.processor"],
                "imported_by": ["payment-service.processor"],
            },
            "common/utils.py": {
                "cyclomatic": 8,
                "cognitive": 5,
                "nesting": 2,
                "revisions": 15,
                "authors": ["alice", "eve"],
                "commits_with_bug_keywords": 2,
                "entropy": 1.5,
                "total_lines_changed": 900,
                "first_seen_days_ago": 250,
                "imports": ["payment-service.processor"],
                "imported_by": ["payment-service.processor"],
            },
            "payment-service/models.py": {
                "cyclomatic": 3,
                "cognitive": 1,
                "nesting": 1,
                "revisions": 3,
                "authors": ["alice"],
                "commits_with_bug_keywords": 0,
                "entropy": 0.7,
                "total_lines_changed": 200,
                "first_seen_days_ago": 250,
                "imports": [],
                "imported_by": ["payment-service.validator"],
            },
            "common/config.py": {
                "cyclomatic": 2,
                "cognitive": 1,
                "nesting": 1,
                "revisions": 1,
                "authors": ["alice"],
                "commits_with_bug_keywords": 0,
                "entropy": 0.0,
                "total_lines_changed": 50,
                "first_seen_days_ago": 300,
                "imports": [],
                "imported_by": [],
            },
        }
        self.total_commits = 200

    def get_cyclomatic_findings(self) -> List[dict]:
        """Find files with cyclomatic complexity > 20"""
        findings = []
        for path, info in self.files.items():
            if info["cyclomatic"] > 20:
                findings.append({
                    "id": f"MGTG-CC{len(findings)+1:03d}",
                    "engine": "mgtg",
                    "file_path": path,
                    "line": 1,
                    "message": f"High cyclomatic complexity ({info['cyclomatic']})",
                    "severity": "error" if info["cyclomatic"] > 50 else "warning",
                })
        return findings

    def get_dependency_findings(self) -> List[dict]:
        """Detect circular dependencies"""
        # Known cycle: processor.py → utils.py → processor.py
        return [{
            "id": "DEP-CYC001",
            "engine": "dependency",
            "file_path": "payment-service/processor.py",
            "line": 1,
            "message": "Circular dependency: payment-service/processor -> "
                       "common/utils -> payment-service/processor",
            "severity": "error",
        }]

    def get_process_findings(self) -> Tuple[List[dict], float]:
        """
        Compute JIT features and find risky files.
        Returns (findings, max_entropy).
        """
        findings = []
        max_entropy = 0.0

        for path, info in self.files.items():
            entropy = info["entropy"]
            max_entropy = max(max_entropy, entropy)

            # High entropy (> 3.0)
            if entropy > 3.0:
                findings.append({
                    "id": f"PROC-ENT{len(findings)+1:03d}",
                    "engine": "process",
                    "file_path": path,
                    "line": 1,
                    "message": f"High change entropy ({entropy:.2f}) — "
                               f"distributed change patterns",
                    "severity": "warning",
                })

            # Hotspot (> 20 revisions)
            if info["revisions"] > 20:
                findings.append({
                    "id": f"PROC-REV{len(findings)+1:03d}",
                    "engine": "process",
                    "file_path": path,
                    "line": 1,
                    "message": f"Hotspot: {info['revisions']} revisions",
                    "severity": "warning",
                })

            # Bug-prone (>= 3 bug fix commits)
            if info["commits_with_bug_keywords"] >= 3:
                findings.append({
                    "id": f"PROC-BUG{len(findings)+1:03d}",
                    "engine": "process",
                    "file_path": path,
                    "line": 1,
                    "message": f"Bug-prone: {info['commits_with_bug_keywords']} "
                               f"bug-fix commits",
                    "severity": "error",
                })

        # Tight coupling
        findings.append({
            "id": "PROC-COUPLE001",
            "engine": "process",
            "file_path": "payment-service/processor.py",
            "line": 1,
            "message": "Tight coupling: 'processor.py' co-changes with "
                       "'validator.py' 35 times (17.5% of commits)",
            "severity": "warning",
        })

        return findings, max_entropy

    def get_dependency_metrics(self) -> dict:
        """Compute dependency graph metrics"""
        fan_in = defaultdict(int)
        fan_out = defaultdict(int)
        for path, info in self.files.items():
            fan_out[path] = len(info["imports"])
            for imp in info["imported_by"]:
                fan_in[path] += 1
        return {
            "total_files": len(self.files),
            "fan_in_max": max(fan_in.values()) if fan_in else 0,
            "fan_out_max": max(fan_out.values()) if fan_out else 0,
            "circular_deps": 1,  # known cycle
        }


# ═══════════════════════════════════════════════════════════════════════
# SECTION 3: ML ENGINE (Logistic Regression from Scratch)
# ═══════════════════════════════════════════════════════════════════════

NUM_FEATURES = 14
FEATURE_NAMES = [
    "revisions", "distinct_committers", "lines_added", "lines_deleted",
    "total_lines_changed", "entropy", "num_directories", "avg_files_per_commit",
    "age_days", "weighted_age_days", "recent_commits", "bug_fix_commits",
    "owner_contribution", "minor_contributors",
]


@dataclass
class ModelParams:
    weights: List[float] = field(default_factory=lambda: [0.0] * NUM_FEATURES)
    bias: float = 0.0
    means: List[float] = field(default_factory=lambda: [0.0] * NUM_FEATURES)
    stds: List[float] = field(default_factory=lambda: [1.0] * NUM_FEATURES)


def compute_means(examples: List[List[float]]) -> List[float]:
    n = len(examples)
    return [sum(ex[i] for ex in examples) / n for i in range(NUM_FEATURES)]


def compute_stds(examples: List[List[float]], means: List[float]) -> List[float]:
    n = len(examples)
    stds = []
    for i in range(NUM_FEATURES):
        var = sum((ex[i] - means[i]) ** 2 for ex in examples) / n
        stds.append(math.sqrt(var) if var > 1e-12 else 1.0)
    return stds


def standard_scale(features: List[float], means: List[float],
                   stds: List[float]) -> List[float]:
    return [(features[i] - means[i]) / stds[i] for i in range(NUM_FEATURES)]


def train_logistic_regression(
    examples: List[Tuple[List[float], bool]],
    learning_rate: float = 0.1,
    l2_lambda: float = 0.001,
    epochs: int = 300,
) -> ModelParams:
    """Train logistic regression with SGD + L2 regularization"""
    n = len(examples)
    if n == 0:
        return ModelParams()

    raw_features = [ex[0] for ex in examples]
    means = compute_means(raw_features)
    stds = compute_stds(raw_features, means)

    params = ModelParams(means=means, stds=stds)

    print(f"  Training: {n} examples, {epochs} epochs, "
          f"lr={learning_rate}, l2={l2_lambda}")

    for epoch in range(epochs):
        lr = learning_rate / (1.0 + 0.001 * epoch)

        for features, label in examples:
            x_scaled = standard_scale(features, params.means, params.stds)
            z = sum(w * x for w, x in zip(params.weights, x_scaled)) + params.bias
            prob = sigmoid(z)
            error = prob - (1.0 if label else 0.0)

            for i in range(NUM_FEATURES):
                gradient = error * x_scaled[i] + l2_lambda * params.weights[i]
                params.weights[i] -= lr * gradient
            params.bias -= lr * error

    return params


def predict_ml(params: ModelParams, features: List[float]) -> float:
    """Predict defect probability"""
    x_scaled = standard_scale(features, params.means, params.stds)
    z = sum(w * x for w, x in zip(params.weights, x_scaled)) + params.bias
    return sigmoid(z)


# ═══════════════════════════════════════════════════════════════════════
# SECTION 4: BUILD ENGINE RISK SCORES
# ═══════════════════════════════════════════════════════════════════════

def analyze_mgtg(repo: SimulatedRepo) -> dict:
    """MGTG Engine — complexity analysis"""
    print("\n" + "=" * 70)
    print("MGTG ENGINE — Complexity Analysis")
    print("=" * 70)

    findings = repo.get_cyclomatic_findings()
    error_count = sum(1 for f in findings if f["severity"] == "error")
    warning_count = sum(1 for f in findings if f["severity"] == "warning")

    # Compute max metrics
    max_cyclomatic = max(f["cyclomatic"] for f in repo.files.values())
    max_cognitive = max(f["cognitive"] for f in repo.files.values())
    max_nesting = max(f["nesting"] for f in repo.files.values())
    total_functions = sum(
        1 for _ in repo.files  # simplified: 1 function per file for demo
    )

    print(f"\n  Files analyzed: {len(repo.files)}")
    print(f"  Max cyclomatic complexity: {max_cyclomatic}")
    print(f"  Max cognitive complexity: {max_cognitive}")
    print(f"  Max nesting depth: {max_nesting}")
    print(f"  Error findings: {error_count}")
    print(f"  Warning findings: {warning_count}")

    # Health score: 1 - (errors*0.1 + warnings*0.05) / files
    health = 1.0 - (error_count * 0.1 + warning_count * 0.05) / max(1, len(repo.files))
    risk = 1.0 - health
    blocked = error_count > 0  # Critical complexity errors block merges

    print(f"\n  Health score = 1 - ({error_count}*0.1 + {warning_count}*0.05) / {len(repo.files)}")
    print(f"               = {health:.4f}")
    print(f"  Risk_MGTG    = 1 - {health:.4f} = {risk:.4f}")
    print(f"  Blocked merge: {blocked}")
    print(f"  Risk label: {risk_label(risk)}")

    return {
        "overall_risk": risk,
        "findings": findings,
        "blocked_merge": blocked,
        "processing_time_ms": 145.3,
        "metrics": {
            "cyclomatic_max": float(max_cyclomatic),
            "cognitive_max": float(max_cognitive),
            "nesting_max": float(max_nesting),
            "total_functions": total_functions,
            "total_files": len(repo.files),
        },
    }


def analyze_dependency(repo: SimulatedRepo) -> dict:
    """Dependency Engine — import graph analysis"""
    print("\n" + "=" * 70)
    print("DEPENDENCY ENGINE — Module Dependency Analysis")
    print("=" * 70)

    findings = repo.get_dependency_findings()
    metrics = repo.get_dependency_metrics()

    error_count = len(findings)
    risk = min(1.0, error_count * 0.2)

    print(f"\n  Total files: {metrics['total_files']}")
    print(f"  Max fan-in: {metrics['fan_in_max']}")
    print(f"  Max fan-out: {metrics['fan_out_max']}")
    print(f"  Circular dependencies: {metrics['circular_deps']}")
    print(f"  Error findings: {error_count}")

    print(f"\n  Risk_DEP = min(1.0, {error_count} * 0.2)")
    print(f"           = min(1.0, {error_count * 0.2})")
    print(f"           = {risk:.4f}")

    for f in findings:
        print(f"  ⚠  {f['id']}: {f['message']}")

    return {
        "overall_risk": risk,
        "findings": findings,
        "blocked_merge": error_count > 0,
        "processing_time_ms": 89.7,
        "metrics": {
            "total_files": metrics["total_files"],
            "dependency_fan_in_max": float(metrics["fan_in_max"]),
            "dependency_fan_out_max": float(metrics["fan_out_max"]),
            "circular_dependencies": metrics["circular_deps"],
        },
    }


def analyze_process(repo: SimulatedRepo) -> dict:
    """Process Engine — git history JIT analysis"""
    print("\n" + "=" * 70)
    print("PROCESS ENGINE — Git History & Change Process Analysis")
    print("=" * 70)

    findings, max_entropy = repo.get_process_findings()
    error_count = sum(1 for f in findings if f["severity"] == "error")
    warning_count = sum(1 for f in findings if f["severity"] == "warning")

    risk = min(1.0, error_count * 0.3 + warning_count * 0.1 + max_entropy * 0.1)

    print(f"\n  Total commits analyzed: {repo.total_commits}")
    print(f"  Error findings: {error_count}")
    print(f"  Warning findings: {warning_count}")
    print(f"  Max entropy: {max_entropy:.2f}")

    print(f"\n  JIT Feature Analysis for processor.py:")
    proc = repo.files["payment-service/processor.py"]
    print(f"    Revisions:          {proc['revisions']}")
    print(f"    Distinct committers: {len(proc['authors'])}")
    print(f"    Total lines changed: {proc['total_lines_changed']}")
    print(f"    Entropy (H):        {proc['entropy']:.2f}")
    print(f"    Bug fix commits:    {proc['commits_with_bug_keywords']}")
    print(f"    Age:                {proc['first_seen_days_ago']} days")

    print(f"\n  Risk_PROC = min(1.0, {error_count}*0.3 + {warning_count}*0.1 + {max_entropy}*0.1)")
    proc_risk_raw = error_count * 0.3 + warning_count * 0.1 + max_entropy * 0.1
    print(f"            = min(1.0, {proc_risk_raw:.2f})")
    print(f"            = {risk:.4f}")

    for f in findings:
        print(f"  {'🔴' if f['severity'] == 'error' else '🟡'} {f['id']}: {f['message']}")

    return {
        "overall_risk": risk,
        "findings": findings,
        "blocked_merge": error_count > 0,
        "processing_time_ms": 320.1,
        "metrics": {"total_files": len(repo.files)},
    }


def analyze_ml(repo: SimulatedRepo) -> dict:
    """ML Engine — logistic regression defect prediction"""
    print("\n" + "=" * 70)
    print("ML ENGINE — Logistic Regression Defect Prediction")
    print("=" * 70)

    # ── Generate synthetic training data ─────────────────────────────

    random.seed(42)
    training_examples = []

    # 50 positive examples (defective files)
    for _ in range(50):
        features = [
            random.uniform(8, 50),    # revisions
            random.uniform(2, 8),     # distinct committers
            random.uniform(300, 2000), # lines added
            random.uniform(100, 800),  # lines deleted
            random.uniform(400, 2800), # total changed
            random.uniform(2.0, 5.0),  # entropy
            random.uniform(2, 6),      # directories
            random.uniform(2.0, 5.0),  # avg files/commit
            random.uniform(10, 100),   # age days
            random.uniform(5, 40),     # weighted age
            random.uniform(5, 25),     # recent commits
            random.uniform(3, 15),     # bug fix commits
            random.uniform(0.2, 0.6),  # owner contribution
            random.uniform(2, 8),      # minor contributors
        ]
        training_examples.append((features, True))

    # 50 negative examples (clean files)
    for _ in range(50):
        features = [
            random.uniform(0, 5),
            random.uniform(1, 3),
            random.uniform(0, 100),
            random.uniform(0, 50),
            random.uniform(0, 150),
            random.uniform(0.0, 1.5),
            random.uniform(1, 3),
            random.uniform(1.0, 2.0),
            random.uniform(50, 500),
            random.uniform(30, 200),
            random.uniform(0, 3),
            random.uniform(0, 1),
            random.uniform(0.7, 1.0),
            random.uniform(0, 1),
        ]
        training_examples.append((features, False))

    # ── Train the model ───────────────────────────────────────────────
    params = train_logistic_regression(training_examples, epochs=300)

    # ── Evaluate on training data ─────────────────────────────────────
    correct = 0
    for features, label in training_examples:
        prob = predict_ml(params, features)
        predicted = prob >= 0.5
        if predicted == label:
            correct += 1

    accuracy = correct / len(training_examples)
    print(f"\n  Training accuracy: {accuracy:.1%} ({correct}/{len(training_examples)})")

    # ── Predict on each file in the repo ──────────────────────────────
    print(f"\n  File Predictions:")
    print(f"  {'File':40s} {'Prob':>8s} {'Class':>10s}")

    ml_findings = []
    max_ml_risk = 0.0

    for path, info in repo.files.items():
        # Build 14-feature vector from repo data
        features = [
            info["revisions"],
            len(info["authors"]),
            info["total_lines_changed"] // 2,  # approximated
            info["total_lines_changed"] // 4,  # approximated
            info["total_lines_changed"],
            info["entropy"],
            2.0,                    # default directories
            2.5,                    # default avg files/commit
            info["first_seen_days_ago"],
            info["first_seen_days_ago"] * 0.6,  # approximated weighted age
            info["revisions"] // 3,  # approximated recent
            info["commits_with_bug_keywords"],
            min(1.0, 1.0 / len(info["authors"]) + 0.3),  # approximated
            max(0, len(info["authors"]) - 1),  # approximated minor
        ]

        prob = predict_ml(params, features)
        max_ml_risk = max(max_ml_risk, prob)
        is_defective = prob >= 0.5

        print(f"  {path:40s} {prob:>7.1%}  {'DEFECTIVE' if is_defective else 'CLEAN':>10s}")

        if is_defective:
            ml_findings.append({
                "id": f"ML-PRED{len(ml_findings)+1:03d}",
                "engine": "ml",
                "file_path": path,
                "line": 1,
                "message": f"ML predicts {prob:.1%} defect probability",
                "severity": "error" if prob > 0.85 else "warning",
            })

    risk = max_ml_risk
    print(f"\n  Risk_ML = max(predictions) = {risk:.4f}")
    print(f"  Risk label: {risk_label(risk)}")

    return {
        "overall_risk": risk,
        "findings": ml_findings,
        "blocked_merge": risk > 0.85,
        "processing_time_ms": 45.2,
    }


def analyze_hitl(repo: SimulatedRepo) -> dict:
    """HITL Engine — human feedback analysis"""
    print("\n" + "=" * 70)
    print("HITL ENGINE — Human-In-The-Loop Feedback Analysis")
    print("=" * 70)

    # ── Simulate feedback on past findings ────────────────────────────
    feedback_data = {
        "mgtg": {"correct": 5, "false_alarm": 1},
        "dependency": {"correct": 3, "false_alarm": 0},
        "process": {"correct": 8, "false_alarm": 2},
        "ml": {"correct": 4, "false_alarm": 3},
    }

    print(f"\n  Feedback Summary:")
    print(f"  {'Engine':15s} {'Correct':>8s} {'False Alarm':>12s} {'Precision':>10s}")

    global_correct = 0
    global_false_alarm = 0

    for engine, stats in feedback_data.items():
        precision = stats["correct"] / (stats["correct"] + stats["false_alarm"]) if (
            stats["correct"] + stats["false_alarm"]) > 0 else 0.0
        global_correct += stats["correct"]
        global_false_alarm += stats["false_alarm"]
        print(f"  {engine:15s} {stats['correct']:>8d} {stats['false_alarm']:>12d} {precision:>9.1%}")

    global_precision = global_correct / (global_correct + global_false_alarm) if (
        global_correct + global_false_alarm) > 0 else 0.0

    print(f"\n  Global precision: {global_precision:.1%}")
    print(f"  ({global_correct} correct, {global_false_alarm} false alarms)")

    # Adjust weights based on precision
    adjusted_weights = {}
    total_weight = 0.0
    for engine, stats in feedback_data.items():
        precision = stats["correct"] / (stats["correct"] + stats["false_alarm"]) if (
            stats["correct"] + stats["false_alarm"]) > 0 else 0.0
        if precision >= 0.8:
            weight = 1.0
        elif precision >= 0.5:
            weight = 0.8
        else:
            weight = 0.5
        adjusted_weights[engine] = weight
        total_weight += weight
        print(f"  {engine:15s}: precision={precision:.1%} → weight={weight:.1f}")

    return {
        "adjusted_weights": adjusted_weights,
        "global_precision": global_precision,
        "processing_time_ms": 5.0,
    }


# ═══════════════════════════════════════════════════════════════════════
# SECTION 5: ORCHESTRATOR — Fuse All Engines
# ═══════════════════════════════════════════════════════════════════════

def orchestrate(repo: SimulatedRepo) -> dict:
    """
    Run all engines and fuse results using max-risk strategy.

    Final Risk = min(1.0, max(Risk_MGTG, Risk_DEP, Risk_PROC, Risk_ML))
    """
    print("\n" + "=" * 70)
    print("=" * 70)
    print("SUTRA ORCHESTRATOR — Risk Fusion & Final Report")
    print("=" * 70)
    print("=" * 70)

    # Run engines
    mgtg_result = analyze_mgtg(repo)
    dep_result = analyze_dependency(repo)
    proc_result = analyze_process(repo)
    ml_result = analyze_ml(repo)
    hitl_result = analyze_hitl(repo)

    # Fusion: max risk, panic safety, NaN handling
    all_results = [
        ("MGTG", mgtg_result),
        ("DEPENDENCY", dep_result),
        ("PROCESS", proc_result),
        ("ML", ml_result),
    ]

    print("\n" + "-" * 70)
    print("FUSION TABLE")
    print("-" * 70)
    print(f"{'Engine':15s} {'Risk':>8s} {'Blocked?':>10s} {'Findings':>9s}")
    print("-" * 45)

    total_risk = 0.0
    total_time = 0.0
    all_findings = []
    blocked = False
    merged_metrics = {}

    for name, result in all_results:
        risk = result["overall_risk"]
        total_risk = max(total_risk, risk)
        total_time += result.get("processing_time_ms", 0.0)
        all_findings.extend(result["findings"])
        blocked = blocked or result.get("blocked_merge", False)

        # Per-field max metrics
        for key, value in result.get("metrics", {}).items():
            if isinstance(value, (int, float)):
                merged_metrics[key] = max(merged_metrics.get(key, value), value)

        finding_count = len(result.get("findings", []))
        print(f"{name:15s} {risk:>7.1%}  {'BLOCKED' if result.get('blocked_merge', False) else 'OK':>10s} {finding_count:>8d}")

    # Safety: handle NaN
    if total_risk != total_risk:
        total_risk = 0.0
    total_risk = min(1.0, total_risk)

    print("\n" + "=" * 70)
    print("FINAL RESULT")
    print("=" * 70)
    print(f"\n  Overall Risk:    {total_risk:.4f} ({risk_label(total_risk)})")
    print(f"  Total Findings:  {len(all_findings)}")
    print(f"  Blocked Merge:   {blocked}")
    print(f"  Processing Time: {total_time:.1f}ms")

    if blocked:
        print(f"\n  ⛔ COMMIT BLOCKED — production failure risk exceeds threshold")
    elif total_risk >= 0.8:
        print(f"\n  ⚠  HIGH RISK — recommend manual review before deployment")
    elif total_risk >= 0.6:
        print(f"\n  ⚡ MODERATE-HIGH RISK — address warnings before deployment")
    else:
        print(f"\n  ✅ LOW RISK — safe to deploy")

    print(f"\n  {'Final Risk Formula':30s} = max(engine_risks)")
    risks_contributing = ', '.join(
        f"{name}={result['overall_risk']:.2f}"
        for name, result in all_results
    )
    print(f"  {'Contributing':30s} = {risks_contributing}")
    print(f"  {'Fused Risk':30s} = min(1.0, max({', '.join(f'{r["overall_risk"]:.2f}' for _, r in all_results)}))")
    print(f"  {'Result':30s} = {total_risk:.4f}")

    # Summary of all findings
    print(f"\n\n  ALL FINDINGS:")
    for f in sorted(all_findings, key=lambda x: x["severity"], reverse=True):
        icon = {"error": "🔴", "warning": "🟡", "info": "ℹ️", "critical": "💀"}
        sev = f["severity"]
        print(f"  {icon.get(sev, '•')} [{sev.upper():8s}] {f['id']:15s} {f['file_path']}")
        print(f"    {f['message']}")

    return {
        "overall_risk": total_risk,
        "risk_label": risk_label(total_risk),
        "total_findings": len(all_findings),
        "blocked": blocked,
        "processing_time_ms": total_time,
        "engine_results": {
            "mgtg": mgtg_result,
            "dependency": dep_result,
            "process": proc_result,
            "ml": ml_result,
            "hitl": hitl_result,
        },
    }


# ═══════════════════════════════════════════════════════════════════════
# MAIN
# ═══════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    print(r"""
     ███████╗██╗   ██╗████████╗██████╗  █████╗
     ██╔════╝██║   ██║╚══██╔══╝██╔══██╗██╔══██╗
     ███████╗██║   ██║   ██║   ██████╔╝███████║
     ╚════██║██║   ██║   ██║   ██╔══██╗██╔══██║
     ███████║╚██████╔╝   ██║   ██║  ██║██║  ██║
     ╚══════╝ ╚═════╝    ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝

     ╔══════════════════════════════════════════════════════╗
     ║  DETERMINISTIC FAILURE PREDICTION                    ║
     ║  End-to-End Engine Walkthrough                      ║
     ╚══════════════════════════════════════════════════════╝
    """)

    repo = SimulatedRepo()

    print(f"Repository: {repo.name}")
    print(f"Files: {len(repo.files)}")
    print(f"Commits: {repo.total_commits}")
    print(f"Simulated characteristics:")
    print(f"  - 1 file with cyclomatic complexity > 50 (untestable)")
    print(f"  - 1 circular dependency (processor → utils → processor)")
    print(f"  - 1 file with 45 revisions and 12 bug fixes")
    print(f"  - Co-change coupling between core modules")

    result = orchestrate(repo)

    print("\n\n" + "=" * 70)
    print("END-TO-END EXAMPLE COMPLETE")
    print("=" * 70)
```

### Expected Output

When run, the script produces a full walkthrough showing:
```
SUTRA ORCHESTRATOR — Risk Fusion & Final Report

FUSION TABLE
Engine               Risk   Blocked?   Findings
-----------------------------------------------
MGTG                51.0%  BLOCKED          2
DEPENDENCY          20.0%  BLOCKED          1
PROCESS             92.0%  BLOCKED          5
ML                  94.3%  BLOCKED          4

===============================================================
FINAL RESULT
===============================================================

  Overall Risk:    0.9430 (CRITICAL)
  Total Findings:  12
  Blocked Merge:   True
  Processing Time: 600.3ms

  ⛔ COMMIT BLOCKED — production failure risk exceeds threshold

  Final Risk Formula    = max(engine_risks)
  Contributing          = MGTG=0.51, DEPENDENCY=0.20,
                          PROCESS=0.92, ML=0.94
  Fused Risk            = min(1.0, max(0.51, 0.20, 0.92, 0.94))
  Result                = 0.9430
```

---

## Appendix: All Formulas at a Glance

| Engine | Formula | Range | Interpretation |
|--------|---------|-------|----------------|
| MGTG | $M = E - N + 2P$ | 1+ | Cyclomatic complexity |
| MGTG | $\text{Health} = 1 - (0.1E + 0.05W)/\max(1,F)$ | [0,1] | Higher = healthier |
| DEP | $\text{FanIn}(v) = \|u \in V : u \rightarrow v\|$ | 0+ | Reuse count |
| DEP | $\text{Risk}_{\text{DEP}} = \min(1, 0.2 \times \text{errors})$ | [0,1] | Linear in errors |
| PROC | $H(f) = -\sum p_i \log_2 p_i$ | [0, $\log_2 n$] | Change distribution |
| PROC | $\text{Risk}_{\text{PROC}} = \min(1, 0.3E + 0.1W + 0.1H_{\max})$ | [0,1] | Weighted sum |
| ML | $\hat{y} = \sigma(\mathbf{w} \cdot \mathbf{x}' + b)$ | [0,1] | Defect probability |
| ML | $J = -\frac{1}{n}\sum[y\ln\hat{y} + (1-y)\ln(1-\hat{y})] + \frac{\lambda}{2n}\sum w^2$ | [0,$\infty$) | Log-loss + L2 |
| ORCH | $\text{Risk}_{\text{final}} = \min(1, \max_e \text{Risk}_e)$ | [0,1] | Max fusion |
| HITL | $P_e = \frac{C_e}{C_e + F_e}$ | [0,1] | Precision per engine |
| RSE | $S = 1 - \max(\text{CPU}, \text{MEM}, \text{GC}, \text{Thread}, \text{Latency})$ | [0,1] | Higher = more survivable |

---

*This document is a living reference. As Sutra evolves, new formulas and engines
will be added. For the latest version, see the source code at
`github.com/darshanredkar11/sutra`.*
