use ultraapi::prelude::*;

#[get("/parser-invalid/unknown")]
#[response_model(unknown_option = true)]
async fn parser_invalid_unknown_option() -> String {
    "ok".to_string()
}

fn main() {}
