use cloudflare_azure_openai::{app, ChatCompletionRequest, Message};
use tokio::net::TcpListener;
use futures_util::StreamExt;

async fn start_app() -> String {
    let app = app();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    format!("http://{}", addr)
}

#[tokio::test]
async fn test_chat_completion() {
    let api_key = match std::env::var("api_key") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("api_key not set, skipping test");
            return;
        }
    };
    let base = start_app().await;
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![Message { role: "user".into(), content: "Hello".into() }],
        stream: false,
    };
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/chat/completions", base))
        .header("api-key", api_key)
        .json(&req)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let value: serde_json::Value = resp.json().await.unwrap();
    assert!(value["choices"].is_array());
}

#[tokio::test]
async fn test_chat_completion_stream() {
    let api_key = match std::env::var("api_key") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("api_key not set, skipping test");
            return;
        }
    };
    let base = start_app().await;
    let req = ChatCompletionRequest {
        model: "gpt-4o".to_string(),
        messages: vec![Message { role: "user".into(), content: "stream".into() }],
        stream: true,
    };
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v1/chat/completions", base))
        .header("api-key", api_key)
        .json(&req)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let mut stream = resp.bytes_stream();
    let mut got = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        if !chunk.is_empty() {
            got = true;
            break;
        }
    }
    assert!(got);
}
