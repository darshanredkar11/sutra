use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use sutra_common::error::SutraResult;
use sutra_schema::v1::{AnalyzeRequest, ComponentHealth, HealthStatus};
use tokio::sync::RwLock;
use tracing::info;

use crate::coordinator::Orchestrator;

pub struct AppState {
    pub orchestrator: Orchestrator,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

pub type SharedState = Arc<RwLock<AppState>>;

pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/v1/analyze", axum::routing::post(handle_analyze))
        .route("/v1/health", get(handle_health))
        .route("/v1/status", get(handle_status))
        .layer(
            tower_http::cors::CorsLayer::permissive(),
        )
        .with_state(state)
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
