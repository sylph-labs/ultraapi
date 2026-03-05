use ultraapi::prelude::*;

#[get("/parser-invalid/bool")]
#[response_model(by_alias = "true")]
async fn parser_invalid_bool() -> String {
    "ok".to_string()
}

fn main() {}
