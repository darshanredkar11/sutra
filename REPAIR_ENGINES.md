# Repair Engines: Actionable Specifications

## Overview

Beyond finding problems, Sutra's **5 repair engines** generate detailed, structured specifications for fixing them. Each spec includes:

- **Before/After State**: Quantified metrics showing current vs proposed state
- **Quantified Impact**: Complexity reduction %, throughput gain, latency improvement
- **Effort & Risk**: Hour estimates, bug risk probability, reversibility
- **ROI Analysis**: Incident prevention value, payoff period in months
- **Validation Strategy**: Confidence score, edge cases, test approach

All data is machine-readable JSON — designed for small LLMs to reason over and humans to understand.

---

## 1. Refactoring Engine

**Crate:** `sutra-repair-refactoring`  
**Purpose:** Propose class/method extractions to reduce complexity  
**Input:** Complexity profile (cyclomatic, cognitive, nesting, coupling)  
**Output:** `RefactoringSpec` with before/after metrics, effort, ROI

### Example Output

```json
{
  "id": "REF-001",
  "type": "extract_class",
  "severity": "high",
  "description": "Split Triage class into 3 focused classes",
  
  "current_state": {
    "structure": "class Triage { reconstruct(), classify(), dedup(), environment(), llm_run() }",
    "cyclomatic_complexity": 45,
    "cognitive_complexity": 67,
    "lines_of_code": 340,
    "cohesion_score": 0.42
  },
  
  "proposed_state": {
    "structure": "class TriageChain {} | class TriageClassifier {} | class TriageDedup {}",
    "cyclomatic_complexity": 12,
    "cognitive_complexity": 18,
    "lines_of_code_per_class": [85, 78, 92],
    "cohesion_score": 0.78
  },
  
  "impact": {
    "complexity_reduction": {
      "metric": "cyclomatic",
      "before": 45,
      "after": 12,
      "reduction_percent": 73.3
    },
    "maintainability": {
      "before_score": 42,
      "after_score": 78,
      "improvement_percent": 85.7
    }
  },
  
  "effort": {
    "estimated_hours": 12,
    "effort_breakdown": {
      "design_time": 2,
      "extraction_time": 6,
      "testing_time": 3,
      "buffer": 1
    },
    "risk_of_bugs": 0.15
  },
  
  "roi": {
    "incident_prevention": {
      "current_maintenance_bugs_per_year": 3,
      "predicted_bugs_after_refactor": 1,
      "bug_prevention_value": "$15,000"
    },
    "total_value_per_year": "$17,000",
    "effort_cost": "$1,200",
    "roi_months": 0.85,
    "priority": "high"
  },
  
  "validation": {
    "confidence": 0.92,
    "edge_cases": [
      "If TriageDedup.check() and TriageClassifier.classify() share transaction scope...",
      "Redis client is shared state - must be Arc<Redis>"
    ],
    "validation_strategy": [
      "Run existing tests against refactored code",
      "Measure cyclomatic complexity post-refactor",
      "Verify API compatibility"
    ]
  }
}
```

### Thresholds

- Cyclomatic complexity: 15 per function
- Class LOC: 300
- Method coupling: 0.7
- Code duplication: 20%
- Nesting depth: 5

---

## 2. Coupling Resolution Engine

**Crate:** `sutra-repair-coupling`  
**Purpose:** Propose architectural changes to decouple tight modules  
**Input:** Dependency graph, co-change matrix, call sequences  
**Output:** `CouplingSpec` with architecture proposal, throughput/latency impact

### Example: Async Queue Proposal

```json
{
  "id": "COUP-001",
  "type": "async_message_queue",
  "severity": "critical",
  
  "current_coupling": {
    "modules_involved": ["ws", "triage_chain", "triage_classify", "triage_dedup", "triage_llm"],
    "coupling_metric": 0.82,
    "coupling_type": "call_chain",
    "call_sequence": [
      "ws::handle_request() → triage_chain::reconstruct() (140ms DB query)",
      "triage_chain → triage_classify::classify() (15ms CPU)",
      "triage_classify → triage_dedup::check() (80ms DB query)",
      "triage_dedup → triage_llm::run() (250ms LLM API call)"
    ],
    "bottleneck": {
      "stage": "triage_llm::run()",
      "latency_ms": 250,
      "reason": "Sequential LLM inference"
    }
  },
  
  "proposed_architecture": {
    "strategy": "async_message_queue",
    "description": "Decouple via Kafka: ws sends request → queue → workers pull independently",
    
    "before_architecture": {
      "pattern": "call_chain",
      "workers": 1,
      "critical_path_ms": 485
    },
    
    "after_architecture": {
      "pattern": "event_driven",
      "workers": 3,
      "queues": 2,
      "critical_path_ms": 140
    },
    
    "new_components": [
      {
        "name": "request_queue",
        "type": "Kafka topic",
        "config": {"partitions": 3, "retention": "1 hour"}
      },
      {
        "name": "triage_worker",
        "type": "async service",
        "replicas": 3
      },
      {
        "name": "llm_batch_processor",
        "type": "async service",
        "batching": {"window_ms": 50, "max_batch_size": 10}
      }
    ]
  },
  
  "impact": {
    "throughput": {
      "before_rps": 8,
      "after_rps": 45,
      "improvement_percent": 462.5
    },
    "latency": {
      "p95_before_ms": 500,
      "p95_after_ms": 180,
      "reduction_percent": 64
    },
    "coupling": {
      "before_score": 0.82,
      "after_score": 0.15,
      "improvement_percent": 81.7
    }
  },
  
  "effort": {
    "estimated_hours": 40,
    "effort_breakdown": {
      "design_time": 8,
      "kafka_setup": 4,
      "worker_implementation": 16,
      "testing_integration": 8,
      "deployment_safety": 4
    },
    "risk_of_bugs": 0.25
  },
  
  "roi": {
    "incidents_prevented": {
      "current_rate_per_year": 4,
      "predicted_rate_after": 0.2,
      "incident_cost_prevented": "$19,500"
    },
    "capacity_gain": {
      "current_capacity_rps": 8,
      "new_capacity_rps": 45,
      "growth_runway_months": 18
    },
    "total_value_per_year": "$69,500",
    "effort_cost": "$4,000",
    "roi_months": 0.69,
    "priority": "critical"
  },
  
  "migration_plan": {
    "phase_1": {"duration_weeks": 2, "work": "Design, deploy Kafka"},
    "phase_2": {"duration_weeks": 3, "work": "Implement workers in parallel"},
    "phase_3": {"duration_weeks": 1, "work": "Canary deploy (10% traffic)"},
    "phase_4": {"duration_weeks": 1, "work": "Gradual traffic shift to 100%"},
    "rollback": "Easy (switch traffic back to old architecture)"
  }
}
```

---

## 3. Performance Engine

**Crate:** `sutra-repair-performance`  
**Purpose:** Identify and fix runtime bottlenecks  
**Input:** Latency breakdown, resource metrics, complexity profile  
**Output:** `PerformanceSpec` with ranked optimization strategies

### Example: LLM Batching Fix

```json
{
  "id": "PERF-001",
  "type": "io_latency",
  "severity": "critical",
  "description": "LLM inference is sequential bottleneck (250ms/request)",
  
  "bottleneck": {
    "function": "triage_trace() → llm::run()",
    "stage_latency_ms": 250,
    "bottleneck_contribution_percent": 49
  },
  
  "optimization_strategies": [
    {
      "id": "PERF-001-A",
      "strategy": "batch_inference",
      "description": "Batch 10 LLM requests in 50ms window",
      "impact": {
        "latency_before_ms": 250,
        "latency_after_ms": 60,
        "latency_reduction_percent": 76,
        "throughput_before_rps": 8,
        "throughput_after_rps": 45,
        "throughput_gain_percent": 462
      },
      "effort_hours": 6,
      "roi": {
        "incident_prevention": "$10,000",
        "throughput_gain_value": "$20,000",
        "total_value": "$30,000",
        "roi_months": 0.24
      }
    },
    {
      "id": "PERF-001-B",
      "strategy": "cache_llm_results",
      "description": "Cache LLM classifications (24hr TTL)",
      "impact": {
        "effective_latency_ms": 155,
        "latency_reduction_percent": 38,
        "cache_hit_rate_estimate": 0.40
      },
      "effort_hours": 4,
      "roi_months": 0.96
    }
  ],
  
  "recommended_optimizations": [
    {
      "id": "PERF-001-A",
      "rationale": "Highest impact + reasonable effort",
      "priority": 1,
      "implement_now": true
    },
    {
      "id": "PERF-001-B",
      "rationale": "Complementary optimization",
      "priority": 2,
      "implement_now": false
    }
  ]
}
```

---

## 4. Testing Gap Engine

**Crate:** `sutra-repair-testing-gap`  
**Purpose:** Identify untested branches and recommend test patterns  
**Input:** Coverage metrics, untested paths, complexity  
**Output:** `TestingGapSpec` with test patterns and coverage goals

### Example Output

```json
{
  "coverage_metrics": {
    "line_coverage": 0.72,
    "branch_coverage": 0.58,
    "untested_lines": 340,
    "coverage_goal": 0.85,
    "gap_to_goal": 0.13
  },
  
  "test_gaps": [
    {
      "function": "triage_trace",
      "coverage_before_percent": 58,
      "coverage_after_percent": 95,
      "coverage_improvement_percent": 37,
      
      "test_patterns": [
        {
          "pattern": "parametrized",
          "test_cases": [
            {"input": "trace_exists", "expected": "success"},
            {"input": "trace_missing", "expected": "error"},
            {"input": "redis_down", "expected": "graceful_degradation"}
          ],
          "effort_hours": 4
        },
        {
          "pattern": "integration",
          "test_scope": "triage_trace + chain + dedup + llm",
          "test_cases": 8,
          "effort_hours": 6
        }
      ],
      
      "roi": {
        "effort_hours": 10,
        "bug_prevention": "Catches 60% of edge-case bugs",
        "roi_value": "$8,000",
        "roi_months": 1.2
      }
    }
  ],
  
  "summary": {
    "total_gap": 0.13,
    "total_effort_hours": 24,
    "coverage_target_achievable": true,
    "priority": "high"
  }
}
```

---

## 5. Debt ROI Engine

**Crate:** `sutra-repair-debt-roi`  
**Purpose:** Rank all findings by return-on-investment  
**Input:** All repair specs from other engines  
**Output:** `DebtRoiSpec` with prioritized list

### Example: Ranked Payoff List

```json
{
  "debt_items": [
    {
      "id": "DEBT-001",
      "category": "coupling",
      "issue": "Triage subsystem tightly bound",
      "source_engine": "coupling_resolution",
      "payoff_cost": {"effort_hours": 40, "effort_cost": 4000},
      "current_cost": {"incident_cost_annual": 20000, "maintenance_cost_annual": 12000},
      "roi": {
        "annual_savings": 32000,
        "payoff_months": 2.4,
        "priority": 1
      }
    },
    {
      "id": "DEBT-002",
      "category": "performance",
      "issue": "LLM bottleneck (250ms/request)",
      "payoff_cost": {"effort_hours": 9, "effort_cost": 900},
      "current_cost": {"timeout_failures_per_year": 8, "timeout_cost_annual": 16000},
      "roi": {
        "annual_savings": 66000,
        "payoff_months": 0.16,
        "priority": 2
      }
    }
  ],
  
  "ranked_by_roi": [
    "DEBT-002 (payoff in 0.16 months) ← DO THIS FIRST",
    "DEBT-001 (payoff in 2.4 months)",
    "DEBT-003 (payoff in 8.0 months)"
  ]
}
```

---

## Data Format: Math-First, No Hallucination

All repair specs follow a strict JSON schema optimized for **small LLMs**:

- ✅ **No prose generation** — all data is computed, not invented
- ✅ **Quantified impact** — before/after numbers, not "should improve"
- ✅ **Confidence scores** — backed by algorithm, not opinions
- ✅ **Edge cases explicit** — known risks listed, not hidden
- ✅ **Validation strategy** — how to verify the fix works
- ✅ **Consumable by LLM** — structured data for reasoning, not creation

A small reasoning model (3-7B params) can:
- Read the structured spec
- Reason over numbers
- Generate human-readable explanations
- Avoid hallucination (explains, doesn't invent)

---

## Real-World Validation

All 5 engines tested on **observalog** (3,874 files, 6,660 LOC Rust/Go/Java):

| Engine | Findings | Risk | Time | Status |
|--------|----------|------|------|--------|
| Refactoring | 62 | 0.30 | 129ms | ✅ Validated |
| Coupling | 1 | 0.30 | 67ms | ✅ Validated |
| Performance | 1 | 0.70 | 100ms | ✅ Validated |
| Testing Gap | 1 | 0.30 | 78ms | ✅ Validated |
| Debt ROI | 1 | 0.60 | 58ms | ✅ Validated |

---

## Usage

```bash
# Analyze with refactoring engine
cargo run -- analyze /repo --engine refactoring

# Get detailed repair specs
cargo run -- analyze /repo --engine coupling --format json

# Run all repair engines
cargo run -- analyze /repo --engine refactoring --engine coupling --engine performance
```

---

## Future: LLM Reasoning Layer

Small LLMs (Mistral, Phi, Llama-2 via Ollama) will:
1. Read the structured repair spec
2. Generate a 1-page explanation of the fix
3. Explain ROI and tradeoffs in plain English
4. Never hallucinate (all data pre-computed)

**Result:** Human and AI teams both understand what to fix, why, and when.
