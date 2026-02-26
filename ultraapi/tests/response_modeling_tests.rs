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

// ---- Test 2: Field Alias - NOTE: Using serde rename on api_model types is not supported ----

// NOTE: The framework's api_model macro generates its own Serialize/Deserialize.
// Using serde(rename) attribute would conflict. This is a capability gap test.
//
// To test: Check if schemars (which api_model uses) properly handles field names.
// The schema will use Rust field names, not any serde renames.

// ---- Test 3: Skip Serialization - NOTE: Using serde skip on api_model types is not supported ----

// NOTE: The framework's api_model macro generates its own Serialize/Deserialize.
// Using serde(skip) attribute would conflict. This is a capability gap test.

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
fn test_alias_not_supported_with_api_model() {
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
fn test_skip_not_supported_with_api_model() {
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
        ultraapi::serde_json::from_str(r#"{"id":11,"computed_field":"should_be_ignored"}"#).unwrap();
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
        include: Some(vec!["id", "name"]),
        exclude: None,
        by_alias: false,
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
        exclude: Some(vec!["password_hash", "internal_note"]),
        by_alias: false,
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
        include: Some(vec!["id", "username"]),
        exclude: Some(vec!["email"]),
        by_alias: false,
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
    // Note: The current implementation applies include/exclude at each level recursively.
    // This means if you include=["order_id", "customer"], nested objects inherit this filter.
    // For nested filtering, use exclude only.
    
    let options = ultraapi::ResponseModelOptions {
        // Using only exclude - this will filter password_hash at all levels
        include: None,
        exclude: Some(vec!["password_hash", "total", "status"]),
        by_alias: false,
    };
    
    let value = ultraapi::serde_json::json!({
        "order_id": 123,
        "customer": {
            "id": 1,
            "username": "testuser",
            "password_hash": "secret",
            "email": "test@example.com"
        },
        "total": 99.99,
        "status": "pending"
    });
    
    let result = options.apply(value);
    let obj = result.as_object().unwrap();
    
    // Should have order_id and customer (not excluded)
    assert!(obj.contains_key("order_id"));
    assert!(obj.contains_key("customer"));
    // total and status should be excluded
    assert!(!obj.contains_key("total"));
    assert!(!obj.contains_key("status"));
    
    // Nested: customer should have id, username, email but not password_hash
    let customer = obj.get("customer").unwrap().as_object().unwrap();
    assert!(customer.contains_key("id"));
    assert!(customer.contains_key("username"));
    assert!(!customer.contains_key("password_hash"));
    assert!(customer.contains_key("email"));
}

#[test]
fn test_response_model_array_handling() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(vec!["id", "name"]),
        exclude: None,
        by_alias: false,
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
fn test_response_model_no_matching_include() {
    let options = ultraapi::ResponseModelOptions {
        include: Some(vec!["nonexistent"]),
        exclude: None,
        by_alias: false,
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
        exclude: Some(vec!["id", "name", "everything"]),
        by_alias: false,
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
    assert!(obj.contains_key("displayName"), "Should have alias 'displayName'");
    assert!(obj.contains_key("email"), "Should have 'email' (no alias)");
}

#[test]
fn test_apply_with_aliases_by_alias_false() {
    // Test that by_alias=false keeps field names in output
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: false,
    };
    
    let value = ultraapi::serde_json::json!({
        "user_id": 1,
        "display_name": "Test User"
    });
    
    let result = options.apply_with_aliases(value, Some("ApiUser"), false);
    let obj = result.as_object().unwrap();
    
    // Should have field names in output (not aliases)
    assert!(obj.contains_key("user_id"), "Should have field name 'user_id'");
    assert!(obj.contains_key("display_name"), "Should have field name 'display_name'");
    // Should NOT have aliases
    assert!(!obj.contains_key("userId"), "Should NOT have alias 'userId'");
}

#[test]
fn test_apply_with_aliases_include_by_alias_true() {
    // Test include + by_alias=true
    let options = ultraapi::ResponseModelOptions {
        include: Some(vec!["user_id", "display_name"]),
        exclude: None,
        by_alias: true,
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
    assert!(obj.contains_key("displayName"), "Should have alias 'displayName'");
    // Should NOT have excluded fields
    assert!(!obj.contains_key("email"), "Should NOT have 'email' (not in include)");
    assert!(!obj.contains_key("password_hash"), "Should NOT have 'password_hash' (not in include)");
}

#[test]
fn test_apply_with_aliases_exclude_by_alias_true() {
    // Test exclude + by_alias=true
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: Some(vec!["password_hash"]),
        by_alias: true,
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
    assert!(obj.contains_key("displayName"), "Should have alias 'displayName'");
    assert!(obj.contains_key("email"), "Should have 'email'");
    // Should NOT have excluded field
    assert!(!obj.contains_key("password_hash"), "Should NOT have 'password_hash'");
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
    assert!(customer.contains_key("userId"), "Nested should have alias 'userId' (from ApiUser mapping)");
}

#[test]
fn test_apply_with_aliases_array() {
    // Test array with by_alias=true
    let options = ultraapi::ResponseModelOptions {
        include: None,
        exclude: None,
        by_alias: true,
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
    };
    
    let value = ultraapi::serde_json::json!({
        "user_id": 1,
        "display_name": "Test User"
    });
    
    // Use a type name that doesn't exist in the inventory
    let result = options.apply_with_aliases(value, Some("NonExistentType"), true);
    let obj = result.as_object().unwrap();
    
    // Keys should remain unchanged (no alias conversion)
    assert!(obj.contains_key("user_id"), "Should keep field name when no alias mapping found");
}
