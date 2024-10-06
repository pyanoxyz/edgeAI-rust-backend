use actix_web::{test, App};
use crate::chats::chat::{chat, ChatRequest}; // Replace `my_crate` with your crate name

#[test]
async fn test_chat_endpoint() {
    let mut app = test::init_service(
        App::new().service(chat)
    ).await;

    let request_payload = ChatRequest {
        prompt: "Hello, how can I improve my Rust code?".to_string(),
        session_id: None,
    };

    let req = test::TestRequest::post()
        .uri("/chat")
        .set_json(&request_payload)
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let response_text = String::from_utf8(body.to_vec()).unwrap();

    println!("Response: {}", response_text);

    // Add assertions based on the expected response
    // For example, if you expect a specific JSON structure:
    // let response_json: serde_json::Value = serde_json::from_str(&response_text).unwrap();
    // assert_eq!(response_json["field"], "expected_value");
}
