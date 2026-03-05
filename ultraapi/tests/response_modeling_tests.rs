// Response Modeling Tests
// Tests for response serialization, alias, include-exclude, and optional field behavior

use std::collections::HashMap;
use ultraapi::prelude::*;

// ---- Test 1: Response Serialization ----

/// A basic response model
#[api_model]
#[derive(Debug, Clone)]
struct BasicResponse {
    id: i64,
    name: String,
    created_at: String,
}

#[test]
fn test_response_serialization_json() {
    let response = BasicResponse {
        id: 1,
        name: "Test".into(),
        created_at: "2024-01-01T00:00:00Z".into(),
    };
    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"name\":\"Test\""));
    assert!(json.contains("\"created_at\":\"2024-01-01T00:00:00Z\""));
}

#[test]
fn test_response_deserialization_json() {
    let json = r#"{"id":1,"name":"Test","created_at":"2024-01-01T00:00:00Z"}"#;
    let response: BasicResponse = ultraapi::serde_json::from_str(json).unwrap();
    assert_eq!(response.id, 1);
    assert_eq!(response.name, "Test");
}

#[test]
fn test_response_schema_generated() {
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas.iter().find(|s| s.name == "BasicResponse").unwrap();
    let schema = (info.schema_fn)();
    assert_eq!(schema.type_name, "object");
    assert!(schema.properties.contains_key("id"));
    assert!(schema.properties.contains_key("name"));
    assert!(schema.properties.contains_key("created_at"));
}

// ---- Test 2: Field Alias support (serde rename) ----

// NOTE: `#[api_model]` preserves serde rename attributes,
// reflecting aliases in JSON serialization/deserialization and schema generation.

// ---- Test 3: Skip Serialization support (serde skip) ----

// NOTE: `#[api_model]` preserves serde(skip),
// excluding skipped fields in both runtime JSON and generated schema.

// The framework does support Option<T> which maps to nullable in schema
// and is omitted when None during serialization (via the Serialize impl).

// ---- Test 4: Optional Field Behavior ----

/// A response with various optional field patterns
#[api_model]
#[derive(Debug, Clone)]
struct OptionalFieldsResponse {
    id: i64,
    /// Required name
    name: String,
    /// Optional nickname - represented as Option
    nickname: Option<String>,
    /// Optional email
    email: Option<String>,
}

#[test]
fn test_optional_field_some_serializes_value() {
    let response = OptionalFieldsResponse {
        id: 1,
        name: "Test".into(),
        nickname: Some("nick".into()),
        email: Some("test@example.com".into()),
    };
    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"nickname\":\"nick\""));
    assert!(json.contains("\"email\":\"test@example.com\""));
}

#[test]
fn test_optional_field_none_becomes_null() {
    let response = OptionalFieldsResponse {
        id: 1,
        name: "Test".into(),
        nickname: None,
        email: None,
    };
    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"name\":\"Test\""));
    // Optional fields are serialized as null when None (standard serde behavior)
    // This is different from skip_serializing which would omit the field entirely
    assert!(json.contains("\"nickname\":null") || !json.contains("\"nickname\""));
    assert!(json.contains("\"email\":null") || !json.contains("\"email\""));
}

#[test]
fn test_optional_field_deserialization() {
    // With all optional fields
    let json = r#"{"id":1,"name":"Test","nickname":"nick","email":"a@b.com"}"#;
    let response: OptionalFieldsResponse = ultraapi::serde_json::from_str(json).unwrap();
    assert_eq!(response.nickname, Some("nick".into()));
    assert_eq!(response.email, Some("a@b.com".into()));

    // Without optional fields
    let json2 = r#"{"id":2,"name":"Test2"}"#;
    let response2: OptionalFieldsResponse = ultraapi::serde_json::from_str(json2).unwrap();
    assert_eq!(response2.nickname, None);
    assert_eq!(response2.email, None);
}

#[test]
fn test_optional_field_schema_nullable() {
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas
        .iter()
        .find(|s| s.name == "OptionalFieldsResponse")
        .unwrap();
    let schema = (info.schema_fn)();

    // Required fields should not be nullable
    let id_prop = &schema.properties["id"];
    assert!(!id_prop.nullable);

    let name_prop = &schema.properties["name"];
    assert!(!name_prop.nullable);

    // Optional fields should be nullable
    let nickname_prop = &schema.properties["nickname"];
    assert!(nickname_prop.nullable);

    let email_prop = &schema.properties["email"];
    assert!(email_prop.nullable);

    // Optional fields should NOT be in required array
    assert!(!schema.required.contains(&"nickname".to_string()));
    assert!(!schema.required.contains(&"email".to_string()));
}

// ---- Test 5: Response with Vec of Objects ----

#[api_model]
#[derive(Debug, Clone)]
struct Item {
    id: i64,
    name: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct ItemsResponse {
    items: Vec<Item>,
    total: i64,
}

#[test]
fn test_nested_vec_response_serialization() {
    let response = ItemsResponse {
        items: vec![
            Item {
                id: 1,
                name: "A".into(),
            },
            Item {
                id: 2,
                name: "B".into(),
            },
        ],
        total: 2,
    };
    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"items\":["));
    assert!(json.contains("\"total\":2"));
}

#[test]
fn test_nested_vec_response_schema() {
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas.iter().find(|s| s.name == "ItemsResponse").unwrap();
    let schema = (info.schema_fn)();
    let items_prop = &schema.properties["items"];
    assert_eq!(items_prop.type_name, "array");
    assert!(items_prop.items.is_some());
}

// ---- Test 6: Complex Nested Objects ----

#[api_model]
#[derive(Debug, Clone)]
struct Address {
    street: String,
    city: String,
    zip: String,
}

#[api_model]
#[derive(Debug, Clone)]
struct UserWithProfile {
    id: i64,
    name: String,
    address: Address,
    settings: HashMap<String, String>,
}

#[test]
fn test_complex_nested_response_serialization() {
    let response = UserWithProfile {
        id: 1,
        name: "John".into(),
        address: Address {
            street: "123 Main St".into(),
            city: "Boston".into(),
            zip: "02101".into(),
        },
        settings: HashMap::from([
            ("theme".into(), "dark".into()),
            ("lang".into(), "en".into()),
        ]),
    };
    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"name\":\"John\""));
    assert!(json.contains("\"street\":\"123 Main St\""));
    assert!(json.contains("\"theme\":\"dark\""));
}

#[test]
fn test_complex_nested_response_schema() {
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas
        .iter()
        .find(|s| s.name == "UserWithProfile")
        .unwrap();
    let schema = (info.schema_fn)();

    // Check nested Address is collected
    let nested = (info.nested_fn)();
    assert!(nested.contains_key("Address"));

    // Check address property has $ref
    let addr_prop = &schema.properties["address"];
    assert!(addr_prop.ref_path.is_some());

    // Check HashMap becomes additionalProperties
    let settings_prop = &schema.properties["settings"];
    assert_eq!(settings_prop.type_name, "object");
    assert!(settings_prop.additional_properties.is_some());
}

// ---- Test 7: Field Alias (serde rename) with api_model ----

#[api_model]
#[derive(Debug, Clone)]
struct AliasedApiModelResponse {
    #[serde(rename = "userId")]
    user_id: i64,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[test]
fn test_alias_supported_with_api_model() {
    let response = AliasedApiModelResponse {
        user_id: 7,
        display_name: "Alice".into(),
    };

    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"userId\":7"));
    assert!(json.contains("\"displayName\":\"Alice\""));
    assert!(!json.contains("\"user_id\""));
    assert!(!json.contains("\"display_name\""));

    let parsed: AliasedApiModelResponse =
        ultraapi::serde_json::from_str(r#"{"userId":8,"displayName":"Bob"}"#).unwrap();
    assert_eq!(parsed.user_id, 8);
    assert_eq!(parsed.display_name, "Bob");

    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas
        .iter()
        .find(|s| s.name == "AliasedApiModelResponse")
        .unwrap();
    let schema = (info.schema_fn)();
    assert!(schema.properties.contains_key("userId"));
    assert!(schema.properties.contains_key("displayName"));
    assert!(!schema.properties.contains_key("user_id"));
    assert!(!schema.properties.contains_key("display_name"));
}

// ---- Test 8: serde(skip) with api_model ----

#[api_model]
#[derive(Debug, Clone)]
struct SkipApiModelResponse {
    id: i64,
    #[serde(skip)]
    internal: String,
}

#[test]
fn test_skip_supported_with_api_model() {
    let response = SkipApiModelResponse {
        id: 10,
        internal: "secret".into(),
    };

    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":10"));
    assert!(!json.contains("internal"));

    let parsed: SkipApiModelResponse = ultraapi::serde_json::from_str(r#"{"id":11}"#).unwrap();
    assert_eq!(parsed.id, 11);
    assert_eq!(parsed.internal, "");

    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas
        .iter()
        .find(|s| s.name == "SkipApiModelResponse")
        .unwrap();
    let schema = (info.schema_fn)();
    assert!(schema.properties.contains_key("id"));
    assert!(!schema.properties.contains_key("internal"));
    assert!(!schema.required.contains(&"internal".to_string()));
}

// ---- Test 8b: Custom #[alias] attribute with api_model ----

#[api_model]
#[derive(Debug, Clone)]
struct CustomAliasResponse {
    #[alias("userId")]
    user_id: i64,
    #[alias("displayName")]
    display_name: String,
}

#[test]
fn test_custom_alias_attribute() {
    let response = CustomAliasResponse {
        user_id: 7,
        display_name: "Alice".into(),
    };

    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"userId\":7"));
    assert!(json.contains("\"displayName\":\"Alice\""));
    assert!(!json.contains("\"user_id\""));
    assert!(!json.contains("\"display_name\""));

    let parsed: CustomAliasResponse =
        ultraapi::serde_json::from_str(r#"{"userId":8,"displayName":"Bob"}"#).unwrap();
    assert_eq!(parsed.user_id, 8);
    assert_eq!(parsed.display_name, "Bob");

    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas
        .iter()
        .find(|s| s.name == "CustomAliasResponse")
        .unwrap();
    let schema = (info.schema_fn)();
    assert!(schema.properties.contains_key("userId"));
    assert!(schema.properties.contains_key("displayName"));
}

// ---- Test 8c: Custom #[skip_serializing] attribute with api_model ----

#[api_model]
#[derive(Debug, Clone)]
struct CustomSkipSerializingResponse {
    id: i64,
    #[skip_serializing]
    internal_note: String,
}

#[test]
fn test_custom_skip_serializing_attribute() {
    let response = CustomSkipSerializingResponse {
        id: 10,
        internal_note: "secret note".into(),
    };

    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":10"));
    assert!(!json.contains("internal_note"));
    assert!(!json.contains("secret note"));

    // Should still deserialize the field
    let parsed: CustomSkipSerializingResponse =
        ultraapi::serde_json::from_str(r#"{"id":11,"internal_note":"loaded"}"#).unwrap();
    assert_eq!(parsed.id, 11);
    assert_eq!(parsed.internal_note, "loaded");
}

// ---- Test 8d: Custom #[skip_deserializing] attribute with api_model ----

#[api_model]
#[derive(Debug, Clone)]
struct CustomSkipDeserializingResponse {
    id: i64,
    #[skip_deserializing]
    computed_field: String,
}

#[test]
fn test_custom_skip_deserializing_attribute() {
    // computed_field is set to default value
    let response = CustomSkipDeserializingResponse {
        id: 10,
        computed_field: "computed_value".into(),
    };

    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":10"));
    assert!(json.contains("\"computed_field\":\"computed_value\""));

    // Should NOT deserialize the field from JSON - gets default (empty string)
    let parsed: CustomSkipDeserializingResponse =
        ultraapi::serde_json::from_str(r#"{"id":11,"computed_field":"should_be_ignored"}"#)
            .unwrap();
    assert_eq!(parsed.id, 11);
    assert_eq!(parsed.computed_field, ""); // Default value (empty string) because skip_deserializing
}

// ---- Test 8e: Custom #[skip] attribute with api_model ----

#[api_model]
#[derive(Debug, Clone)]
struct CustomSkipAllResponse {
    id: i64,
    #[skip]
    internal: String,
}

#[test]
fn test_custom_skip_attribute() {
    let response = CustomSkipAllResponse {
        id: 10,
        internal: "secret".into(),
    };

    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"id\":10"));
    assert!(!json.contains("internal"));

    let parsed: CustomSkipAllResponse = ultraapi::serde_json::from_str(r#"{"id":11}"#).unwrap();
    assert_eq!(parsed.id, 11);
    assert_eq!(parsed.internal, "");
}

// ---- Test 9: Enum Response ----

#[api_model]
#[derive(Debug, Clone, PartialEq)]
enum Status {
    Active,
    Inactive,
    Pending,
}

#[api_model]
#[derive(Debug, Clone)]
struct StatusResponse {
    status: Status,
    message: String,
}

#[test]
fn test_enum_response_serialization() {
    let response = StatusResponse {
        status: Status::Active,
        message: "All good".into(),
    };
    let json = ultraapi::serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"Active\""));
    assert!(json.contains("\"message\":\"All good\""));
}

#[test]
fn test_enum_response_schema() {
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas.iter().find(|s| s.name == "StatusResponse").unwrap();
    let schema = (info.schema_fn)();

    let status_prop = &schema.properties["status"];
    // Enum field should have $ref
    assert!(status_prop.ref_path.is_some());
}

// ============================================================================
// Response Model Shaping Tests (include/exclude/by_alias)
// ============================================================================

/// Test model for include/exclude testing
#[api_model]
#[derive(Debug, Clone)]
struct UserProfile {
    id: i64,
    username: String,
    email: String,
    password_hash: String,
    created_at: String,
    is_admin: bool,
}

/// Test model with nested objects for nested include/exclude
#[api_model]
#[derive(Debug, Clone)]
struct Order {
    order_id: i64,
    customer: UserProfile,
    total: f64,
    status: String,
}

/// Test model with alias for by_alias testing
#[api_model]
#[derive(Debug, Clone)]
struct ApiUser {
    #[serde(rename = "userId")]
    user_id: i64,
    #[serde(rename = "displayName")]
    display_name: String,
    email: String,
    internal_note: String,
}

// ---- Test: ResponseModelOptions apply() method ----

#[test]
fn test_response_model_include() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["id", "name"]),
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "name": "Test",
        "created_at": "2024-01-01",
        "extra_field": "should be removed"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("name"));
    assert!(!obj.contains_key("created_at"));
    assert!(!obj.contains_key("extra_field"));
}

#[test]
fn test_response_model_exclude() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: Some(&["password_hash", "internal_note"]),
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "username": "testuser",
        "password_hash": "secret123",
        "email": "test@example.com",
        "internal_note": "confidential"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("username"));
    assert!(obj.contains_key("email"));
    assert!(!obj.contains_key("password_hash"));
    assert!(!obj.contains_key("internal_note"));
}

#[test]
fn test_response_model_include_takes_precedence() {
    // When both include and exclude are specified, include takes precedence
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["id", "username"]),
        exclude: Some(&["email"]),
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "username": "testuser",
        "email": "test@example.com"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    // Include takes precedence - only id and username should be present
    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("username"));
    assert!(!obj.contains_key("email"));
}

#[test]
fn test_response_model_nested_include_exclude() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["order_id", "customer.email", "items.*.sku"]),
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "order_id": 123,
        "customer": {
            "id": 1,
            "username": "testuser",
            "password_hash": "secret",
            "email": "test@example.com"
        },
        "items": [
            {"sku": "A-1", "qty": 2, "secret": "x"},
            {"sku": "B-2", "qty": 1, "secret": "y"}
        ],
        "total": 99.99,
        "status": "pending"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert!(obj.contains_key("order_id"));
    assert!(obj.contains_key("customer"));
    assert!(obj.contains_key("items"));
    assert!(!obj.contains_key("total"));
    assert!(!obj.contains_key("status"));

    let customer = obj.get("customer").unwrap().as_object().unwrap();
    assert!(!customer.contains_key("id"));
    assert!(!customer.contains_key("username"));
    assert!(!customer.contains_key("password_hash"));
    assert_eq!(customer.get("email").unwrap(), "test@example.com");

    let items = obj.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        let item = item.as_object().unwrap();
        assert!(item.contains_key("sku"));
        assert!(!item.contains_key("qty"));
        assert!(!item.contains_key("secret"));
    }
}

#[test]
fn test_response_model_nested_include_parent_field_keeps_subtree() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["order_id", "customer", "items.*.sku"]),
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "order_id": 321,
        "customer": {
            "id": 10,
            "email": "parent@example.com",
            "password_hash": "keep-me"
        },
        "items": [
            {"sku": "A-1", "qty": 2, "internal_code": "x"},
            {"sku": "B-2", "qty": 1, "internal_code": "y"}
        ],
        "status": "pending"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert_eq!(obj.get("order_id"), Some(&ultraapi::serde_json::json!(321)));

    let customer = obj.get("customer").unwrap().as_object().unwrap();
    assert_eq!(customer.get("id"), Some(&ultraapi::serde_json::json!(10)));
    assert_eq!(
        customer.get("email"),
        Some(&ultraapi::serde_json::json!("parent@example.com"))
    );
    assert_eq!(
        customer.get("password_hash"),
        Some(&ultraapi::serde_json::json!("keep-me"))
    );

    let items = obj.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        let item = item.as_object().unwrap();
        assert!(item.contains_key("sku"));
        assert!(!item.contains_key("qty"));
        assert!(!item.contains_key("internal_code"));
    }

    assert!(!obj.contains_key("status"));
}

#[test]
fn test_response_model_nested_include_array_wildcard_keeps_item_subtree() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["order_id", "items.*"]),
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "order_id": 777,
        "items": [
            {"sku": "A-1", "qty": 2, "internal_code": "x"},
            {"sku": "B-2", "qty": 1, "internal_code": "y"}
        ],
        "status": "pending"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert_eq!(obj.get("order_id"), Some(&ultraapi::serde_json::json!(777)));
    assert!(!obj.contains_key("status"));

    let items = obj.get("items").unwrap().as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0],
        ultraapi::serde_json::json!({"sku": "A-1", "qty": 2, "internal_code": "x"})
    );
    assert_eq!(
        items[1],
        ultraapi::serde_json::json!({"sku": "B-2", "qty": 1, "internal_code": "y"})
    );
}

#[test]
fn test_response_model_nested_exclude_with_array_wildcard() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: Some(&["customer.password_hash", "items.*.secret", "internal_note"]),
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "order_id": 555,
        "customer": {
            "id": 1,
            "email": "alice@example.com",
            "password_hash": "hidden"
        },
        "items": [
            {"sku": "A-1", "secret": "x"},
            {"sku": "B-2", "secret": "y"}
        ],
        "internal_note": "private"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert!(!obj.contains_key("internal_note"));

    let customer = obj.get("customer").unwrap().as_object().unwrap();
    assert!(customer.contains_key("id"));
    assert!(customer.contains_key("email"));
    assert!(!customer.contains_key("password_hash"));

    let items = obj.get("items").unwrap().as_array().unwrap();
    for item in items {
        let item = item.as_object().unwrap();
        assert!(item.contains_key("sku"));
        assert!(!item.contains_key("secret"));
    }
}

#[test]
fn test_response_model_array_handling() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["id", "name"]),
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!([
        {"id": 1, "name": "A", "extra": "x"},
        {"id": 2, "name": "B", "extra": "y"},
        {"id": 3, "name": "C", "extra": "z"}
    ]);

    let result = options.apply(value);
    let arr = result.as_array().unwrap();

    assert_eq!(arr.len(), 3);
    for item in arr {
        let obj = item.as_object().unwrap();
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("name"));
        assert!(!obj.contains_key("extra"));
    }
}

#[test]
fn test_response_model_by_alias() {
    // Note: by_alias controls whether serde uses the renamed field names.
    // The actual serialization uses serde's behavior, but by_alias=true
    // is stored in options for potential future use with custom serializers.
    // The main behavior is that api_model types with serde(rename) already
    // serialize with aliases by default.

    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: true,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    // Without include/exclude, the value passes through unchanged
    let value = ultraapi::serde_json::json!({
        "userId": 1,
        "displayName": "Test User"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    // Keys should remain as-is (no filtering applied)
    assert!(obj.contains_key("userId"));
    assert!(obj.contains_key("displayName"));
}

#[test]
fn test_response_model_empty_options() {
    let options = ultraapi::ResponseModelOptions::default();

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "name": "Test",
        "extra": "should remain"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    // All fields should pass through
    assert_eq!(obj.len(), 3);
    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("name"));
    assert!(obj.contains_key("extra"));
}

#[test]
fn test_response_model_exclude_none_filters_null_fields() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
        exclude_none: true,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "name": "Test",
        "nickname": null,
        "profile": {
            "bio": null,
            "city": "Tokyo"
        }
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("name"));
    assert!(!obj.contains_key("nickname"));

    let profile = obj.get("profile").unwrap().as_object().unwrap();
    assert!(!profile.contains_key("bio"));
    assert_eq!(profile.get("city").unwrap(), "Tokyo");
}

#[test]
fn test_response_model_exclude_unset_keeps_explicit_null_and_empty_containers() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: true,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "name": "Test",
        "nickname": null,
        "tags": [],
        "meta": {}
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("name"));
    assert_eq!(
        obj.get("nickname"),
        Some(&ultraapi::serde_json::Value::Null)
    );
    assert_eq!(obj.get("tags"), Some(&ultraapi::serde_json::json!([])));
    assert_eq!(obj.get("meta"), Some(&ultraapi::serde_json::json!({})));
}

#[test]
fn test_response_model_exclude_defaults_without_metadata_keeps_values() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: true,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 0,
        "name": "",
        "enabled": false,
        "score": 0.0,
        "empty_list": [],
        "empty_obj": {},
        "present": "value",
        "active": true,
        "count": 3
    });

    let result = options.apply(value.clone());
    assert_eq!(result, value);
}

fn default_enabled_true() -> bool {
    true
}

fn default_retry_count() -> i64 {
    5
}

#[api_model]
#[derive(Debug, Clone)]
struct FieldAwareDefaultsResponse {
    #[serde(default = "default_enabled_true")]
    enabled: bool,
    #[serde(default = "default_retry_count")]
    retry_count: i64,
    #[serde(default)]
    tags: Vec<String>,
    required_flag: bool,
}

#[test]
fn test_response_model_exclude_defaults_uses_api_model_field_defaults() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: true,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "enabled": true,
        "retry_count": 5,
        "tags": [],
        "required_flag": false
    });

    let result = options.apply_with_aliases(value, Some("FieldAwareDefaultsResponse"), false);
    let obj = result.as_object().unwrap();

    // Declared defaults should be removed.
    assert!(obj.get("enabled").is_none());
    assert!(obj.get("retry_count").is_none());
    assert!(obj.get("tags").is_none());
    // Required field has no default metadata; falsy values should be retained.
    assert_eq!(
        obj.get("required_flag"),
        Some(&ultraapi::serde_json::json!(false))
    );

    let non_default_value = ultraapi::serde_json::json!({
        "enabled": false,
        "retry_count": 3,
        "tags": ["x"],
        "required_flag": false
    });

    let non_default_result =
        options.apply_with_aliases(non_default_value, Some("FieldAwareDefaultsResponse"), false);
    let non_default_obj = non_default_result.as_object().unwrap();
    assert_eq!(
        non_default_obj.get("enabled"),
        Some(&ultraapi::serde_json::json!(false))
    );
    assert_eq!(
        non_default_obj.get("retry_count"),
        Some(&ultraapi::serde_json::json!(3))
    );
    assert_eq!(
        non_default_obj.get("tags"),
        Some(&ultraapi::serde_json::json!(["x"]))
    );
}

#[test]
fn test_response_model_no_matching_include() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["nonexistent"]),
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "name": "Test"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    // No fields should be included since "nonexistent" doesn't match
    assert!(obj.is_empty());
}

#[test]
fn test_response_model_all_excluded() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: Some(&["id", "name", "everything"]),
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "id": 1,
        "name": "Test"
    });

    let result = options.apply(value);
    let obj = result.as_object().unwrap();

    // All fields should be excluded
    assert!(obj.is_empty());
}

// ============================================================================
// Tests for apply_with_aliases (new method with alias conversion support)
// ============================================================================

#[test]
fn test_apply_with_aliases_by_alias_true() {
    // Test that by_alias=true converts field names to aliases in output
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: true,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    // Simulate a serialized value with field names (from serde)
    let value = ultraapi::serde_json::json!({
        "user_id": 1,
        "display_name": "Test User",
        "email": "test@example.com"
    });

    // Apply with alias mapping for ApiUser type
    let result = options.apply_with_aliases(value, Some("ApiUser"), true);
    let obj = result.as_object().unwrap();

    // Should have alias keys in output
    assert!(obj.contains_key("userId"), "Should have alias 'userId'");
    assert!(
        obj.contains_key("displayName"),
        "Should have alias 'displayName'"
    );
    assert!(obj.contains_key("email"), "Should have 'email' (no alias)");
}

#[test]
fn test_apply_with_aliases_by_alias_false() {
    // Test that by_alias=false keeps field names in output
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "user_id": 1,
        "display_name": "Test User"
    });

    let result = options.apply_with_aliases(value, Some("ApiUser"), false);
    let obj = result.as_object().unwrap();

    // Should have field names in output (not aliases)
    assert!(
        obj.contains_key("user_id"),
        "Should have field name 'user_id'"
    );
    assert!(
        obj.contains_key("display_name"),
        "Should have field name 'display_name'"
    );
    // Should NOT have aliases
    assert!(
        !obj.contains_key("userId"),
        "Should NOT have alias 'userId'"
    );
}

#[test]
fn test_apply_with_aliases_include_by_alias_true() {
    // Test include + by_alias=true
    let options = ultraapi::ResponseModelOptions {
        include: Some(&["user_id", "display_name"]),
        exclude: None,
        by_alias: true,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "user_id": 1,
        "display_name": "Test User",
        "email": "test@example.com",
        "password_hash": "secret"
    });

    let result = options.apply_with_aliases(value, Some("ApiUser"), true);
    let obj = result.as_object().unwrap();

    // Should have aliases in output
    assert!(obj.contains_key("userId"), "Should have alias 'userId'");
    assert!(
        obj.contains_key("displayName"),
        "Should have alias 'displayName'"
    );
    // Should NOT have excluded fields
    assert!(
        !obj.contains_key("email"),
        "Should NOT have 'email' (not in include)"
    );
    assert!(
        !obj.contains_key("password_hash"),
        "Should NOT have 'password_hash' (not in include)"
    );
}

#[test]
fn test_apply_with_aliases_exclude_by_alias_true() {
    // Test exclude + by_alias=true
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: Some(&["password_hash"]),
        by_alias: true,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "user_id": 1,
        "display_name": "Test User",
        "email": "test@example.com",
        "password_hash": "secret"
    });

    let result = options.apply_with_aliases(value, Some("ApiUser"), true);
    let obj = result.as_object().unwrap();

    // Should have aliases in output
    assert!(obj.contains_key("userId"), "Should have alias 'userId'");
    assert!(
        obj.contains_key("displayName"),
        "Should have alias 'displayName'"
    );
    assert!(obj.contains_key("email"), "Should have 'email'");
    // Should NOT have excluded field
    assert!(
        !obj.contains_key("password_hash"),
        "Should NOT have 'password_hash'"
    );
}

#[test]
fn test_apply_with_aliases_nested() {
    // Test nested object with by_alias=true
    // Note: Nested objects are handled by recursive calls with the SAME type_name
    // This tests the recursion, not the specific nested type alias mapping
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: true,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    // Simulate a nested structure similar to what would come from serialization
    // The key thing is that the top-level gets transformed
    let value = ultraapi::serde_json::json!({
        "order_id": 100,
        "customer": {
            "user_id": 1,
            "display_name": "John Doe"
        },
        "total": 99.99
    });

    // Using ApiUser as the type - this tests that:
    // 1. Top-level keys are processed
    // 2. Nested objects are recursively processed with the same type
    let result = options.apply_with_aliases(value, Some("ApiUser"), true);
    let obj = result.as_object().unwrap();

    // Top-level should NOT have alias (order_id doesn't have alias in OrderWithUser)
    // The nested customer should have aliases (user_id -> userId)
    assert!(obj.contains_key("total"), "Should have 'total'");

    // Nested object should also have aliases (recursively processed)
    let customer = obj.get("customer").unwrap().as_object().unwrap();
    // Note: This uses the ApiUser alias mapping, not OrderWithUser's
    assert!(
        customer.contains_key("userId"),
        "Nested should have alias 'userId' (from ApiUser mapping)"
    );
}

#[test]
fn test_apply_with_aliases_array() {
    // Test array with by_alias=true
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: true,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!([
        {"user_id": 1, "display_name": "John"},
        {"user_id": 2, "display_name": "Jane"}
    ]);

    let result = options.apply_with_aliases(value, Some("ApiUser"), true);
    let arr = result.as_array().unwrap();

    assert_eq!(arr.len(), 2);

    // Each item should have aliases
    assert!(arr[0].as_object().unwrap().contains_key("userId"));
    assert!(arr[1].as_object().unwrap().contains_key("displayName"));
}

#[test]
fn test_apply_with_aliases_no_type_matching() {
    // Test when type name doesn't match any registered aliases
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: true,
        exclude_none: false,
        exclude_unset: false,
        exclude_defaults: false,
        content_type: None,
    };

    let value = ultraapi::serde_json::json!({
        "user_id": 1,
        "display_name": "Test User"
    });

    // Use a type name that doesn't exist in the inventory
    let result = options.apply_with_aliases(value, Some("NonExistentType"), true);
    let obj = result.as_object().unwrap();

    // Keys should remain unchanged (no alias conversion)
    assert!(
        obj.contains_key("user_id"),
        "Should keep field name when no alias mapping found"
    );
}

// ============================================================================
// Mixed serde + custom attribute tests (P3-3)
// ============================================================================

// ---- Test: serde(rename) + custom #[skip_serializing] on same struct ----

#[api_model]
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct MixedRenameSkipSer {
    #[serde(rename = "userId")]
    user_id: i64,
    #[skip_serializing]
    secret: String,
    visible: String,
}

#[test]
fn test_mixed_serde_rename_and_custom_skip_serializing() {
    let val = MixedRenameSkipSer {
        user_id: 42,
        secret: "hidden".into(),
        visible: "shown".into(),
    };
    let json = ultraapi::serde_json::to_string(&val).unwrap();
    // rename should work
    assert!(json.contains("\"userId\":42"), "rename failed: {json}");
    assert!(!json.contains("\"user_id\""), "rename not applied: {json}");
    // skip_serializing should work
    assert!(!json.contains("secret"), "skip_serializing failed: {json}");
    assert!(json.contains("\"visible\":\"shown\""));
}

// ---- Test: serde(rename) + custom #[read_only] on the same field ----

#[api_model]
#[derive(Debug, Clone)]
struct RenameWithReadOnly {
    #[serde(rename = "itemId")]
    #[read_only]
    item_id: i64,
    name: String,
}

#[test]
fn test_serde_rename_combined_with_read_only() {
    // read_only → skip_deserializing: field should serialize but not deserialize
    let val = RenameWithReadOnly {
        item_id: 99,
        name: "widget".into(),
    };
    let json = ultraapi::serde_json::to_string(&val).unwrap();
    assert!(
        json.contains("\"itemId\":99"),
        "rename+read_only ser: {json}"
    );

    // Deserialize without the read_only field — should succeed with default
    let parsed: RenameWithReadOnly =
        ultraapi::serde_json::from_str(r#"{"name":"gadget"}"#).unwrap();
    assert_eq!(parsed.item_id, 0); // default
    assert_eq!(parsed.name, "gadget");
}

// ---- Test: serde(rename) + custom #[write_only] on the same field ----

#[api_model]
#[derive(Debug, Clone)]
struct RenameWithWriteOnly {
    id: i64,
    #[serde(rename = "secretToken")]
    #[write_only]
    secret_token: String,
}

#[test]
fn test_serde_rename_combined_with_write_only() {
    // write_only → skip_serializing: field accepted on input, hidden on output
    let val = RenameWithWriteOnly {
        id: 1,
        secret_token: "abc123".into(),
    };
    let json = ultraapi::serde_json::to_string(&val).unwrap();
    assert!(
        !json.contains("secretToken"),
        "write_only should hide: {json}"
    );
    assert!(
        !json.contains("secret_token"),
        "write_only should hide: {json}"
    );

    // Should still deserialize via the renamed key
    let parsed: RenameWithWriteOnly =
        ultraapi::serde_json::from_str(r#"{"id":2,"secretToken":"xyz"}"#).unwrap();
    assert_eq!(parsed.id, 2);
    assert_eq!(parsed.secret_token, "xyz");
}

// ---- Test: serde(default) + custom #[skip_serializing] (passthrough preservation) ----

#[api_model]
#[derive(Debug, Clone)]
struct DefaultWithSkipSer {
    id: i64,
    #[serde(default)]
    #[skip_serializing]
    cached: String,
}

#[test]
fn test_serde_default_preserved_with_custom_skip_ser() {
    // serde(default) should be preserved so deserialization works without field
    let parsed: DefaultWithSkipSer = ultraapi::serde_json::from_str(r#"{"id":5}"#).unwrap();
    assert_eq!(parsed.id, 5);
    assert_eq!(parsed.cached, ""); // default empty string

    // skip_serializing should hide the field
    let val = DefaultWithSkipSer {
        id: 5,
        cached: "data".into(),
    };
    let json = ultraapi::serde_json::to_string(&val).unwrap();
    assert!(
        !json.contains("cached"),
        "skip_ser should hide cached: {json}"
    );
}

// ---- Test: serde(rename) on one field + custom #[alias] on another ----

#[api_model]
#[derive(Debug, Clone)]
struct MixedRenameAlias {
    #[serde(rename = "externalId")]
    external_id: i64,
    #[alias("displayLabel")]
    display_label: String,
}

#[test]
fn test_serde_rename_and_custom_alias_coexist() {
    let val = MixedRenameAlias {
        external_id: 10,
        display_label: "hello".into(),
    };
    let json = ultraapi::serde_json::to_string(&val).unwrap();
    assert!(json.contains("\"externalId\":10"), "serde rename: {json}");
    assert!(
        json.contains("\"displayLabel\":\"hello\""),
        "custom alias: {json}"
    );
    assert!(!json.contains("\"external_id\""));
    assert!(!json.contains("\"display_label\""));

    // Deserialize with renamed keys
    let parsed: MixedRenameAlias =
        ultraapi::serde_json::from_str(r#"{"externalId":20,"displayLabel":"world"}"#).unwrap();
    assert_eq!(parsed.external_id, 20);
    assert_eq!(parsed.display_label, "world");

    // Schema should use renamed keys
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let info = schemas
        .iter()
        .find(|s| s.name == "MixedRenameAlias")
        .unwrap();
    let schema = (info.schema_fn)();
    assert!(schema.properties.contains_key("externalId"));
    assert!(schema.properties.contains_key("displayLabel"));
}

// ---- Test: serde(skip) coexists with serde(rename) on different fields ----

#[api_model]
#[derive(Debug, Clone)]
struct SkipAndRename {
    #[serde(rename = "userName")]
    user_name: String,
    #[serde(skip)]
    cache_key: String,
    visible: bool,
}

#[test]
fn test_serde_skip_and_rename_different_fields() {
    let val = SkipAndRename {
        user_name: "alice".into(),
        cache_key: "k123".into(),
        visible: true,
    };
    let json = ultraapi::serde_json::to_string(&val).unwrap();
    assert!(json.contains("\"userName\":\"alice\""));
    assert!(!json.contains("cache_key"));
    assert!(json.contains("\"visible\":true"));

    let parsed: SkipAndRename =
        ultraapi::serde_json::from_str(r#"{"userName":"bob","visible":false}"#).unwrap();
    assert_eq!(parsed.user_name, "bob");
    assert_eq!(parsed.cache_key, ""); // default from skip
    assert!(!parsed.visible);
}

// ---- Test: serde(rename) + serde(default) + #[skip_deserializing] all on same field ----

#[api_model]
#[derive(Debug, Clone)]
struct TripleCombo {
    id: i64,
    #[serde(rename = "computedScore", default)]
    #[skip_deserializing]
    computed_score: f64,
}

#[test]
fn test_triple_combo_rename_default_skip_deser() {
    let val = TripleCombo {
        id: 1,
        computed_score: 99.5,
    };
    let json = ultraapi::serde_json::to_string(&val).unwrap();
    // Should serialize with renamed key
    assert!(json.contains("\"computedScore\":99.5"), "ser: {json}");

    // Should deserialize without the field (skip_deserializing + default)
    let parsed: TripleCombo = ultraapi::serde_json::from_str(r#"{"id":2}"#).unwrap();
    assert_eq!(parsed.id, 2);
    assert_eq!(parsed.computed_score, 0.0); // f64 default
}

#[api_model]
#[derive(Debug, Clone, Default)]
struct ExcludeUnsetNestedModel {
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
}

#[api_model]
#[derive(Debug, Clone)]
struct ExcludeUnsetRootModel {
    id: i64,
    #[serde(default)]
    nickname: Option<String>,
    #[serde(default)]
    nested: ExcludeUnsetNestedModel,
}

#[test]
fn test_apply_with_field_set_excludes_unset_fields_for_api_model() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: true,
        exclude_defaults: false,
        content_type: None,
    };

    let serialized = ultraapi::serde_json::json!({
        "id": 10,
        "nickname": null,
        "nested": {
            "note": null,
            "labels": []
        }
    });
    let request_payload = ultraapi::serde_json::json!({
        "id": 10,
        "nested": {}
    });
    let field_set = ultraapi::collect_present_field_paths(&request_payload);

    let result = options.apply_with_aliases_and_field_set(
        serialized,
        Some("ExcludeUnsetRootModel"),
        false,
        Some(&field_set),
    );

    assert_eq!(
        result,
        ultraapi::serde_json::json!({"id": 10, "nested": {}})
    );
}

#[test]
fn test_apply_with_field_set_keeps_explicit_null_and_empty_values_for_api_model() {
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
        exclude_none: false,
        exclude_unset: true,
        exclude_defaults: false,
        content_type: None,
    };

    let serialized = ultraapi::serde_json::json!({
        "id": 11,
        "nickname": null,
        "nested": {
            "note": null,
            "labels": []
        }
    });
    let request_payload = ultraapi::serde_json::json!({
        "id": 11,
        "nickname": null,
        "nested": {
            "note": null,
            "labels": []
        }
    });
    let field_set = ultraapi::collect_present_field_paths(&request_payload);

    let result = options.apply_with_aliases_and_field_set(
        serialized,
        Some("ExcludeUnsetRootModel"),
        false,
        Some(&field_set),
    );

    assert_eq!(
        result,
        ultraapi::serde_json::json!({
            "id": 11,
            "nickname": null,
            "nested": {
                "note": null,
                "labels": []
            }
        })
    );
}
