use ultraapi::prelude::*;

#[api_model]
struct ParserPassProfile {
    name: String,
    email: String,
}

#[api_model]
struct ParserPassPayload {
    id: i32,
    profile: ParserPassProfile,
    secret: String,
}

#[get("/parser-pass/ordered")]
#[response_model(
    exclude_defaults = true,
    content_type = "application/vnd.ultraapi+json",
    include = { "id", "profile": { "name" } },
    by_alias = false,
    exclude_none = true,
    exclude = { "secret" },
    exclude_unset = false
)]
async fn parser_pass_ordered() -> ParserPassPayload {
    ParserPassPayload {
        id: 1,
        profile: ParserPassProfile {
            name: "neo".to_string(),
            email: "neo@example.com".to_string(),
        },
        secret: "hidden".to_string(),
    }
}

#[get("/parser-pass/compact")]
#[response_model(include={"id","profile":{"email"}},exclude={"secret"},by_alias=true,exclude_none=false,exclude_unset=true,exclude_defaults=false,content_type="application/json")]
async fn parser_pass_compact() -> ParserPassPayload {
    ParserPassPayload {
        id: 2,
        profile: ParserPassProfile {
            name: "trinity".to_string(),
            email: "trinity@example.com".to_string(),
        },
        secret: "classified".to_string(),
    }
}

fn main() {}
