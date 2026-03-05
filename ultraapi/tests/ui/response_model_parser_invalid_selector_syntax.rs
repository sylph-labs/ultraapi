use ultraapi::prelude::*;

#[get("/parser-invalid/selector")]
#[response_model(include = ["id", "profile"])]
async fn parser_invalid_selector_syntax() -> String {
    "ok".to_string()
}

fn main() {}
