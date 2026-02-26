// OpenAPI Callback Attribute Tests
// `#[callback(...)]` 属性から OpenAPI Operation.callbacks を生成できることを確認する。

use serde_json::Value;
use ultraapi::axum;
use ultraapi::prelude::*;

// --- Callback route ---

#[api_model]
#[derive(Debug, Clone)]
struct PaymentCallbackEvent {
    transaction_id: String,
    amount: i64,
}

#[post("/callbacks/payment")]
#[tag("callbacks")]
async fn payment_callback(event: PaymentCallbackEvent) -> Result<PaymentCallbackEvent, ApiError> {
    Ok(event)
}

// --- Owner routes ---

#[api_model]
#[derive(Debug, Clone)]
struct SubscriptionRequest {
    callback_url: String,
    plan: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct SubscriptionResponse {
    id: String,
    status: String,
}

#[post("/subscriptions-with-callback")]
#[tag("subscriptions")]
#[callback(
    name = "paymentEvent",
    expression = "{$request.body#/callbackUrl}",
    route = __HAYAI_ROUTE_PAYMENT_CALLBACK
)]
async fn create_subscription_with_callback(
    _body: SubscriptionRequest,
) -> Result<SubscriptionResponse, ApiError> {
    Ok(SubscriptionResponse {
        id: "sub_123".to_string(),
        status: "active".to_string(),
    })
}

// 複数 callback を同一ルートに付与できること
#[post("/orders-with-callbacks")]
#[tag("orders")]
#[callback(
    name = "orderCreated",
    expression = "{$request.body#/webhookUrl}",
    route = __HAYAI_ROUTE_PAYMENT_CALLBACK
)]
#[callback(
    name = "orderUpdated",
    expression = "{$request.body#/statusCallbackUrl}",
    route = __HAYAI_ROUTE_PAYMENT_CALLBACK
)]
async fn create_order_with_callbacks(
    _body: SubscriptionRequest,
) -> Result<SubscriptionResponse, ApiError> {
    Ok(SubscriptionResponse {
        id: "order_123".to_string(),
        status: "pending".to_string(),
    })
}

// --- Helpers ---

async fn spawn_app_implicit() -> String {
    let app = UltraApiApp::new()
        .title("Callback Attribute Test")
        .version("1.0.0")
        .into_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_app_explicit() -> String {
    // 明示ルーティング: owner route のみ include（callback route は include しない）
    let router = UltraApiRouter::new("/api")
        .tag("subscriptions")
        .route(__HAYAI_ROUTE_CREATE_SUBSCRIPTION_WITH_CALLBACK)
        .route(__HAYAI_ROUTE_CREATE_ORDER_WITH_CALLBACKS);

    let app = UltraApiApp::new()
        .title("Callback Attribute Explicit")
        .version("1.0.0")
        .include(router)
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
async fn test_callback_attribute_generates_callbacks_implicit_routing() {
    let base = spawn_app_implicit().await;
    let json = fetch_openapi(&base).await;

    let paths = json["paths"].as_object().unwrap();
    let owner = &paths["/subscriptions-with-callback"]["post"];

    let callbacks = owner.get("callbacks").expect("callbacks should exist");
    assert!(callbacks.is_object());

    let payment_event = &callbacks["paymentEvent"];
    assert!(payment_event.is_object());
    assert!(payment_event.get("{$request.body#/callbackUrl}").is_some());

    let expr = &payment_event["{$request.body#/callbackUrl}"];
    assert!(expr.get("post").is_some());
}

#[tokio::test]
async fn test_callback_attribute_multiple_callbacks_implicit_routing() {
    let base = spawn_app_implicit().await;
    let json = fetch_openapi(&base).await;

    let paths = json["paths"].as_object().unwrap();
    let owner = &paths["/orders-with-callbacks"]["post"];

    let callbacks = owner.get("callbacks").expect("callbacks should exist");
    assert!(callbacks.is_object());

    assert!(callbacks.get("orderCreated").is_some());
    assert!(callbacks.get("orderUpdated").is_some());
}

#[tokio::test]
async fn test_callback_attribute_generates_callbacks_explicit_routing() {
    let base = spawn_app_explicit().await;
    let json = fetch_openapi(&base).await;

    let paths = json["paths"].as_object().unwrap();
    let owner = &paths["/api/subscriptions-with-callback"]["post"];

    let callbacks = owner.get("callbacks").expect("callbacks should exist");
    assert!(callbacks.is_object());

    let payment_event = &callbacks["paymentEvent"];
    assert!(payment_event.is_object());
    assert!(payment_event.get("{$request.body#/callbackUrl}").is_some());
}
