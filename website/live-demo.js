// ── Live Repo Analysis Demo ──

const API_BASE = window.SUTRA_API_URL || "http://localhost:8080";

const form = document.getElementById("live-demo-form");
const input = document.getElementById("live-demo-url");
const statusEl = document.getElementById("live-demo-status");
const resultsEl = document.getElementById("live-demo-results");

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const url = input.value.trim();
  if (!url) return;

  resultsEl.classList.add("live-hidden");
  statusEl.classList.remove("live-hidden");
  statusEl.innerHTML = `
    <div class="live-spinner"></div>
    <span>Cloning repository and running all 5 engines...</span>
  `;

  try {
    const res = await fetch(`${API_BASE}/v1/demo`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ repo_url: url }),
      signal: AbortSignal.timeout(120000),
    });

    const data = await res.json();
    statusEl.classList.add("live-hidden");
    resultsEl.classList.remove("live-hidden");

    if (data.error) {
      resultsEl.innerHTML = `
        <div class="live-error">
          <h4>Error</h4>
          <p>${escapeHtml(data.error)}</p>
        </div>`;
      return;
    }

    renderResults(data);
  } catch (err) {
    statusEl.classList.add("live-hidden");
    resultsEl.classList.remove("live-hidden");
    resultsEl.innerHTML = `
      <div class="live-error">
        <h4>Request Failed</h4>
        <p>${escapeHtml(err.message)}</p>
        <p class="live-hint">Make sure the demo server is running. Set <code>SUTRA_API_URL</code> if using a custom endpoint.</p>
      </div>`;
  }
});

function renderResults(data) {
  const labelClass = `live-badge-${data.risk_label.toLowerCase()}`;
  const findingsTotal = data.findings_count;
  const breakdown = [
    { label: "Errors", count: data.errors, cls: "live-count-error" },
    { label: "Warnings", count: data.warnings, cls: "live-count-warn" },
    { label: "Info", count: data.info_count, cls: "live-count-info" },
  ];

  let findingsHtml = "";
  if (data.findings && data.findings.length > 0) {
    findingsHtml = data.findings.slice(0, 20).map(f => `
      <tr>
        <td class="live-find-id">${escapeHtml(f.id)}</td>
        <td class="live-find-engine">${escapeHtml(f.engine)}</td>
        <td class="live-find-file">${escapeHtml(truncatePath(f.file, 40))}:${f.line}</td>
        <td class="live-find-msg">${escapeHtml(truncate(f.message, 60))}</td>
        <td class="live-find-sev live-sev-${f.severity.toLowerCase()}">${f.severity}</td>
      </tr>
    `).join("");
  } else {
    findingsHtml = `<tr><td colspan="5" class="live-empty">No findings — clean bill of health</td></tr>`;
  }

  const enginesTriggered = data.engines_triggered && data.engines_triggered.length > 0
    ? data.engines_triggered.map(e => `<span class="live-engine-tag">${e}</span>`).join("")
    : '<span class="live-muted">none</span>';

  resultsEl.innerHTML = `
    <div class="live-summary">
      <div class="live-risk-section">
        <div class="live-risk-label">Risk Score</div>
        <div class="live-risk-value ${labelClass}">${(data.overall_risk * 100).toFixed(1)}%</div>
        <div class="live-risk-badge ${labelClass}">${data.risk_label}</div>
      </div>
      <div class="live-meta-section">
        <div class="live-meta-row">
          <span class="live-meta-label">Repository</span>
          <span class="live-meta-value">${escapeHtml(data.repo_name)}</span>
        </div>
        <div class="live-meta-row">
          <span class="live-meta-label">Time</span>
          <span class="live-meta-value">${data.processing_time_ms.toFixed(0)}ms</span>
        </div>
        <div class="live-meta-row">
          <span class="live-meta-label">Findings</span>
          <span class="live-meta-value">${findingsTotal}</span>
        </div>
        <div class="live-meta-row">
          <span class="live-meta-label">Engines</span>
          <span class="live-meta-value">${enginesTriggered}</span>
        </div>
      </div>
      <div class="live-breakdown">
        ${breakdown.map(b => `
          <div class="live-breakdown-item">
            <span class="live-breakdown-count ${b.cls}">${b.count}</span>
            <span class="live-breakdown-label">${b.label}</span>
          </div>
        `).join("")}
      </div>
    </div>

    <div class="live-findings-section">
      <h4>Findings <span class="live-count-badge">${findingsTotal}</span></h4>
      <div class="live-table-wrap">
        <table class="live-table">
          <thead>
            <tr><th>ID</th><th>Engine</th><th>Location</th><th>Message</th><th>Severity</th></tr>
          </thead>
          <tbody>${findingsHtml}</tbody>
        </table>
      </div>
      ${data.findings && data.findings.length > 20
        ? `<p class="live-truncated">Showing top 20 of ${data.findings.length} findings</p>`
        : ""}
    </div>
  `;
}

function escapeHtml(s) {
  if (typeof s !== "string") return String(s);
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function truncate(s, n) {
  if (typeof s !== "string") return String(s);
  return s.length > n ? s.slice(0, n) + "…" : s;
}

function truncatePath(p, n) {
  if (typeof p !== "string") return String(p);
  if (p.length <= n) return p;
  const parts = p.split("/");
  let result = parts.pop();
  while (parts.length > 0 && result.length + parts[parts.length-1].length + 3 < n) {
    result = parts.pop() + "/" + result;
  }
  return "…/" + result;
}
