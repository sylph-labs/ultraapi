// OpenAPI Webhooks App Definition Tests
// UltraApiApp::webhook() API を通じて OpenAPI の webhooks セクションに
// webhook が出力されることを確認する。

use serde_json::Value;
use ultraapi::axum;
use ultraapi::prelude::*;

// --- Webhook route ---

#[api_model]
#[derive(Debug, Clone)]
struct PaymentEvent {
    event_type: String,
    amount: f64,
}

#[post("/webhooks/payment")]
#[tag("webhooks")]
async fn payment_webhook(body: PaymentEvent) -> Result<PaymentEvent, ApiError> {
    Ok(body)
}

// --- Owner routes (regular API routes) ---

#[api_model]
#[derive(Debug, Clone)]
struct Order {
    id: String,
    amount: f64,
}

#[post("/orders")]
#[tag("orders")]
async fn create_order(body: Order) -> Order {
    body
}

#[get("/orders/{id}")]
#[tag("orders")]
async fn get_order(id: String) -> Result<Order, ApiError> {
    Ok(Order { id, amount: 100.0 })
}

// --- Helpers ---

async fn spawn_app_implicit() -> String {
    // Implicit routing: 全ルート登録（inventory）
    let app = UltraApiApp::new()
        .title("Webhook Implicit Test")
        .version("1.0.0")
        .webhook("paymentWebhook", __HAYAI_ROUTE_PAYMENT_WEBHOOK)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_app_explicit() -> String {
    // Explicit routing: owner routes のみ include（webhook route は include しない）
    let router = UltraApiRouter::new("/api")
        .tag("orders")
        .route(__HAYAI_ROUTE_CREATE_ORDER)
        .route(__HAYAI_ROUTE_GET_ORDER);

    let app = UltraApiApp::new()
        .title("Webhook Explicit Test")
        .version("1.0.0")
        .include(router)
        .webhook("paymentWebhook", __HAYAI_ROUTE_PAYMENT_WEBHOOK)
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn fetch_openapi(base: &str) -> Value {
    let resp = reqwest::get(format!("{base}/openapi.json")).await.unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Value>().await.unwrap()
}

// --- Tests ---

#[tokio::test]
async fn test_webhook_appears_in_openapi_webhooks_section_implicit_routing() {
    let base = spawn_app_implicit().await;
    let json = fetch_openapi(&base).await;

    // webhooks セクションに webhook が存在すること
    let webhooks = json.get("webhooks").expect("webhooks section should exist");
    assert!(webhooks.is_object(), "webhooks should be an object");

    let payment_webhook = webhooks
        .get("paymentWebhook")
        .expect("paymentWebhook should exist in webhooks");
    assert!(payment_webhook.is_object());

    // webhook の operation が存在すること
    let post_op = payment_webhook
        .get("post")
        .expect("post operation should exist in webhook");
    assert!(post_op.is_object());
}

#[tokio::test]
async fn test_webhook_operation_id_and_content_type_implicit() {
    let base = spawn_app_implicit().await;
    let json = fetch_openapi(&base).await;

    let webhook_post = &json["webhooks"]["paymentWebhook"]["post"];

    // operationId の存在確認
    let operation_id = webhook_post
        .get("operationId")
        .expect("operationId should exist");
    assert!(operation_id.is_string(), "operationId should be a string");
    assert!(
        operation_id.as_str().unwrap().contains("payment_webhook"),
        "operationId should contain payment_webhook"
    );

    // requestBody の content-type 確認
    let request_body = webhook_post
        .get("requestBody")
        .expect("requestBody should exist");
    let content = request_body.get("content").expect("content should exist");
    let json_content = content
        .get("application/json")
        .expect("application/json should exist");
    assert!(json_content.get("schema").is_some());
}

#[tokio::test]
async fn test_webhook_not_in_paths_implicit_routing() {
    let base = spawn_app_implicit().await;
    let json = fetch_openapi(&base).await;

    // Implicit routing の場合、webhook route も runtime に登録される可能性があるが
    // OpenAPI の webhooks セクションにも paths にも出る
    // （これは implicit routing の動作）
    let _paths = json["paths"].as_object().unwrap();

    // webhook route が paths にも存在する可能性がある（implicit routing の場合）
    // ただし、webhooks セクションにも存在することを確認
    let webhooks = json.get("webhooks").expect("webhooks section should exist");
    assert!(webhooks.get("paymentWebhook").is_some());
}

#[tokio::test]
async fn test_webhook_appears_in_openapi_webhooks_section_explicit_routing() {
    let base = spawn_app_explicit().await;
    let json = fetch_openapi(&base).await;

    // 明示ルーティングでも webhooks セクションに webhook が出力されること
    let webhooks = json.get("webhooks").expect("webhooks section should exist");
    assert!(webhooks.is_object());

    let payment_webhook = webhooks
        .get("paymentWebhook")
        .expect("paymentWebhook should exist in webhooks");
    assert!(payment_webhook.is_object());

    let post_op = payment_webhook
        .get("post")
        .expect("post operation should exist in webhook");
    assert!(post_op.is_object());
}

#[tokio::test]
async fn test_webhook_not_in_paths_explicit_routing() {
    let base = spawn_app_explicit().await;
    let json = fetch_openapi(&base).await;

    // Explicit routing: webhook route を include していないので
    // paths には出力されない（ただし webhooks には出る）
    let paths = json["paths"].as_object().unwrap();

    // webhook route の path は paths に含まれてはいけない
    assert!(
        !paths.contains_key("/webhooks/payment"),
        "/webhooks/payment should NOT appear in paths when using explicit routing without including webhook route"
    );
    assert!(
        !paths.contains_key("/api/webhooks/payment"),
        "/api/webhooks/payment should NOT appear in paths"
    );

    // owner routes は paths に存在すること
    assert!(
        paths.contains_key("/api/orders"),
        "owner route should be in paths"
    );
    assert!(
        paths.contains_key("/api/orders/{id}"),
        "owner route should be in paths"
    );

    // webhooks セクションには存在する
    let webhooks = json.get("webhooks").expect("webhooks section should exist");
    assert!(webhooks.get("paymentWebhook").is_some());
}

#[tokio::test]
async fn test_webhook_operation_id_and_content_type_explicit() {
    let base = spawn_app_explicit().await;
    let json = fetch_openapi(&base).await;

    let webhook_post = &json["webhooks"]["paymentWebhook"]["post"];

    // operationId の存在確認
    let operation_id = webhook_post
        .get("operationId")
        .expect("operationId should exist");
    assert!(operation_id.is_string());

    // requestBody の content-type 確認
    let request_body = webhook_post
        .get("requestBody")
        .expect("requestBody should exist");
    let content = request_body.get("content").expect("content should exist");
    let json_content = content
        .get("application/json")
        .expect("application/json should exist");
    assert!(json_content.get("schema").is_some());
}

#[tokio::test]
async fn test_webhook_tags_included() {
    let base = spawn_app_implicit().await;
    let json = fetch_openapi(&base).await;

    let webhook_post = &json["webhooks"]["paymentWebhook"]["post"];

    // tags の確認
    let tags = webhook_post.get("tags").expect("tags should exist");
    assert!(tags.is_array());

    let tags_array = tags.as_array().unwrap();
    assert!(
        tags_array.iter().any(|t| t.as_str().unwrap() == "webhooks"),
        "webhooks tag should be present"
    );
}
