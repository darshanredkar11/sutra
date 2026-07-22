# Sutra — parked, resume here

Parked mid-way through item 2 of a 4-item plan. Items 1, 3 done and pushed.
Item 2 (Visita precision re-audit) is **in progress, not concluded** —
no final precision number yet. Item 4 (connection-science engine) untouched,
correctly gated on item 2.

## Done and pushed (origin/main, in this order)

1. `5753d7d` — Replaced `sutra-rse::queueing`'s "empirical, not theoretical"
   heuristic with exact M/M/1 (`mm1`, `mm1_percentile`) and M/M/c (`mmc` via
   `erlang_b`). Regression-tested to 1e-9 against the closed-form values.
   One spec constant (p95 @ λ=0.8,μ=1) had a transposed digit — used the
   verified-correct value instead, noted in the test comment.
2. `a8754fc` — Fixed `cargo test --workspace` (was hanging/failing).
   Root cause: `sutra-hitl`'s `InMemoryFeedbackStore::store()` did O(n)
   real-disk I/O (full-file rewrite of `~/.sutra/hitl-feedback.json`) on
   *every* call, and `HitlEngine::new()` replayed it once per saved entry
   on construction → O(n²). The file had grown to 3249 entries from prior
   test runs, so every `HitlEngine::new()` (used by all 15 unit tests)
   took minutes. Fixed: store is now pure in-memory, persistence moved to
   `HitlEngine` behind an opt-in `persist_path` only set by `new()`; all
   tests switched to a hermetic `with_store()` helper. `cargo test
   --workspace` now runs clean in ~15-20s — **use this as the CI gate**.
3. `51deeaf` — Found during the Visita re-run (see below): 5
   `sutra-repair-*` engines each had their own copy-pasted `walkdir`
   walker with an incomplete vendor-dir exclusion (had `target/`,
   `node_modules/`, missing `build/`, `dist/`, `vendor/`, `.gradle/`,
   `venv/`, `__pycache__/`). Centralized into
   `sutra_common::fs::discover_source_files`. `mgtg` (standalone vendored
   crate, no sutra-common dep) got the same fix inline in `scanner.rs`.
4. `5c3c5e3` — Found the same way: `MGTG-G000` ("no else branch") was the
   single largest finding bucket (114/406 = 28% of everything) and was
   **structurally always-on for Rust** — the line-based `rust.rs` parser
   never populated `then_branch`/`else_branch` for Rust conditionals, so
   the guard-clause exception never fired and every single `if` in every
   Rust file got flagged, with corrupted messages ("Conditional 'if if
   request.method()...'", "Conditional 'if } else {'..."). Fixed the
   parser: proper else-branch attachment for `} else {`/`else if` chains,
   brace-balanced guard-clause detection (`return`/`Err(`/`panic!`/etc.),
   condition text no longer retains the keyword, `match` exempted (Rust
   matches are exhaustive by compiler enforcement, not an if/else gap).
   MGTG-G000 dropped 114 → 20 on the Visita re-run.

All four commits: `cargo build --workspace` clean, `cargo test
--workspace` green (~20-25s), each carries new regression tests. Verify
still-clean state before resuming: `cd sutra && cargo test --workspace`.

## Item 2 — Visita precision re-audit (WHERE THIS STOPPED)

Ran `target/release/sutra-cli analyze /Users/darshanredkar/darshan/visita
--engine all --format json`. Before the two "found during audit" fixes
above: **406 findings**. After: **303 findings** (26% drop, all four
fixes compounding). Raw output saved at `/tmp/visita_findings2.json` on
this machine (not committed — regenerate with the command above, it's
fast, ~10s including build).

Finding counts by rule (post-fix run):
```
TEST-NO-BRANCH       94   REF-COUPLING       61   RSE-SURV            33
PERF-LARGE-FUNC       21   MGTG-G000          20   TEST-ERROR-PATH     19
TEST-GAP              15   PERF-SYNC-IO       10   COUP-HUB             8
PERF-IO-HEAVY          7   DEBT-LARGE-FUNC     6   PERF-NESTED-LOOP     4
REF-DUPLICATION        3   REF-EXTRACT-METHOD  1   DEBT-COMPLEXITY      1
```
Confirmed via grep: **0 findings remaining against vendored/generated
paths** (`/build/`, `/generated/`, `/node_modules/`, `/target/`,
`/dist/`, `/.gradle/`, `/vendor/`) — was 11/406 before the walker fix.

### What's verified so far (partial — NOT a full audit)

Spot-checked against real Visita source (`/Users/darshanredkar/darshan/visita`):

- **TEST-NO-BRANCH** `health` handler "no branches" — **TRUE POSITIVE**.
  `crates/api/src/handlers/health.rs`: genuinely a trivial accessor, zero
  branches. Factually correct (whether it's worth *reporting* is a
  separate calibration question).
- **REF-COUPLING** `partition_upper_bound` / `add_months`
  (`crates/api/src/scheduler.rs:167`) — **LIKELY FALSE POSITIVE**. Read
  the actual code: `partition_upper_bound` is a private helper that calls
  `add_months` once, as one step of straightforward date arithmetic. That
  is completely normal function decomposition, not a coupling smell.
  REF-COUPLING's "tight coupling (0.75)" score looks like it's just
  measuring call-adjacency/frequency, not any real coupling-smell signal
  (shared mutable state, bidirectional dependency, etc.) — **this rule's
  calibration is suspect and needs the same kind of scrutiny MGTG-G000
  got**. Only 1 of 5 sampled findings checked so far.
- **COUP-HUB** — mid-investigation when parked. Two suspicious samples:
  `test_platform_admin_rbac_boundary_is_bidirectional` (a `#[tokio::test]`
  fn) reported with "fan-in 51, fan-out 0", and `fake_jwks` (a private
  test-only helper in `google_auth.rs`) reported with "fan-in 16,
  fan-out 0". Test functions should structurally be **leaves** (nothing
  else calls a `#[test]` fn) — "fan-in 51" for one is a red flag for
  either (a) a call-graph direction bug in whichever engine computes
  fan-in/fan-out (counting *outbound* calls as fan-in?), or (b) it's
  correctly counting `fake_jwks`'s callers (5-ish visible via grep, not
  16) and something is inflating the count. **Next step: find the
  fan-in/fan-out computation (likely in `sutra-repair-coupling` or
  `sutra-dependency`) and check both the direction and the count against
  ground truth before trusting any COUP-HUB finding.**
- **RSE-SURV** (33 findings, all "Healthy" info-level, no RSE-QUEUE
  overload findings fired) — not false positives in the "wrong claim"
  sense since they're just status info, not defect claims. Correctly
  reflects that the default 30 RPS expected-load config keeps every
  endpoint well under saturation with the new exact M/M/1 math. Not
  useful signal either way for the precision question — exclude from the
  precision denominator or treat separately as "informational."

### Not yet sampled at all
TEST-ERROR-PATH (19), TEST-GAP (15), PERF-SYNC-IO (10), PERF-IO-HEAVY (7),
DEBT-LARGE-FUNC (6), PERF-NESTED-LOOP (4, now Kotlin-test-only after the
vendor fix — worth checking these are real test files, not more
generated/fixture noise), REF-DUPLICATION (3), REF-EXTRACT-METHOD (1),
DEBT-COMPLEXITY (1).

### What "done" looks like for item 2
1. Resolve the COUP-HUB fan-in question (bug or real).
2. Sample-verify REF-COUPLING more (currently 1/5 = 20% true-positive on
   n=5, too small to trust — needs a bigger sample or a look at the
   coupling-score formula itself, likely in
   `sutra-repair-coupling/src/engine.rs`).
3. Sample the untouched rule types above.
4. Decide how to treat "if let Some(x) = y { ... }" with no else under
   MGTG-G000 (20 remaining findings, several are `if let` — idiomatic
   Rust very often has no meaningful "else" for the None/Err case; unlike
   plain `if/else` omitting an else is much less often a real gap here.
   This is a calibration call, not a parser bug like before — decide
   whether MGTG-G000 should skip `if let` entirely, or keep it but this
   is a real precision cost either way).
5. Compute final precision = true positives / total sampled (or full
   count if fully hand-graded), compare against the "did it move from
   1/12 toward >50%?" bar from the original ungraded/unrecorded manual
   review (that grading only ever existed in a prior chat session — not
   in the repo, not reproducible exactly; this re-audit is a fresh,
   independently-graded number, not a literal re-score of the same 12
   findings).
6. Only then: item 4 (centrality/articulation/cascade engine) unblocks.

## Quick resume commands
```
cd ~/darshan/sutra
cargo test --workspace                      # confirm gate still green
cargo run --release -p sutra-cli -- analyze /Users/darshanredkar/darshan/visita --engine all --format json > /tmp/visita_findings2.json
```
