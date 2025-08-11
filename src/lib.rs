use axum::{
    body::Body,
    extract::{Json, State},
    http::{header::HeaderValue, HeaderMap, StatusCode},
    response::Response,
    routing::post,
    Router,
};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const AZURE_BASE_URL: &str = "https://east-us2gpt.openai.azure.com";
const API_VERSION: &str = "2024-02-15-preview";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Clone)]
pub struct AppState {
    client: Client,
}

impl AppState {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

fn map_model(model: &str) -> &str {
    match model {
        "gpt-4o" => "east-us2gpt",
        other => other,
    }
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut req): Json<ChatCompletionRequest>,
) -> Result<Response, StatusCode> {
    let api_key = headers
        .get("api-key")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .to_string();

    let deployment = map_model(&req.model).to_string();
    req.model = deployment.clone();
    let url = format!(
        "{}/openai/deployments/{}/chat/completions?api-version={}",
        AZURE_BASE_URL, deployment, API_VERSION
    );

    let azure_resp = state
        .client
        .post(url)
        .header("api-key", api_key)
        .json(&req)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if req.stream {
        let stream = azure_resp.bytes_stream().map(|chunk| chunk.map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "stream error")));
        let body = Body::from_stream(stream);
        let mut resp = Response::new(body);
        resp.headers_mut()
            .insert("content-type", HeaderValue::from_static("text/event-stream"));
        Ok(resp)
    } else {
        let status = azure_resp.status();
        let bytes = azure_resp
            .bytes()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        let mut resp = Response::new(Body::from(bytes));
        *resp.status_mut() = axum::http::StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK);
        resp.headers_mut()
            .insert("content-type", HeaderValue::from_static("application/json"));
        Ok(resp)
    }
}

pub fn app() -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(AppState::new())
}
