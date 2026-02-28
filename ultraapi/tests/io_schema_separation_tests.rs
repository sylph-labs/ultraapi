// Tests for input/output schema separation feature
// This tests the read_only and write_only field attributes

use ultraapi::prelude::*;

// Test model with write_only field (field appears in request but not in response)
#[api_model]
#[allow(dead_code)]
struct UserWithPassword {
    /// User ID (only in response)
    id: i64,
    /// Username
    username: String,
    /// Password (only in request - write only)
    #[write_only]
    password: String,
}

// Test model with read_only field
#[api_model]
struct UserReadOnly {
    /// User ID (only in response - read only)
    #[read_only]
    id: i64,
    /// Username
    username: String,
}

// Test model with both read_only and write_only fields
#[api_model]
struct UserComplete {
    /// User ID (only in response)
    #[read_only]
    id: i64,
    /// Username
    username: String,
    /// Password (only in request)
    #[write_only]
    password: String,
    /// Email (in both request and response)
    email: String,
}

#[test]
fn test_write_only_field_not_in_serialization() {
    // When serializing to response, password should be excluded
    let user = UserComplete {
        id: 1,
        username: "testuser".to_string(),
        password: "secret123".to_string(),
        email: "test@example.com".to_string(),
    };

    let json = serde_json::to_value(&user).unwrap();

    // id, username, email should be present
    assert!(json.get("id").is_some());
    assert!(json.get("username").is_some());
    assert!(json.get("email").is_some());

    // password should NOT be in the serialized output (write_only)
    assert!(json.get("password").is_none());
}

#[test]
fn test_read_only_field_not_in_deserialization() {
    // When deserializing from request, id should be ignored
    let json = serde_json::json!({
        "id": 999,  // This should be ignored
        "username": "testuser",
        "password": "secret123",
        "email": "test@example.com"
    });

    let user: UserComplete = serde_json::from_value(json).unwrap();

    // id should be default (0) because it was ignored in deserialization
    assert_eq!(user.id, 0);
    assert_eq!(user.username, "testuser");
    assert_eq!(user.password, "secret123");
    assert_eq!(user.email, "test@example.com");
}

#[test]
fn test_write_only_schema_property() {
    // Get schema from inventory (this is how UltraAPI generates schemas with patches applied)
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let user_complete = schemas
        .iter()
        .find(|s| s.name == "UserComplete")
        .expect("UserComplete should be in inventory");
    let schema = (user_complete.schema_fn)();

    // Password should have writeOnly: true
    if let Some(password_prop) = schema.properties.get("password") {
        assert!(
            password_prop.write_only,
            "password should have write_only = true"
        );
        assert!(
            !password_prop.read_only,
            "password should NOT have read_only = true"
        );
    } else {
        panic!("password field not found in schema");
    }

    // ID should have readOnly: true
    if let Some(id_prop) = schema.properties.get("id") {
        assert!(id_prop.read_only, "id should have read_only = true");
        assert!(!id_prop.write_only, "id should NOT have write_only = true");
    } else {
        panic!("id field not found in schema");
    }

    // Email should have neither
    if let Some(email_prop) = schema.properties.get("email") {
        assert!(!email_prop.read_only, "email should NOT have read_only");
        assert!(!email_prop.write_only, "email should NOT have write_only");
    } else {
        panic!("email field not found in schema");
    }
}

#[test]
fn test_read_only_schema_json_output() {
    // Test that readOnly appears in the JSON schema output
    // Get schema from inventory (this is how UltraAPI generates schemas with patches applied)
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let user_complete = schemas
        .iter()
        .find(|s| s.name == "UserComplete")
        .expect("UserComplete should be in inventory");
    let schema = (user_complete.schema_fn)();
    let json = schema.to_json_value();

    // Check readOnly in JSON output for id field
    if let Some(id_json) = json["properties"].get("id") {
        assert_eq!(id_json["readOnly"], serde_json::json!(true));
    } else {
        panic!("id field not in properties");
    }

    // Check writeOnly in JSON output for password field
    if let Some(password_json) = json["properties"].get("password") {
        assert_eq!(password_json["writeOnly"], serde_json::json!(true));
    } else {
        panic!("password field not in properties");
    }
}

// Test that separate input/output schemas can be generated for routes
// This is a basic test to ensure the infrastructure works
#[test]
fn test_model_in_inventory() {
    // Both models should be registered in inventory
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();

    let user_complete = schemas.iter().find(|s| s.name == "UserComplete");
    assert!(
        user_complete.is_some(),
        "UserComplete should be in inventory"
    );

    let schema = (user_complete.unwrap().schema_fn)();
    // Verify properties exist
    assert!(schema.properties.contains_key("id"));
    assert!(schema.properties.contains_key("username"));
    assert!(schema.properties.contains_key("password"));
    assert!(schema.properties.contains_key("email"));
}

// Backward compatibility test - existing models without read_only/write_only should work
#[api_model]
#[derive(Debug, Clone)]
struct SimpleModel {
    id: i64,
    name: String,
}

#[test]
fn test_backward_compatibility() {
    // Existing models should still work - get schema from inventory
    let schemas: Vec<_> = inventory::iter::<ultraapi::SchemaInfo>().collect();
    let simple_model = schemas
        .iter()
        .find(|s| s.name == "SimpleModel")
        .expect("SimpleModel should be in inventory");
    let schema = (simple_model.schema_fn)();

    // Properties should exist
    assert!(schema.properties.contains_key("id"));
    assert!(schema.properties.contains_key("name"));

    // Neither should be read_only or write_only
    if let Some(id_prop) = schema.properties.get("id") {
        assert!(!id_prop.read_only);
        assert!(!id_prop.write_only);
    }

    if let Some(name_prop) = schema.properties.get("name") {
        assert!(!name_prop.read_only);
        assert!(!name_prop.write_only);
    }
}
