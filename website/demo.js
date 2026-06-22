// ── Sigmoid ──
function sigmoid(z) {
  if (z > 700) return 1.0;
  if (z < -700) return 0.0;
  return 1 / (1 + Math.exp(-z));
}

// ── Dot product ──
function dot(w, x) {
  let sum = 0;
  for (let i = 0; i < w.length; i++) sum += w[i] * x[i];
  return sum;
}

// ── Standard scale ──
function scale(features, means, stds) {
  return features.map((v, i) => (v - means[i]) / Math.max(stds[i], 1.0));
}

// ── Predict risk ──
function predictRisk(features, model) {
  const scaled = scale(features, model.means, model.stds);
  const z = dot(model.weights, scaled) + model.bias;
  return sigmoid(z);
}

// ── Default model (trained on representative data) ──
const DEFAULT_MODEL = {
  weights: [
    0.21,   // revisions
    0.15,   // distinct_committers
    0.08,   // lines_added
    0.06,   // lines_deleted
    0.12,   // total_lines_changed
    0.32,   // entropy (highest weight — most predictive)
    0.10,   // num_directories
    0.14,   // avg_files_per_commit
   -0.05,   // age_days (older files are more stable)
    0.18,   // weighted_age_days
    0.25,   // recent_commits
    0.30,   // bug_fix_commits
   -0.20,   // owner_contribution (bus factor — more owners = more risk)
    0.11,   // minor_contributors
  ],
  bias: -1.5,
  means: [10, 3, 500, 200, 700, 2.5, 2, 3, 365, 180, 15, 5, 0.6, 2],
  stds:  [15, 4, 800, 400, 1200, 2.0, 2, 2, 400, 250, 25, 10, 0.3, 3],
};

const FEATURE_NAMES = [
  "revisions", "distinct_committers", "lines_added", "lines_deleted",
  "total_lines_changed", "entropy", "num_directories", "avg_files_per_commit",
  "age_days", "weighted_age_days", "recent_commits", "bug_fix_commits",
  "owner_contribution", "minor_contributors",
];

const FEATURE_DESCS = [
  "Times file was modified",
  "Unique authors",
  "Lines added total",
  "Lines deleted total",
  "Sum added + deleted",
  "Change distribution (0–6+)",
  "Directories touched",
  "Avg files per commit",
  "Days since first change",
  "Age × change frequency",
  "Commits (90 days)",
  "Commits with fix keywords",
  "Primary author share",
  "Authors with <5% share",
];

const FEATURE_DEFAULTS = [5, 2, 200, 80, 280, 1.5, 1, 2.0, 180, 90, 8, 2, 0.7, 1];

// ── State ──
let featureValues = [...FEATURE_DEFAULTS];
let riskScore = 0;
let selectedFeatures = new Set(FEATURE_NAMES.map((_, i) => i));

// ── DOM refs ──
const slidersContainer = document.getElementById("demo-sliders");
const riskGauge = document.getElementById("demo-risk-gauge");
const riskValue = document.getElementById("demo-risk-value");
const riskLabel = document.getElementById("demo-risk-label");
const breakdownTable = document.getElementById("demo-breakdown");
const pipelineFlow = document.getElementById("demo-pipeline-flow");

// ── Init sliders ──
function initSliders() {
  for (let i = 0; i < FEATURE_NAMES.length; i++) {
    const div = document.createElement("div");
    div.className = "demo-slider-row";

    const maxVals = [50, 15, 3000, 1500, 4500, 6, 8, 10, 1500, 1000, 100, 40, 1.0, 15];

    div.innerHTML = `
      <div class="demo-slider-info">
        <span class="demo-slider-name">${FEATURE_NAMES[i]}</span>
        <span class="demo-slider-desc">${FEATURE_DESCS[i]}</span>
      </div>
      <div class="demo-slider-control">
        <input type="range" id="fs-${i}" min="0" max="${maxVals[i]}" step="${i === 12 ? 0.01 : 0.1}" value="${featureValues[i]}">
        <span class="demo-slider-val" id="fv-${i}">${featureValues[i]}</span>
      </div>
    `;

    const input = div.querySelector("input");
    const valSpan = div.querySelector(".demo-slider-val");

    input.addEventListener("input", () => {
      const v = parseFloat(input.value);
      featureValues[i] = v;
      valSpan.textContent = v.toFixed(v % 1 === 0 ? 0 : 2);
      computeRisk();
    });

    slidersContainer.appendChild(div);
  }
}

// ── Compute risk ──
function computeRisk() {
  const activeFeatures = featureValues.map((v, i) => selectedFeatures.has(i) ? v : 0);
  riskScore = predictRisk(activeFeatures, DEFAULT_MODEL);
  updateGauge();
  updateBreakdown();
  updatePipeline();
}

// ── Gauge ──
function updateGauge() {
  const pct = (riskScore * 100).toFixed(1);
  riskValue.textContent = `${pct}%`;
  riskGauge.style.width = `${Math.min(riskScore * 100, 100)}%`;

  let label, color;
  if (riskScore < 0.3) {
    label = "LOW RISK";
    color = "#22c55e";
  } else if (riskScore < 0.6) {
    label = "MODERATE RISK";
    color = "#eab308";
  } else if (riskScore < 0.8) {
    label = "HIGH RISK";
    color = "#f97316";
  } else {
    label = "CRITICAL";
    color = "#ef4444";
  }

  riskLabel.textContent = label;
  riskLabel.style.color = color;
  riskGauge.style.background = color;
}

// ── Breakdown table ──
function updateBreakdown() {
  let html = `
    <tr><td>z = w·x + b</td><td>${(dot(DEFAULT_MODEL.weights, featureValues.map((v, i) => selectedFeatures.has(i) ? (v - DEFAULT_MODEL.means[i]) / Math.max(DEFAULT_MODEL.stds[i], 1) : 0)) + DEFAULT_MODEL.bias).toFixed(4)}</td></tr>
    <tr><td>σ(z) = 1/(1+e<sup>−z</sup>)</td><td>${riskScore.toFixed(6)}</td></tr>
    <tr><td>Engines triggered</td><td>${getTriggeredEngines().join(", ") || "none"}</td></tr>
  `;
  breakdownTable.innerHTML = html;
}

function getTriggeredEngines() {
  const triggered = [];
  if (featureValues[0] > 20) triggered.push("mgtg");
  if (featureValues[5] > 3) triggered.push("process");
  if (featureValues[0] > 10 && featureValues[5] > 2) triggered.push("ml");
  if (riskScore > 0.6) triggered.push("hitl");
  return triggered;
}

// ── Pipeline flow ──
function updatePipeline() {
  const triggered = getTriggeredEngines();
  const steps = pipelineFlow.querySelectorAll(".pipeline-step");
  steps.forEach(s => {
    const name = s.getAttribute("data-engine");
    s.classList.toggle("demo-active", triggered.includes(name));
    s.classList.toggle("demo-inactive", !triggered.includes(name));
  });
}

// ── Presets ──
const PRESETS = {
  stable: { values: [2, 1, 50, 10, 60, 0.5, 1, 1.2, 500, 250, 2, 0, 0.9, 0], label: "Stable file" },
  risky: { values: [25, 8, 1500, 600, 2100, 4.5, 4, 5.5, 100, 400, 40, 15, 0.3, 8], label: "High-risk file" },
  churn: { values: [40, 12, 2500, 1200, 3700, 5.8, 6, 7.0, 50, 800, 80, 25, 0.4, 12], label: "Churn hotspot" },
  new: { values: [1, 1, 300, 100, 400, 1.0, 1, 2.0, 10, 5, 5, 1, 1.0, 0], label: "New file" },
};

function applyPreset(name) {
  const preset = PRESETS[name];
  if (!preset) return;
  featureValues = [...preset.values];
  for (let i = 0; i < FEATURE_NAMES.length; i++) {
    const input = document.getElementById(`fs-${i}`);
    const valSpan = document.getElementById(`fv-${i}`);
    if (input) {
      input.value = featureValues[i];
      valSpan.textContent = featureValues[i].toFixed(featureValues[i] % 1 === 0 ? 0 : 2);
    }
  }
  computeRisk();
}

// ── Expose ──
window.applyPreset = applyPreset;

// ── Start ──
document.addEventListener("DOMContentLoaded", () => {
  initSliders();
  computeRisk();
});
