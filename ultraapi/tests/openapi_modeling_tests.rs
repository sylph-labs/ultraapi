// OpenAPI Advanced Tests
// Tests for OpenAPI extensions, callbacks, webhooks, and separate input-output schema behavior

use std::collections::HashMap;
use ultraapi::openapi::*;
use ultraapi::prelude::*;

// ---- Test 1: OpenAPI Extensions (x- prefix) ----

#[test]
fn test_extension_on_schema() {
    let mut props = HashMap::new();
    props.insert(
        "name".to_string(),
        Property {
            type_name: "string".to_string(),
            format: None,
            min_length: None,
            max_length: None,
            minimum: None,
            maximum: None,
            pattern: None,
            min_items: None,
            description: None,
            ref_path: None,
            items: None,
            nullable: false,
            example: None,
            additional_properties: None,
            read_only: false,
            write_only: false,
        },
    );

    let schema = Schema {
        type_name: "object".to_string(),
        properties: props,
        required: vec!["name".to_string()],
        description: Some("Test schema".to_string()),
        enum_values: None,
        example: None,
        one_of: None,
        discriminator: None,
    };

    let json = schema.to_json_value();
    // Basic schema should work
    assert_eq!(json["type"], "object");
    assert!(json["properties"].get("name").is_some());
}

// ---- Test 2: OpenAPI Extensions in Operations ----

#[test]
fn test_operation_with_extensions() {
    let operation = Operation {
        summary: Some("Test operation".to_string()),
        description: None,
        operation_id: Some("testOp".to_string()),
        tags: vec!["test".to_string()],
        parameters: vec![],
        request_body: None,
        responses: HashMap::new(),
        security: vec![],
        callbacks: HashMap::new(),
        deprecated: false,
        external_docs: None,
    };

    let json = serde_json::to_value(&operation).unwrap();
    assert_eq!(json["summary"], "Test operation");
    assert_eq!(json["operationId"], "testOp");
}

// ---- Test 3: Servers with Variables ----

#[test]
fn test_server_url_basic() {
    let server = Server {
        url: "https://api.example.com/v1".to_string(),
    };
    let json = serde_json::to_value(&server).unwrap();
    assert_eq!(json["url"], "https://api.example.com/v1");
}

// ---- Test 4: Security Schemes ----

#[test]
fn test_bearer_security_scheme() {
    let scheme = SecurityScheme::Bearer {
        bearer_format: Some("JWT".to_string()),
    };
    let json = serde_json::to_value(&scheme).unwrap();
    assert_eq!(json["type"], "http");
    assert_eq!(json["scheme"], "bearer");
    assert_eq!(json["bearerFormat"], "JWT");
}

#[test]
fn test_api_key_security_scheme() {
    let scheme = SecurityScheme::ApiKey {
        name: "X-API-Key".to_string(),
        location: "header".to_string(),
    };
    let json = serde_json::to_value(&scheme).unwrap();
    assert_eq!(json["type"], "apiKey");
    assert_eq!(json["name"], "X-API-Key");
    assert_eq!(json["in"], "header");
}

#[test]
fn test_oauth2_security_scheme() {
    use ultraapi::openapi::OAuth2Flows;
    let scheme = SecurityScheme::OAuth2(OAuth2Flows::default());
    let json = serde_json::to_value(&scheme).unwrap();
    assert_eq!(json["type"], "oauth2");
}

// ---- Test 5: Parameter Styles ----

#[test]
fn test_path_parameter_with_style() {
    let param = Parameter {
        name: "id",
        location: "path",
        required: true,
        schema: SchemaObject::new_type("string"),
        description: Some("The user ID"),
    };
    let json = serde_json::to_value(&param).unwrap();
    assert_eq!(json["name"], "id");
    assert_eq!(json["in"], "path");
    assert_eq!(json["required"], true);
    assert_eq!(json["description"], "The user ID");
}

// ---- Test 6: Request Body with Examples ----

#[test]
fn test_request_body_json() {
    let body = RequestBody {
        required: true,
        content_type: "application/json".to_string(),
        schema_ref: "#/components/schemas/User".to_string(),
    };
    let json = body.to_json_value();
    assert_eq!(json["required"], true);
    assert!(json["content"]["application/json"].is_object());
}

// ---- Test 7: Response Definition ----

#[test]
fn test_response_definition() {
    let response = ResponseDef {
        description: "A user".to_string(),
        schema_ref: None,
        content_type: None,
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["description"], "A user");
}

// ---- Test 8: Tags ----

#[test]
fn test_operation_tags() {
    let operation = Operation {
        summary: None,
        description: None,
        operation_id: None,
        tags: vec!["users".to_string(), "admin".to_string()],
        parameters: vec![],
        request_body: None,
        responses: HashMap::new(),
        security: vec![],
        callbacks: HashMap::new(),
        deprecated: false,
        external_docs: None,
    };
    let json = serde_json::to_value(&operation).unwrap();
    let tags = json["tags"].as_array().unwrap();
    assert!(tags.iter().any(|t| t == "users"));
    assert!(tags.iter().any(|t| t == "admin"));
}

// ---- Test 9: Discriminator Mapping ----

#[test]
fn test_discriminator_mapping() {
    let mut mapping = HashMap::new();
    mapping.insert("cat".to_string(), "#/components/schemas/Cat".to_string());
    mapping.insert("dog".to_string(), "#/components/schemas/Dog".to_string());

    let discriminator = Discriminator {
        property_name: "pet_type".to_string(),
        mapping,
    };

    let json = serde_json::to_value(&discriminator).unwrap();
    // Check the mapping was serialized (the key might be different due to serde rename)
    let mapping_obj = json.get("mapping");
    assert!(mapping_obj.is_some(), "Should have mapping field");
}

// ---- Test 10: Full OpenAPI Spec ----

#[test]
fn test_full_openapi_spec() {
    let spec = OpenApiSpec {
        openapi: "3.1.0".to_string(),
        info: Info {
            title: "Test API".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test API".to_string()),
            contact: Some(Contact {
                name: Some("Test".to_string()),
                email: Some("test@example.com".to_string()),
                url: Some("https://example.com".to_string()),
            }),
            license: Some(License {
                name: "MIT".to_string(),
                url: Some("https://opensource.org/licenses/MIT".to_string()),
            }),
        },
        servers: vec![Server {
            url: "https://api.example.com".to_string(),
        }],
        paths: HashMap::new(),
        webhooks: HashMap::new(),
        schemas: HashMap::new(),
        security_schemes: HashMap::new(),
    };

    let json = spec.to_json();
    assert_eq!(json["openapi"], "3.1.0");
    assert_eq!(json["info"]["title"], "Test API");
    assert_eq!(json["info"]["version"], "1.0.0");
    assert_eq!(json["info"]["description"], "A test API");
    assert!(json["servers"].as_array().unwrap().len() == 1);
}

// ---- Test 11: Separate Input-Output Schema (Capability Gap) ----

// NOTE: The framework does not have built-in support for generating separate
// input (request) and output (response) schemas from a single model.
//
// Currently, the same schema is used for both request body and response.
//
// Workaround: Users must create separate model types for input and output
// if they need different schemas (e.g., to exclude internal fields from response).
//
// This is a known capability gap - no automatic separation of input/output schemas.

#[api_model]
#[derive(Debug, Clone)]
struct UserInput {
    /// Username for login
    username: String,
    /// Password (should never appear in response)
    password: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct UserOutput {
    id: i64,
    /// Username (safe to show)
    username: String,
    // Note: password is intentionally excluded from output model
}

#[test]
fn test_input_output_are_separate_models() {
    // Verify both schemas exist
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();

    let input_info = schemas.iter().find(|s| s.name == "UserInput");
    let output_info = schemas.iter().find(|s| s.name == "UserOutput");

    assert!(input_info.is_some(), "Input schema should exist");
    assert!(output_info.is_some(), "Output schema should exist");

    // Check that input has password field
    let input_schema = (input_info.unwrap().schema_fn)();
    assert!(input_schema.properties.contains_key("password"));

    // Check that output does NOT have password field
    let output_schema = (output_info.unwrap().schema_fn)();
    assert!(!output_schema.properties.contains_key("password"));
}

// ---- Test 12: Callbacks ----

#[test]
fn test_callback_not_implemented() {
    let mut callback_responses = HashMap::new();
    callback_responses.insert(
        "200".to_string(),
        ResponseDef {
            description: "Callback accepted".to_string(),
            schema_ref: None,
            content_type: None,
        },
    );

    let callback_operation = Operation {
        summary: Some("Order callback receiver".to_string()),
        description: None,
        operation_id: Some("notifyOrderCreated".to_string()),
        tags: vec!["callbacks".to_string()],
        parameters: vec![],
        request_body: None,
        responses: callback_responses,
        security: vec![],
        callbacks: HashMap::new(),
        deprecated: false,
        external_docs: None,
    };

    let mut callback_path_item = HashMap::new();
    callback_path_item.insert("post".to_string(), callback_operation);

    let mut callback_expressions = HashMap::new();
    callback_expressions.insert(
        "{$request.body#/callbackUrl}".to_string(),
        callback_path_item,
    );

    let mut callbacks = HashMap::new();
    callbacks.insert("orderCreated".to_string(), callback_expressions);

    let mut responses = HashMap::new();
    responses.insert(
        "201".to_string(),
        ResponseDef {
            description: "Created".to_string(),
            schema_ref: None,
            content_type: None,
        },
    );

    let operation = Operation {
        summary: Some("Create order".to_string()),
        description: None,
        operation_id: Some("createOrder".to_string()),
        tags: vec!["orders".to_string()],
        parameters: vec![],
        request_body: None,
        responses,
        security: vec![],
        callbacks,
        deprecated: false,
        external_docs: None,
    };

    let json = serde_json::to_value(&operation).unwrap();
    let callback_post = &json["callbacks"]["orderCreated"]["{$request.body#/callbackUrl}"]["post"];
    assert!(callback_post.is_object());
    assert_eq!(callback_post["operationId"], "notifyOrderCreated");
    assert_eq!(
        callback_post["responses"]["200"]["description"],
        "Callback accepted"
    );
}

// ---- Test 13: Webhooks ----

#[test]
fn test_webhook_not_implemented() {
    let mut webhook_responses = HashMap::new();
    webhook_responses.insert(
        "202".to_string(),
        ResponseDef {
            description: "Webhook queued".to_string(),
            schema_ref: None,
            content_type: None,
        },
    );

    let webhook_operation = Operation {
        summary: Some("Order created webhook".to_string()),
        description: Some("Sent when a new order is created".to_string()),
        operation_id: Some("orderCreatedWebhook".to_string()),
        tags: vec!["webhooks".to_string()],
        parameters: vec![],
        request_body: None,
        responses: webhook_responses,
        security: vec![],
        callbacks: HashMap::new(),
        deprecated: false,
        external_docs: None,
    };

    let mut webhook_path_item = HashMap::new();
    webhook_path_item.insert("post".to_string(), webhook_operation);

    let mut webhooks = HashMap::new();
    webhooks.insert("orderCreated".to_string(), webhook_path_item);

    let spec = OpenApiSpec {
        openapi: "3.1.0".to_string(),
        info: Info {
            title: "Webhook API".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            contact: None,
            license: None,
        },
        servers: vec![],
        paths: HashMap::new(),
        webhooks,
        schemas: HashMap::new(),
        security_schemes: HashMap::new(),
    };

    let json = spec.to_json();
    let webhook_post = &json["webhooks"]["orderCreated"]["post"];
    assert!(webhook_post.is_object());
    assert_eq!(webhook_post["operationId"], "orderCreatedWebhook");
    assert_eq!(
        webhook_post["responses"]["202"]["description"],
        "Webhook queued"
    );
}

// ---- Test 14: API Key Authentication ----

#[get("/api-key-test/{id}")]
#[security("api_key")]
async fn api_key_test_route(id: i64) -> OpenApiJsonUser {
    OpenApiJsonUser {
        id,
        name: "api key test".into(),
    }
}

#[test]
fn test_api_key_security_on_route() {
    let app = UltraApiApp::new()
        .security_scheme(
            "api_key",
            SecurityScheme::ApiKey {
                name: "X-API-Key".to_string(),
                location: "header".to_string(),
            },
        )
        .include(UltraApiRouter::new("/test").route(__HAYAI_ROUTE_API_KEY_TEST_ROUTE));

    let resolved = app.resolve_routes();
    assert_eq!(resolved.len(), 1);
    // Security should be merged from route
    let merged = resolved[0].merged_security();
    assert!(merged.contains(&"api_key"));
}

// ---- Test 15: Multiple Auth Schemes ----

#[get("/multi-auth/{id}")]
#[security("bearer")]
#[security("api_key")]
async fn multi_auth_route(id: i64) -> OpenApiJsonUser {
    OpenApiJsonUser {
        id,
        name: "multi auth".into(),
    }
}

#[test]
fn test_multiple_security_schemes() {
    let app = UltraApiApp::new()
        .bearer_auth()
        .security_scheme(
            "api_key",
            SecurityScheme::ApiKey {
                name: "X-API-Key".to_string(),
                location: "header".to_string(),
            },
        )
        .include(UltraApiRouter::new("/test").route(__HAYAI_ROUTE_MULTI_AUTH_ROUTE));

    let resolved = app.resolve_routes();
    let merged = resolved[0].merged_security();
    assert!(merged.contains(&"bearer"));
    assert!(merged.contains(&"api_key"));
}

// ---- Test 16: Custom Header Parameters (Capability Gap) ----

// NOTE: The framework does not support header extraction via attributes.
// Users can access headers through request.extensions() in handlers.

// ---- Test 17: Cookie Parameters (Capability Gap) ----

// NOTE: The framework does not support cookie parameters natively.
// Users would need to access cookies through extensions or custom extractors.

// ---- Test 18: Deprecation (Capability Gap) ----

// NOTE: The framework doesn't currently process the #[deprecated] attribute
// to add deprecation: true to the OpenAPI spec.

// ---- Test 19: Operation Id ----

#[test]
fn test_route_has_handler_name() {
    let found =
        inventory::iter::<&ultraapi::RouteInfo>().find(|r| r.handler_name == "api_key_test_route");
    assert!(found.is_some());
    let info = found.unwrap();
    // handler_name is the Rust function name
    assert_eq!(info.handler_name, "api_key_test_route");
}

// ---- Test 20: Summary ----

// NOTE: The framework doesn't currently have a #[summary] attribute.
// Summary is derived from description or needs to be added.

// ---- Test 21: OpenAPI Spec Export ----

#[test]
fn test_openapi_spec_includes_all_components() {
    // Create a complete spec and verify all parts serialize correctly
    let mut schemas = HashMap::new();
    schemas.insert(
        "User".to_string(),
        Schema {
            type_name: "object".to_string(),
            properties: HashMap::new(),
            required: vec![],
            description: Some("A user".to_string()),
            enum_values: None,
            example: None,
            one_of: None,
            discriminator: None,
        },
    );

    let spec = OpenApiSpec {
        openapi: "3.1.0".to_string(),
        info: Info {
            title: "Complete API".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            contact: None,
            license: None,
        },
        servers: vec![],
        paths: HashMap::new(),
        webhooks: HashMap::new(),
        schemas,
        security_schemes: HashMap::new(),
    };

    let json = spec.to_json();
    assert!(json["components"]["schemas"]["User"].is_object());
}

// ---- Test 22: Field Deprecated (Capability Gap) ----

// NOTE: The framework doesn't support marking individual fields as deprecated
// in the schema via attributes like #[schema(deprecated)].
// Users would need to manually construct schemas if they need this.

// ---- Test 23: External Documentation ----

#[test]
fn test_external_docs_serialization() {
    let external_docs = ExternalDocs {
        description: Some("Find more info here".to_string()),
        url: "https://example.com/docs".to_string(),
    };

    let json = serde_json::to_value(&external_docs).unwrap();
    assert_eq!(json["description"], "Find more info here");
    assert_eq!(json["url"], "https://example.com/docs");
}

#[test]
fn test_external_docs_url_only() {
    let external_docs = ExternalDocs {
        description: None,
        url: "https://example.com/docs".to_string(),
    };

    let json = serde_json::to_value(&external_docs).unwrap();
    assert!(json.get("description").is_none());
    assert_eq!(json["url"], "https://example.com/docs");
}

// ---- Test 24: JSON Schema Additional Features ----

// These OpenAPI 3.1 features are not currently generated by the framework:
// - nullable vs type: [string, null] (OpenAPI 3.1)
// - discriminator.mapping with non-$ref values
// - schema dependencies
// - propertyNames constraint
// - uniqueItems constraint
// - maxContains/minContains for arrays
// - multipleOf constraint
// - exclusiveMinimum/exclusiveMaximum (now supported via minimum/maximum with exclusive: true in 3.0)

#[test]
fn test_min_items_constraint() {
    let prop = Property {
        type_name: "array".to_string(),
        format: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        pattern: None,
        min_items: Some(1),
        // max_items is not available in current Property struct
        // this test verifies min_items works
        description: None,
        ref_path: None,
        items: Some(Box::new(Property {
            type_name: "string".to_string(),
            format: None,
            min_length: None,
            max_length: None,
            minimum: None,
            maximum: None,
            pattern: None,
            min_items: None,
            description: None,
            ref_path: None,
            items: None,
            nullable: false,
            example: None,
            additional_properties: None,
            read_only: false,
            write_only: false,
        })),
        nullable: false,
        example: None,
        additional_properties: None,
        read_only: false,
        write_only: false,
    };

    let json = prop.to_json_value();
    assert_eq!(json["type"], "array");
    assert_eq!(json["minItems"], 1);
}

#[test]
fn test_pattern_constraint() {
    let prop = Property {
        type_name: "string".to_string(),
        format: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        pattern: Some("^[a-z]+$".to_string()),
        min_items: None,
        description: None,
        ref_path: None,
        items: None,
        nullable: false,
        example: None,
        additional_properties: None,
        read_only: false,
        write_only: false,
    };

    let json = prop.to_json_value();
    assert_eq!(json["pattern"], "^[a-z]+$");
}

// ---- Test 25: ReadOnly and WriteOnly ----

#[test]
fn test_read_only_property() {
    let prop = Property {
        type_name: "string".to_string(),
        format: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        pattern: None,
        min_items: None,
        description: None,
        ref_path: None,
        items: None,
        nullable: false,
        example: None,
        additional_properties: None,
        read_only: true,
        write_only: false,
    };

    let json = prop.to_json_value();
    assert_eq!(json["readOnly"], true);
    assert!(json.get("writeOnly").is_none());
}

#[test]
fn test_write_only_property() {
    let prop = Property {
        type_name: "string".to_string(),
        format: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        pattern: None,
        min_items: None,
        description: None,
        ref_path: None,
        items: None,
        nullable: false,
        example: None,
        additional_properties: None,
        read_only: false,
        write_only: true,
    };

    let json = prop.to_json_value();
    assert_eq!(json["writeOnly"], true);
    assert!(json.get("readOnly").is_none());
}

#[test]
fn test_read_and_write_only_false_by_default() {
    let prop = Property {
        type_name: "string".to_string(),
        format: None,
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        pattern: None,
        min_items: None,
        description: None,
        ref_path: None,
        items: None,
        nullable: false,
        example: None,
        additional_properties: None,
        read_only: false,
        write_only: false,
    };

    let json = prop.to_json_value();
    // Neither should be present when both are false
    assert!(json.get("readOnly").is_none());
    assert!(json.get("writeOnly").is_none());
}

// ---- Test 26: Operation Deprecated ----

#[test]
fn test_operation_deprecated() {
    let operation = Operation {
        summary: Some("Test operation".to_string()),
        description: None,
        operation_id: Some("testOp".to_string()),
        tags: vec!["test".to_string()],
        parameters: vec![],
        request_body: None,
        responses: HashMap::new(),
        security: vec![],
        callbacks: HashMap::new(),
        deprecated: true,
        external_docs: None,
    };

    let json = serde_json::to_value(&operation).unwrap();
    assert_eq!(json["deprecated"], true);
}

#[test]
fn test_operation_not_deprecated_by_default() {
    let operation = Operation {
        summary: Some("Test operation".to_string()),
        description: None,
        operation_id: Some("testOp".to_string()),
        tags: vec!["test".to_string()],
        parameters: vec![],
        request_body: None,
        responses: HashMap::new(),
        security: vec![],
        callbacks: HashMap::new(),
        deprecated: false,
        external_docs: None,
    };

    let json = serde_json::to_value(&operation).unwrap();
    assert!(json.get("deprecated").is_none());
}

#[test]
fn test_operation_with_external_docs() {
    let external_docs = ExternalDocs {
        description: Some("Full API docs".to_string()),
        url: "https://api.example.com/docs".to_string(),
    };

    let operation = Operation {
        summary: Some("Test operation".to_string()),
        description: None,
        operation_id: Some("testOp".to_string()),
        tags: vec!["test".to_string()],
        parameters: vec![],
        request_body: None,
        responses: HashMap::new(),
        security: vec![],
        callbacks: HashMap::new(),
        deprecated: false,
        external_docs: Some(external_docs),
    };

    let json = serde_json::to_value(&operation).unwrap();
    assert_eq!(json["externalDocs"]["url"], "https://api.example.com/docs");
    assert_eq!(json["externalDocs"]["description"], "Full API docs");
}

// Re-export OpenApiJsonUser from other test files for consistency
#[api_model]
#[derive(Debug, Clone)]
struct OpenApiJsonUser {
    id: i64,
    name: String,
}
