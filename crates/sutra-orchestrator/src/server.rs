use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sutra_common::error::SutraResult;
use sutra_schema::v1::{AnalyzeRequest, ComponentHealth, Engine, HealthStatus, Severity};
use tokio::sync::RwLock;
use tracing::info;

use crate::coordinator::Orchestrator;

pub struct AppState {
    pub orchestrator: Orchestrator,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

pub type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug, Deserialize)]
pub struct DemoRequest {
    pub repo_url: String,
}

#[derive(Debug, Serialize)]
pub struct DemoResponse {
    pub repo_url: String,
    pub repo_name: String,
    pub overall_risk: f64,
    pub risk_label: String,
    pub findings_count: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info_count: usize,
    pub processing_time_ms: f64,
    pub findings: Vec<serde_json::Value>,
    pub error: Option<String>,
}

fn risk_label(risk: f64) -> &'static str {
    if risk < 0.3 { "LOW" } else if risk < 0.6 { "MODERATE" } else if risk < 0.8 { "HIGH" } else { "CRITICAL" }
}

fn extract_repo_name(url: &str) -> String {
    url.trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .trim_start_matches("github.com/")
        .trim_end_matches(".git")
        .trim_end_matches('/')
        .to_string()
}

fn is_github_url(url: &str) -> bool {
    url.starts_with("https://github.com/") || url.starts_with("http://github.com/") || url.starts_with("github.com/")
}

pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/v1/analyze", axum::routing::post(handle_analyze))
        .route("/v1/demo", axum::routing::post(handle_demo))
        .route("/v1/report", get(handle_report))
        .route("/v1/health", get(handle_health))
        .route("/v1/status", get(handle_status))
        .route("/v1/openapi.json", get(handle_openapi))
        .route("/v1/docs", get(handle_swagger_ui))
        .layer(
            tower_http::cors::CorsLayer::permissive(),
        )
        .with_state(state)
}

async fn handle_openapi() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        include_str!("openapi.json"),
    )
}

async fn handle_swagger_ui() -> impl IntoResponse {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Sutra API — Swagger UI</title>
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
<div id="swagger-ui"></div>
<script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
<script>
  SwaggerUIBundle({
    url: '/v1/openapi.json',
    dom_id: '#swagger-ui',
    presets: [SwaggerUIBundle.presets.apis],
    layout: 'BaseLayout',
    deepLinking: true,
    showExtensions: true,
    showCommonExtensions: true,
  });
</script>
</body>
</html>"#;
    (StatusCode::OK, [("content-type", "text/html")], html)
}

#[derive(Deserialize)]
pub struct ReportQuery {
    pub repo: Option<String>,
}

pub async fn handle_report(
    Query(query): Query<ReportQuery>,
) -> impl IntoResponse {
    let html = include_str!("report.html");
    let html = if let Some(repo) = &query.repo {
        if is_github_url(repo) {
            html.replace(
                r#"value="https://github.com/darshanredkar11/sutra""#,
                &format!("value=\"{}\"", repo),
            )
        } else {
            html.to_string()
        }
    } else {
        html.to_string()
    };
    Html(html)
}

async fn handle_demo(
    State(state): State<SharedState>,
    Json(request): Json<DemoRequest>,
) -> impl IntoResponse {
    if !is_github_url(&request.repo_url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Only public GitHub URLs are supported (https://github.com/owner/repo)"
            })),
        ).into_response();
    }

    let repo_name = extract_repo_name(&request.repo_url);
    let tmp_id = uuid::Uuid::new_v4().to_string();
    let tmp_dir = std::env::temp_dir().join(format!("sutra-demo-{}", tmp_id));

    // Shallow clone with blobless filter for speed
    let clone_result = tokio::process::Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--single-branch")
        .arg("--filter=blob:none")
        .arg(&request.repo_url)
        .arg(&tmp_dir)
        .output()
        .await;

    let _output = match clone_result {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!(DemoResponse {
                    repo_url: request.repo_url,
                    repo_name,
                    overall_risk: 0.0,
                    risk_label: "ERROR".into(),
                    findings_count: 0,
                    errors: 0,
                    warnings: 0,
                    info_count: 0,
                    processing_time_ms: 0.0,
                    findings: vec![],
                    error: Some(format!("Clone failed: {}", stderr)),
                })),
            ).into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!(DemoResponse {
                    repo_url: request.repo_url,
                    repo_name,
                    overall_risk: 0.0,
                    risk_label: "ERROR".into(),
                    findings_count: 0,
                    errors: 0,
                    warnings: 0,
                    info_count: 0,
                    processing_time_ms: 0.0,
                    findings: vec![],
                    error: Some(format!("Clone error: {}", e)),
                })),
            ).into_response();
        }
    };

    let path_str = tmp_dir.to_str().unwrap_or("").to_string();
    let mut analysis_request = AnalyzeRequest::new(&path_str, "HEAD");
    analysis_request.request_id = format!("demo-{}", tmp_id);
    analysis_request.engines = vec![
        Engine::Mgtg, Engine::Dependency, Engine::Process,
        Engine::Ml, Engine::Hitl,
    ];

    let state = state.read().await;
    let start = std::time::Instant::now();
    let analysis = state.orchestrator.analyze(&analysis_request);
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    drop(state);

    // Cleanup temp dir in background
    let cleanup_dir = tmp_dir.clone();
    tokio::spawn(async move {
        tokio::process::Command::new("rm")
            .arg("-rf")
            .arg(&cleanup_dir)
            .output()
            .await
            .ok();
    });

    match analysis {
        Ok(result) => {
            let errors = result.findings.iter().filter(|f| matches!(f.severity, Severity::Error | Severity::Critical)).count();
            let warnings = result.findings.iter().filter(|f| f.severity == Severity::Warning).count();
            let info_count = result.findings.iter().filter(|f| f.severity == Severity::Info).count();

            let findings: Vec<serde_json::Value> = result.findings.iter().map(|f| {
                serde_json::json!({
                    "id": f.id,
                    "engine": f.engine.as_str(),
                    "file": f.file_path,
                    "line": f.line,
                    "message": f.message,
                    "severity": format!("{:?}", f.severity),
                })
            }).collect();

            (
                StatusCode::OK,
                Json(serde_json::json!(DemoResponse {
                    repo_url: request.repo_url,
                    repo_name,
                    overall_risk: result.overall_risk,
                    risk_label: risk_label(result.overall_risk).into(),
                    findings_count: result.findings.len(),
                    errors,
                    warnings,
                    info_count,
                    processing_time_ms: elapsed,
                    findings,
                    error: None,
                })),
            ).into_response()
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!(DemoResponse {
                    repo_url: request.repo_url,
                    repo_name,
                    overall_risk: 0.0,
                    risk_label: "ERROR".into(),
                    findings_count: 0,
                    errors: 0,
                    warnings: 0,
                    info_count: 0,
                    processing_time_ms: elapsed,
                    findings: vec![],
                    error: Some(format!("Analysis failed: {}", e)),
                })),
            ).into_response()
        }
    }
}

pub fn create_shared_state(orchestrator: Orchestrator) -> SharedState {
    Arc::new(RwLock::new(AppState {
        orchestrator,
        started_at: chrono::Utc::now(),
    }))
}

async fn handle_analyze(
    State(state): State<SharedState>,
    Json(request): Json<AnalyzeRequest>,
) -> impl IntoResponse {
    let state = state.read().await;
    match state.orchestrator.analyze(&request) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string(),
                "request_id": request.request_id,
            })),
        )
            .into_response(),
    }
}

async fn handle_health(State(state): State<SharedState>) -> Json<Vec<ComponentHealth>> {
    let state = state.read().await;
    let health: Vec<ComponentHealth> = state
        .orchestrator
        .health_check()
        .into_iter()
        .map(|(engine, _)| ComponentHealth {
            name: engine.as_str().to_string(),
            status: HealthStatus::Healthy,
            message: None,
            last_heartbeat_ms: state.started_at.timestamp_millis() as u64,
        })
        .collect();
    Json(health)
}

async fn handle_status(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let uptime = chrono::Utc::now()
        .signed_duration_since(state.started_at)
        .num_seconds();
    Json(serde_json::json!({
        "version": "0.1.0",
        "uptime_seconds": uptime,
        "engines": state.orchestrator.engine_names(),
        "started_at": state.started_at.to_rfc3339(),
    }))
}

pub async fn start_server(
    orchestrator: Orchestrator,
    port: u16,
) -> SutraResult<()> {
    let state = create_shared_state(orchestrator);
    let app = build_router(state);
    let addr = format!("0.0.0.0:{}", port);
    info!("starting sutra server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| sutra_common::error::SutraError::config(format!("cannot bind to {}: {}", addr, e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| sutra_common::error::SutraError::config(format!("server error: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use sutra_common::engine::AnalysisEngine;
    use sutra_common::error::SutraResult;
    use sutra_schema::v1::{Engine, Finding, Severity};
    use tower::ServiceExt;

    struct MockEngine;

    impl AnalysisEngine for MockEngine {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn analyze(&self, request: &AnalyzeRequest) -> SutraResult<sutra_schema::v1::AnalysisResult> {
            Ok(sutra_schema::v1::AnalysisResult {
                request_id: request.request_id.clone(),
                commit_hash: request.commit_hash.clone(),
                overall_risk: 0.3,
                findings: vec![Finding::new("M-1", Engine::Mgtg, "f.rs", 1, "mock", Severity::Info)],
                recommendations: vec![],
                metrics: None,
                processing_time_ms: 5.0,
                blocked_merge: false,
                jit_features: None,
            })
        }
    }

    fn test_orchestrator() -> Orchestrator {
        let mut o = Orchestrator::new();
        o.register(Engine::Mgtg, Box::new(MockEngine));
        o
    }

    async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
        response.into_body().collect().await.unwrap().to_bytes().to_vec()
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let o = test_orchestrator();
        let app = build_router(create_shared_state(o));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: Vec<serde_json::Value> =
            serde_json::from_slice(&body_bytes(response).await).unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0]["name"], "mgtg");
    }

    #[tokio::test]
    async fn test_status_endpoint() {
        let o = test_orchestrator();
        let app = build_router(create_shared_state(o));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&body_bytes(response).await).unwrap();
        assert_eq!(body["version"], "0.1.0");
        assert!(body["uptime_seconds"].as_i64().unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_analyze_endpoint() {
        let o = test_orchestrator();
        let app = build_router(create_shared_state(o));

        let req = AnalyzeRequest::new("/repo", "abc123");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/analyze")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: sutra_schema::v1::AnalysisResult =
            serde_json::from_slice(&body_bytes(response).await).unwrap();
        assert_eq!(body.findings.len(), 1);
    }

    #[tokio::test]
    async fn test_analyze_endpoint_bad_request() {
        let o = test_orchestrator();
        let app = build_router(create_shared_state(o));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/analyze")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"bad": "json"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_analyze_endpoint_empty_body() {
        let o = test_orchestrator();
        let app = build_router(create_shared_state(o));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/analyze")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(""))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_analyze_endpoint_malformed_json() {
        let o = test_orchestrator();
        let app = build_router(create_shared_state(o));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/analyze")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from("{invalid json!!}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_analyze_endpoint_missing_fields_use_defaults() {
        let o = test_orchestrator();
        let app = build_router(create_shared_state(o));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/analyze")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"repo_path": "/repo", "commit_hash": "abc", "request_id": "test-id"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: sutra_schema::v1::AnalysisResult =
            serde_json::from_slice(&body_bytes(response).await).unwrap();
        // Response should be valid with default values for omitted optional fields
        assert!(body.request_id == "test-id");
    }
}
