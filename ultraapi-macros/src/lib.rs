use proc_macro::TokenStream;
use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use syn::{
    parse::Parser, parse_macro_input, punctuated::Punctuated, FnArg, ItemEnum, ItemFn, ItemStruct,
    LitInt, LitStr, PatType, Path, PathSegment, Type,
};

fn extract_inner_type(seg: &PathSegment) -> Option<&Type> {
    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
            return Some(inner);
        }
    }
    None
}

fn is_dep_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Dep";
        }
    }
    false
}

fn is_state_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "State";
        }
    }
    false
}

fn is_depends_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Depends";
        }
    }
    false
}

fn is_query_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Query";
        }
    }
    false
}

fn option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                return extract_inner_type(seg);
            }
        }
    }
    None
}

fn typed_header_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "TypedHeader" {
                return extract_inner_type(seg);
            }
            if seg.ident == "Option" {
                if let Some(inner) = extract_inner_type(seg) {
                    return typed_header_inner_type(inner);
                }
            }
        }
    }
    None
}

fn is_header_type(ty: &Type) -> bool {
    typed_header_inner_type(ty).is_some()
}

fn is_header_map_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "HeaderMap";
        }
    }
    false
}

fn is_request_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Request";
        }
    }
    false
}

fn is_cookie_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Cookie" || seg.ident == "CookieJar";
        }
    }
    false
}

fn is_form_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Form";
        }
    }
    false
}

fn is_multipart_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Multipart";
        }
    }
    false
}

fn is_session_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Session";
        }
    }
    false
}

fn is_background_tasks_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "BackgroundTasks";
        }
    }
    false
}

/// Check if the type is OAuth2PasswordBearer
fn is_oauth2_password_bearer_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "OAuth2PasswordBearer";
        }
    }
    false
}

/// Check if the type is OptionalOAuth2PasswordBearer
fn is_optional_oauth2_password_bearer_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "OptionalOAuth2PasswordBearer";
        }
    }
    false
}

/// Check if the type is OAuth2AuthorizationCodeBearer
fn is_oauth2_auth_code_bearer_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "OAuth2AuthorizationCodeBearer";
        }
    }
    false
}

/// Check if the type is OptionalOAuth2AuthorizationCodeBearer
fn is_optional_oauth2_auth_code_bearer_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "OptionalOAuth2AuthorizationCodeBearer";
        }
    }
    false
}

fn get_type_name(ty: &Type) -> String {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    "Unknown".to_string()
}

fn pat_ident_name(pat: &syn::Pat) -> Option<String> {
    if let syn::Pat::Ident(pi) = pat {
        let ident = pi.ident.to_string();
        if ident == "_" {
            return None;
        }

        let trimmed = ident.trim_start_matches('_');
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    } else {
        None
    }
}

/// Check if the type is Result<T, E> and return the Ok type
fn get_result_ok_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Result" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(ok_type)) = args.args.first() {
                        return Some(ok_type);
                    }
                }
            }
        }
    }
    None
}

/// Check if the type is Vec<T> and return the inner type name
fn get_vec_inner_type_name(ty: &Type) -> Option<String> {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Vec" {
                if let Some(inner) = extract_inner_type(seg) {
                    return Some(get_type_name(inner));
                }
            }
        }
    }
    None
}

fn resolve_dependency_type(ty: &Type) -> Type {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Depends" || seg.ident == "Dep" || seg.ident == "State" {
                if let Some(inner) = extract_inner_type(seg) {
                    return inner.clone();
                }
            }
        }
    }

    ty.clone()
}

fn parse_dependencies_attr(tokens: TokenStream2) -> syn::Result<Vec<Type>> {
    let parser = Punctuated::<Type, syn::Token![,]>::parse_terminated;
    let parsed = parser.parse2(tokens)?;

    if parsed.is_empty() {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "dependencies(...) requires at least one dependency type",
        ));
    }

    Ok(parsed.into_iter().collect())
}

fn is_primitive_type(ty: &Type) -> bool {
    let name = get_type_name(ty);
    matches!(
        name.as_str(),
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "f32"
            | "f64"
            | "String"
            | "bool"
    )
}

fn is_scalar_query_type(ty: &Type) -> bool {
    if is_primitive_type(ty) {
        return true;
    }

    if let Some(inner) = option_inner_type(ty) {
        return is_primitive_type(inner);
    }

    false
}

/// Extract doc comment string from attributes
fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let syn::Meta::NameValue(nv) = &attr.meta {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    lines.push(s.value().trim().to_string());
                }
            }
        }
    }
    lines.join("\n").trim().to_string()
}

#[derive(Default)]
struct ParsedResponseModelArgs {
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    by_alias: Option<bool>,
    exclude_none: Option<bool>,
    exclude_unset: Option<bool>,
    exclude_defaults: Option<bool>,
    content_type: Option<String>,
}

fn parse_bool_token(token: &TokenTree) -> Option<bool> {
    match token {
        TokenTree::Ident(ident) if ident == "true" => Some(true),
        TokenTree::Ident(ident) if ident == "false" => Some(false),
        _ => None,
    }
}

fn parse_string_token(token: &TokenTree) -> Option<String> {
    if let TokenTree::Literal(lit) = token {
        syn::parse_str::<LitStr>(&lit.to_string())
            .ok()
            .map(|s| s.value())
    } else {
        None
    }
}

fn parse_selector_key_token(token: &TokenTree) -> Option<String> {
    if let Some(value) = parse_string_token(token) {
        return Some(value);
    }

    match token {
        TokenTree::Ident(ident) => Some(ident.to_string()),
        TokenTree::Literal(lit) => Some(lit.to_string()),
        _ => None,
    }
}

fn normalize_selector_segment(segment: &str) -> String {
    match segment {
        "__all__" | "*" => "*".to_string(),
        _ => segment.to_string(),
    }
}

fn join_selector_path(prefix: &str, segment: &str) -> String {
    let segment = normalize_selector_segment(segment);
    if prefix.is_empty() {
        segment
    } else {
        format!("{}.{}", prefix, segment)
    }
}

fn parse_selector_group(group: &proc_macro2::Group, prefix: &str) -> syn::Result<Vec<String>> {
    if group.delimiter() != Delimiter::Brace {
        return Err(syn::Error::new(
            group.span(),
            "include/exclude value must use {...} set/dict syntax",
        ));
    }

    let mut paths = Vec::new();
    let tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    let mut idx = 0usize;

    while idx < tokens.len() {
        if matches!(&tokens[idx], TokenTree::Punct(p) if p.as_char() == ',') {
            idx += 1;
            continue;
        }

        let key = parse_selector_key_token(&tokens[idx]).ok_or_else(|| {
            syn::Error::new(
                tokens[idx].span(),
                "include/exclude key must be a string literal or identifier",
            )
        })?;
        idx += 1;

        let path = join_selector_path(prefix, &key);

        let has_nested =
            idx < tokens.len() && matches!(&tokens[idx], TokenTree::Punct(p) if p.as_char() == ':');

        if has_nested {
            idx += 1;
            if idx >= tokens.len() {
                return Err(syn::Error::new(
                    group.span(),
                    "expected nested selector after ':'",
                ));
            }

            match &tokens[idx] {
                TokenTree::Group(nested) if nested.delimiter() == Delimiter::Brace => {
                    let nested_paths = parse_selector_group(nested, &path)?;
                    paths.extend(nested_paths);
                }
                token => {
                    if parse_bool_token(token).unwrap_or(false) {
                        paths.push(path.clone());
                    } else {
                        return Err(syn::Error::new(
                            token.span(),
                            "nested selector must be {...} or boolean",
                        ));
                    }
                }
            }
            idx += 1;
        } else {
            paths.push(path);
        }

        if idx < tokens.len() {
            if matches!(&tokens[idx], TokenTree::Punct(p) if p.as_char() == ',') {
                idx += 1;
            } else {
                return Err(syn::Error::new(
                    tokens[idx].span(),
                    "expected ',' between include/exclude entries",
                ));
            }
        }
    }

    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn parse_response_model_args(tokens: TokenStream2) -> syn::Result<ParsedResponseModelArgs> {
    let mut parsed = ParsedResponseModelArgs::default();

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("include") {
            let value = meta.value()?;
            let token: TokenTree = value.parse()?;
            match token {
                TokenTree::Group(group) => {
                    parsed.include = Some(parse_selector_group(&group, "")?);
                }
                token => {
                    return Err(syn::Error::new(
                        token.span(),
                        "response_model(include=...) must use {...} syntax",
                    ));
                }
            }
        } else if meta.path.is_ident("exclude") {
            let value = meta.value()?;
            let token: TokenTree = value.parse()?;
            match token {
                TokenTree::Group(group) => {
                    parsed.exclude = Some(parse_selector_group(&group, "")?);
                }
                token => {
                    return Err(syn::Error::new(
                        token.span(),
                        "response_model(exclude=...) must use {...} syntax",
                    ));
                }
            }
        } else if meta.path.is_ident("by_alias") {
            let value = meta.value()?;
            let token: TokenTree = value.parse()?;
            parsed.by_alias = Some(parse_bool_token(&token).ok_or_else(|| {
                syn::Error::new(
                    token.span(),
                    "response_model(by_alias=...) expects true/false",
                )
            })?);
        } else if meta.path.is_ident("exclude_none") {
            let value = meta.value()?;
            let token: TokenTree = value.parse()?;
            parsed.exclude_none = Some(parse_bool_token(&token).ok_or_else(|| {
                syn::Error::new(
                    token.span(),
                    "response_model(exclude_none=...) expects true/false",
                )
            })?);
        } else if meta.path.is_ident("exclude_unset") {
            let value = meta.value()?;
            let token: TokenTree = value.parse()?;
            parsed.exclude_unset = Some(parse_bool_token(&token).ok_or_else(|| {
                syn::Error::new(
                    token.span(),
                    "response_model(exclude_unset=...) expects true/false",
                )
            })?);
        } else if meta.path.is_ident("exclude_defaults") {
            let value = meta.value()?;
            let token: TokenTree = value.parse()?;
            parsed.exclude_defaults = Some(parse_bool_token(&token).ok_or_else(|| {
                syn::Error::new(
                    token.span(),
                    "response_model(exclude_defaults=...) expects true/false",
                )
            })?);
        } else if meta.path.is_ident("content_type") {
            let value = meta.value()?;
            let token: TokenTree = value.parse()?;
            parsed.content_type = Some(parse_string_token(&token).ok_or_else(|| {
                syn::Error::new(
                    token.span(),
                    "response_model(content_type=...) expects a string literal",
                )
            })?);
        } else {
            return Err(meta.error(
                "unsupported response_model option (expected include/exclude/by_alias/exclude_none/exclude_unset/exclude_defaults/content_type)",
            ));
        }

        Ok(())
    });

    parser.parse2(tokens)?;
    Ok(parsed)
}

fn route_macro_impl(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr).value();
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    // Parse custom attributes: #[status(N)], #[tag("x")], #[security("x")],
    // #[dependencies(...)], #[response_model(include={"a","b"})],
    // #[response_model(exclude={"a","b"})], #[response_model(by_alias=true)],
    // #[response_class("json"|"html"|"text"|"binary"|"stream"|"xml")], doc comments
    let mut status_code: Option<u16> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut security_schemes: Vec<String> = Vec::new();
    let mut route_dependencies: Vec<Type> = Vec::new();
    // Response model shaping options
    let mut has_response_model: bool = false; // Track if #[response_model(...)] was used
    let mut include_fields: Option<Vec<String>> = None;
    let mut exclude_fields: Option<Vec<String>> = None;
    let mut by_alias: bool = false;
    // FastAPI parity options for response_model
    let mut exclude_none: bool = false;
    let mut exclude_unset: bool = false;
    let mut exclude_defaults: bool = false;
    // Response model content-type override
    let mut response_model_content_type: Option<String> = None;
    // Response class (default: json)
    let mut response_class: Option<String> = None;
    // OpenAPI metadata extensions
    let mut summary: Option<String> = None;
    let mut deprecated: bool = false;
    let mut external_docs_url: Option<String> = None;
    let mut external_docs_description: Option<String> = None;
    let description = extract_doc_comment(&input_fn.attrs);

    let mut clean_attrs: Vec<&syn::Attribute> = Vec::new();
    for attr in &input_fn.attrs {
        if attr.path().is_ident("status") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitInt>(tokens) {
                    status_code = Some(lit.base10_parse().unwrap());
                }
            }
        } else if attr.path().is_ident("tag") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    tags.push(lit.value());
                }
            }
        } else if attr.path().is_ident("security") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    security_schemes.push(lit.value());
                }
            }
        } else if attr.path().is_ident("dependencies") {
            if let syn::Meta::List(list) = &attr.meta {
                let parsed = match parse_dependencies_attr(list.tokens.clone()) {
                    Ok(parsed) => parsed,
                    Err(err) => return err.to_compile_error().into(),
                };
                route_dependencies.extend(parsed);
            } else {
                return syn::Error::new_spanned(
                    attr,
                    "dependencies attribute must be used as #[dependencies(...)]",
                )
                .to_compile_error()
                .into();
            }
        } else if attr.path().is_ident("response_model") {
            // Mark that response_model attribute was used
            has_response_model = true;

            if let syn::Meta::List(list) = &attr.meta {
                let parsed = match parse_response_model_args(list.tokens.clone()) {
                    Ok(parsed) => parsed,
                    Err(err) => return err.to_compile_error().into(),
                };

                if let Some(include) = parsed.include {
                    include_fields = Some(include);
                }
                if let Some(exclude) = parsed.exclude {
                    exclude_fields = Some(exclude);
                }
                if let Some(flag) = parsed.by_alias {
                    by_alias = flag;
                }
                if let Some(flag) = parsed.exclude_none {
                    exclude_none = flag;
                }
                if let Some(flag) = parsed.exclude_unset {
                    exclude_unset = flag;
                }
                if let Some(flag) = parsed.exclude_defaults {
                    exclude_defaults = flag;
                }
                if let Some(content_type) = parsed.content_type {
                    response_model_content_type = Some(content_type);
                }
            }
        } else if attr.path().is_ident("response_class") {
            // Parse response_class("json"|"html"|"text"|"binary"|"stream"|"xml"|"file"|"redirect"|"cookie")
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    response_class = Some(lit.value());
                }
            }
        } else if attr.path().is_ident("summary") {
            // Parse summary("...")
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    summary = Some(lit.value());
                }
            }
        } else if attr.path().is_ident("deprecated") {
            // Parse deprecated - can be #[deprecated] or #[deprecated()]
            deprecated = true;
        } else if attr.path().is_ident("external_docs") {
            // Parse external_docs(url = "...", description = "...")
            if let syn::Meta::List(list) = &attr.meta {
                // Robustly parse: external_docs(url = "...", description = "...")
                // (avoid `to_string()` parsing which breaks on `https://...`)
                let parser = syn::meta::parser(|meta| {
                    if meta.path.is_ident("url") {
                        let value = meta.value()?;
                        let lit: LitStr = value.parse()?;
                        external_docs_url = Some(lit.value());
                        Ok(())
                    } else if meta.path.is_ident("description") {
                        let value = meta.value()?;
                        let lit: LitStr = value.parse()?;
                        external_docs_description = Some(lit.value());
                        Ok(())
                    } else {
                        // Ignore unknown keys for forward compatibility
                        Ok(())
                    }
                });

                // Parse errors should be surfaced as a compile error (DX)
                if let Err(err) = parser.parse2(list.tokens.clone()) {
                    return err.to_compile_error().into();
                }
            }
        } else if attr.path().is_ident("callback") {
            // Parse #[callback(name = "...", expression = "...", route = ROUTE_REF)]
            // This attribute is handled separately - it generates inventory::submit! for CallbackInfo
            // We don't include it in clean_attrs as it's not a runtime attribute
        } else {
            // Not doc, not status/tag/security/dependencies/response_model/response_class/callback - keep it (e.g. serde, schemars)
            clean_attrs.push(attr);
        }
    }

    // Default status codes
    let default_status: u16 = if response_class.as_deref() == Some("redirect") {
        307
    } else {
        match method {
            "post" => 201,
            "delete" => 204,
            _ => 200,
        }
    };
    let success_status = status_code.unwrap_or(default_status);

    let path_params: Vec<String> = path
        .split('/')
        .filter(|s| s.starts_with('{') && s.ends_with('}'))
        .map(|s| s[1..s.len() - 1].to_string())
        .collect();

    let axum_path = path.clone();
    let method_upper = method.to_uppercase();
    let method_ident = format_ident!("{}", method.to_lowercase());
    let wrapper_name = format_ident!("__{}_axum_handler", fn_name);
    let route_info_name = format_ident!("__{}_route_info", fn_name);
    let route_ref_name = format_ident!("__ULTRAAPI_ROUTE_{}", fn_name.to_string().to_uppercase());
    let hayai_route_ref_name =
        format_ident!("__HAYAI_ROUTE_{}", fn_name.to_string().to_uppercase());

    // Parse #[callback(...)] attributes from the function
    // These define callbacks for OpenAPI specification
    // Must be after route_info_name is defined so we can reference it
    let mut callback_submits: Vec<proc_macro2::TokenStream> = Vec::new();
    for attr in &input_fn.attrs {
        if attr.path().is_ident("callback") {
            if let syn::Meta::List(list) = &attr.meta {
                // Parse using parse_nested_meta for robust parsing
                let mut callback_name: Option<String> = None;
                let mut callback_expression: Option<String> = None;
                let mut callback_route_ident: Option<syn::Ident> = None;

                let parser = syn::meta::parser(|meta| {
                    if meta.path.is_ident("name") {
                        let value: LitStr = meta.value()?.parse()?;
                        callback_name = Some(value.value());
                    } else if meta.path.is_ident("expression") {
                        let value: LitStr = meta.value()?.parse()?;
                        callback_expression = Some(value.value());
                    } else if meta.path.is_ident("route") {
                        // Parse the route identifier (e.g., CALLBACK_ROUTE)
                        let ident: syn::Ident = meta.value()?.parse()?;
                        callback_route_ident = Some(ident);
                    }
                    Ok(())
                });

                if let Err(err) = parser.parse2(list.tokens.clone()) {
                    return err.to_compile_error().into();
                }

                if let (Some(name), Some(expr), Some(route_ident)) =
                    (callback_name, callback_expression, callback_route_ident)
                {
                    // Create the inventory::submit! token stream for this callback
                    let submit = quote! {
                        ultraapi::inventory::submit! {
                            ultraapi::CallbackInfo {
                                owner: &#route_info_name,
                                name: #name,
                                expression: #expr,
                                route: #route_ident,
                            }
                        }
                    };
                    callback_submits.push(submit);
                } else {
                    return syn::Error::new_spanned(
                        list,
                        "#[callback(...)] requires name, expression, and route parameters",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
    }

    let mut dep_extractions = Vec::new();
    let mut call_args = Vec::new();
    let mut has_body = false;
    let mut has_form_body = false;
    let mut has_multipart_body = false;
    let mut has_request_extractor = false;
    let mut has_generator_deps = false;
    let mut has_depends_params = false;
    let mut body_type: Option<&Type> = None;
    let mut path_param_types: Vec<(&syn::Ident, &Type)> = Vec::new();
    let mut query_type: Option<&Type> = None;
    let mut query_extraction = quote! {};
    let mut scalar_query_params: Vec<(&syn::Ident, &Type)> = Vec::new();
    let mut openapi_dynamic_params: Vec<proc_macro2::TokenStream> = Vec::new();

    for arg in &input_fn.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            let param_name = quote!(#pat).to_string();
            if is_dep_type(ty) {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::Dep<#inner> = ultraapi::Dep::from_app_state(&state)?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_state_type(ty) {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::State<#inner> = ultraapi::State::from_app_state(&state)?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_depends_type(ty) {
                // FastAPI-style Depends<T> - resolve dependency with nested support and cycle detection
                // Uses DependsResolver from AppState if available, otherwise falls back to direct lookup
                // Also handles generator (yield-based) dependencies
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            // Track that we have Depends parameters so we can create request-local cache.
                            has_depends_params = true;
                            // Keep shared cleanup scope behavior for yield-based deps.
                            has_generator_deps = true;

                            dep_extractions.push(quote! {
                                // FastAPI use_cache=true behavior: resolve once per request per type.
                                let #pat: ultraapi::Depends<#inner> = if let Some(cached) = depends_cache.get::<#inner>() {
                                    ultraapi::Depends(cached)
                                } else if let Some(resolver) = state.get_depends_resolver() {
                                    if resolver.is_generator::<#inner>() {
                                        match resolver.resolve_generator::<#inner>(&state, &dep_scope).await {
                                            Ok(dep) => {
                                                // Cast from Arc<dyn Any> back to Arc<#inner>
                                                let dep_any = dep.clone();
                                                let dep_typed: std::sync::Arc<#inner> = dep_any.downcast()
                                                    .map_err(|_| ultraapi::ApiError::internal(
                                                        format!("Type mismatch for generator: {}", std::any::type_name::<#inner>())
                                                    ))?;
                                                depends_cache.insert(dep_typed.clone());
                                                ultraapi::Depends(dep_typed)
                                            }
                                            Err(e) => return Err(ultraapi::ApiError::internal(e.to_string())),
                                        }
                                    } else {
                                        // Not a generator - use regular resolve with shared request-local cache.
                                        match resolver.resolve_with_cache::<#inner>(&state, &depends_cache).await {
                                            Ok(dep) => ultraapi::Depends(dep),
                                            Err(e) => return Err(ultraapi::ApiError::internal(e.to_string())),
                                        }
                                    }
                                } else {
                                    // Fallback: try direct AppState resolution for simple cases.
                                    let dep = state
                                        .get::<#inner>()
                                        .ok_or_else(|| ultraapi::ApiError::internal(
                                            format!("Dependency not registered: {}", std::any::type_name::<#inner>())
                                        ))?;
                                    depends_cache.insert(dep.clone());
                                    ultraapi::Depends(dep)
                                };
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_query_type(ty) {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            query_type = Some(inner);
                            query_extraction = quote! {
                                let #pat: ultraapi::axum::extract::Query<#inner> =
                                    ultraapi::axum::extract::Query::from_request_parts(&mut parts, &state).await
                                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid query parameters: {}", e)))?;
                                // Validate the query params using ValidatedWrapper (handles both api_model and non-api_model types)
                                if let Err(e) = ultraapi::ValidatedWrapper::validate(&#pat.0) {
                                    return Err(ultraapi::ApiError::validation_error(e));
                                }
                            };
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_header_type(ty) {
                // TypedHeader<T> and Option<TypedHeader<T>> extractor
                if let Some(inner) = typed_header_inner_type(ty) {
                    if option_inner_type(ty).is_some() {
                        dep_extractions.push(quote! {
                            let #pat: Option<ultraapi::axum_extra::extract::TypedHeader<#inner>> =
                                <ultraapi::axum_extra::extract::TypedHeader<#inner> as ultraapi::axum::extract::OptionalFromRequestParts<ultraapi::AppState>>::from_request_parts(&mut parts, &state).await
                                .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid header: {}", e)))?;
                        });
                    } else {
                        dep_extractions.push(quote! {
                            let #pat: ultraapi::axum_extra::extract::TypedHeader<#inner> =
                                ultraapi::axum_extra::extract::TypedHeader::from_request_parts(&mut parts, &state).await
                                .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid header: {}", e)))?;
                        });
                    }

                    let header_required = option_inner_type(ty).is_none();
                    openapi_dynamic_params.push(quote! {
                        params.push(ultraapi::openapi::DynParameter {
                            name: <#inner as ultraapi::axum_extra::headers::Header>::name().as_str().to_string(),
                            location: "header".to_string(),
                            required: #header_required,
                            schema_type: "string".to_string(),
                            description: None,
                            style: Some("simple".to_string()),
                            explode: Some(false),
                            example: None,
                            examples: None,
                            minimum: None,
                            maximum: None,
                            min_length: None,
                            max_length: None,
                            pattern: None,
                        });
                    });

                    call_args.push(quote!(#pat));
                }
            } else if is_header_map_type(ty) {
                // HeaderMap extractor (clone request headers)
                dep_extractions.push(quote! {
                    let #pat: ultraapi::axum::http::HeaderMap = parts.headers.clone();
                });
                call_args.push(quote!(#pat));
            } else if is_request_type(ty) {
                // Request<Body> extractor - pass through raw request
                has_request_extractor = true;
                dep_extractions.push(quote! {
                    let #pat: ultraapi::axum::http::Request<ultraapi::axum::body::Body> = req;
                });
                call_args.push(quote!(#pat));
            } else if is_cookie_type(ty) {
                // Cookie/CookieJar extractor
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        let cookie_type = &seg.ident;
                        dep_extractions.push(quote! {
                            let #pat: ultraapi::axum_extra::extract::#cookie_type =
                                ultraapi::axum_extra::extract::#cookie_type::from_request_parts(&mut parts, &state).await
                                .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid cookie: {}", e)))?;
                        });

                        let cookie_name =
                            pat_ident_name(pat).unwrap_or_else(|| "cookie".to_string());
                        let cookie_type_name = cookie_type.to_string();
                        let cookie_required = cookie_type_name != "CookieJar";
                        openapi_dynamic_params.push(quote! {
                            params.push(ultraapi::openapi::DynParameter {
                                name: #cookie_name.to_string(),
                                location: "cookie".to_string(),
                                required: #cookie_required,
                                schema_type: "string".to_string(),
                                description: None,
                                style: Some("form".to_string()),
                                explode: Some(true),
                                example: None,
                                examples: None,
                                minimum: None,
                                maximum: None,
                                min_length: None,
                                max_length: None,
                                pattern: None,
                            });
                        });

                        call_args.push(quote!(#pat));
                    }
                }
            } else if is_form_type(ty) {
                // Form<T> extractor for application/x-www-form-urlencoded
                has_body = true;
                has_form_body = true;
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            body_type = Some(inner);
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::axum::extract::Form<#inner> =
                                    ultraapi::axum::extract::Form::from_request(req, &state).await
                                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid form data: {}", e)))?;
                                if let Err(e) = ultraapi::ValidatedWrapper::validate(&#pat.0) {
                                    return Err(ultraapi::ApiError::validation_error(e));
                                }
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_multipart_type(ty) {
                // Multipart extractor for file uploads
                has_body = true;
                has_multipart_body = true;
                dep_extractions.push(quote! {
                    let #pat: ultraapi::axum::extract::Multipart =
                        ultraapi::axum::extract::Multipart::from_request(req, &state).await
                        .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid multipart: {}", e)))?;
                });
                call_args.push(quote!(#pat));
            } else if is_oauth2_password_bearer_type(ty) {
                // OAuth2PasswordBearer extractor
                dep_extractions.push(quote! {
                    let #pat: ultraapi::middleware::OAuth2PasswordBearer =
                        ultraapi::middleware::OAuth2PasswordBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
                });
                call_args.push(quote!(#pat));
            } else if is_optional_oauth2_password_bearer_type(ty) {
                // OptionalOAuth2PasswordBearer extractor (auto_error=false)
                dep_extractions.push(quote! {
                    let #pat: ultraapi::middleware::OptionalOAuth2PasswordBearer =
                        ultraapi::middleware::OptionalOAuth2PasswordBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
                });
                call_args.push(quote!(#pat));
            } else if is_oauth2_auth_code_bearer_type(ty) {
                // OAuth2AuthorizationCodeBearer extractor
                dep_extractions.push(quote! {
                    let #pat: ultraapi::middleware::OAuth2AuthorizationCodeBearer =
                        ultraapi::middleware::OAuth2AuthorizationCodeBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
                });
                call_args.push(quote!(#pat));
            } else if is_optional_oauth2_auth_code_bearer_type(ty) {
                // OptionalOAuth2AuthorizationCodeBearer extractor (auto_error=false)
                dep_extractions.push(quote! {
                    let #pat: ultraapi::middleware::OptionalOAuth2AuthorizationCodeBearer =
                        ultraapi::middleware::OptionalOAuth2AuthorizationCodeBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
                });
                call_args.push(quote!(#pat));
            } else if is_session_type(ty) {
                // Session extractor
                dep_extractions.push(quote! {
                    let #pat: ultraapi::session::Session =
                        ultraapi::session::Session::from_request_parts(&mut parts, &state).await
                        .map_err(|e| ultraapi::ApiError::internal(format!("Session extraction error: {:?}", e)))?;
                });
                call_args.push(quote!(#pat));
            } else if is_background_tasks_type(ty) {
                // BackgroundTasks extractor (injected by response_task_middleware)
                dep_extractions.push(quote! {
                    let #pat: ultraapi::response_tasks::BackgroundTasks =
                        ultraapi::response_tasks::BackgroundTasks::from_request_parts(&mut parts, &state).await
                        .map_err(|e| ultraapi::ApiError::internal(format!("BackgroundTasks extraction error: {:?}", e)))?;
                });
                call_args.push(quote!(#pat));
            } else if path_params.contains(&param_name) {
                if let syn::Pat::Ident(pi) = pat.as_ref() {
                    path_param_types.push((&pi.ident, ty));
                    call_args.push(quote!(#pat));
                }
            } else if is_scalar_query_type(ty) {
                if let syn::Pat::Ident(pi) = pat.as_ref() {
                    scalar_query_params.push((&pi.ident, ty));
                    call_args.push(quote!(#pat));
                } else {
                    return syn::Error::new_spanned(
                        pat,
                        "Scalar query parameters must use identifier patterns",
                    )
                    .to_compile_error()
                    .into();
                }
            } else if !is_header_type(ty)
                && !is_header_map_type(ty)
                && !is_request_type(ty)
                && !is_cookie_type(ty)
                && !is_form_type(ty)
                && !is_multipart_type(ty)
                && !is_oauth2_password_bearer_type(ty)
                && !is_optional_oauth2_password_bearer_type(ty)
                && !is_oauth2_auth_code_bearer_type(ty)
                && !is_optional_oauth2_auth_code_bearer_type(ty)
                && !is_session_type(ty)
                && !is_background_tasks_type(ty)
            {
                has_body = true;
                body_type = Some(ty);
                call_args.push(quote!(#pat));
            } else {
                call_args.push(quote!(#pat));
            }
        }
    }

    let mut route_dependency_extractions: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut route_dependency_type_names: Vec<String> = Vec::new();

    for dep_ty in &route_dependencies {
        let resolved_ty = resolve_dependency_type(dep_ty);
        route_dependency_type_names.push(get_type_name(&resolved_ty));

        if is_oauth2_password_bearer_type(dep_ty) || is_oauth2_password_bearer_type(&resolved_ty) {
            route_dependency_extractions.push(quote! {
                let _ultraapi_route_dependency: ultraapi::middleware::OAuth2PasswordBearer =
                    ultraapi::middleware::OAuth2PasswordBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
            });
            if !security_schemes.iter().any(|s| s == "oauth2Password") {
                security_schemes.push("oauth2Password".to_string());
            }
            continue;
        }

        if is_optional_oauth2_password_bearer_type(dep_ty)
            || is_optional_oauth2_password_bearer_type(&resolved_ty)
        {
            route_dependency_extractions.push(quote! {
                let _ultraapi_route_dependency: ultraapi::middleware::OptionalOAuth2PasswordBearer =
                    ultraapi::middleware::OptionalOAuth2PasswordBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
            });
            if !security_schemes.iter().any(|s| s == "oauth2Password") {
                security_schemes.push("oauth2Password".to_string());
            }
            continue;
        }

        if is_oauth2_auth_code_bearer_type(dep_ty) || is_oauth2_auth_code_bearer_type(&resolved_ty)
        {
            route_dependency_extractions.push(quote! {
                let _ultraapi_route_dependency: ultraapi::middleware::OAuth2AuthorizationCodeBearer =
                    ultraapi::middleware::OAuth2AuthorizationCodeBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
            });
            if !security_schemes.iter().any(|s| s == "oauth2AuthCode") {
                security_schemes.push("oauth2AuthCode".to_string());
            }
            continue;
        }

        if is_optional_oauth2_auth_code_bearer_type(dep_ty)
            || is_optional_oauth2_auth_code_bearer_type(&resolved_ty)
        {
            route_dependency_extractions.push(quote! {
                let _ultraapi_route_dependency: ultraapi::middleware::OptionalOAuth2AuthorizationCodeBearer =
                    ultraapi::middleware::OptionalOAuth2AuthorizationCodeBearer::from_request_parts(&mut parts, &state).await
                        .map_err(|e| e)?;
            });
            if !security_schemes.iter().any(|s| s == "oauth2AuthCode") {
                security_schemes.push("oauth2AuthCode".to_string());
            }
            continue;
        }

        route_dependency_extractions.push(quote! {
            let _ultraapi_route_dependency =
                ultraapi::resolve_route_dependency::<#resolved_ty>(&state, &dep_scope, &depends_cache).await?;
        });
    }

    if !route_dependency_extractions.is_empty() {
        has_depends_params = true;
        has_generator_deps = true;
    }

    route_dependency_type_names.sort();
    route_dependency_type_names.dedup();

    if has_request_extractor && (has_body || has_form_body || has_multipart_body) {
        return syn::Error::new_spanned(
            &input_fn.sig,
            "Request extractor cannot be combined with body/Form/Multipart extractors",
        )
        .to_compile_error()
        .into();
    }

    let return_type = match &input_fn.sig.output {
        syn::ReturnType::Type(_, ty) => Some(ty.as_ref()),
        _ => None,
    };

    // Detect Result<T, ApiError> return type
    let is_result_return = return_type
        .map(|t| get_result_ok_type(t).is_some())
        .unwrap_or(false);
    let effective_return_type = return_type
        .and_then(|t| get_result_ok_type(t))
        .or(return_type);

    let return_type_name = effective_return_type
        .map(get_type_name)
        .unwrap_or_else(|| "()".to_string());
    let body_type_name_for_field_set = body_type.map(get_type_name);
    let should_capture_request_field_set = has_response_model
        && exclude_unset
        && has_body
        && body_type_name_for_field_set
            .as_deref()
            .map(|name| name == return_type_name)
            .unwrap_or(false);

    // Detect Vec<T> return type for array schema (check effective type, i.e. inside Result if applicable)
    let is_vec_response = effective_return_type
        .map(|t| get_vec_inner_type_name(t).is_some())
        .unwrap_or(false);
    let vec_inner_type_name = effective_return_type
        .and_then(get_vec_inner_type_name)
        .unwrap_or_default();

    let path_extraction = if !path_param_types.is_empty() {
        let names: Vec<_> = path_param_types.iter().map(|(n, _)| *n).collect();
        let types: Vec<_> = path_param_types.iter().map(|(_, t)| *t).collect();
        if path_param_types.len() == 1 {
            let n = names[0];
            let t = types[0];
            quote! {
                let ultraapi::axum::extract::Path(#n): ultraapi::axum::extract::Path<#t> =
                    ultraapi::axum::extract::Path::from_request_parts(&mut parts, &state).await
                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid path param: {}", e)))?;
            }
        } else {
            quote! {
                let ultraapi::axum::extract::Path((#(#names),*)): ultraapi::axum::extract::Path<(#(#types),*)> =
                    ultraapi::axum::extract::Path::from_request_parts(&mut parts, &state).await
                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid path params: {}", e)))?;
            }
        }
    } else {
        quote! {}
    };

    let scalar_query_struct_name = format_ident!("__ultraapi_scalar_query_{}", fn_name);
    let scalar_query_struct_def = if scalar_query_params.is_empty() {
        quote! {}
    } else {
        let scalar_query_fields: Vec<_> = scalar_query_params
            .iter()
            .map(|(ident, ty)| quote! { #ident: #ty })
            .collect();

        quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[derive(ultraapi::serde::Deserialize, ultraapi::schemars::JsonSchema)]
            struct #scalar_query_struct_name {
                #(#scalar_query_fields,)*
            }
        }
    };

    let scalar_query_extraction = if scalar_query_params.is_empty() {
        quote! {}
    } else {
        let scalar_query_bindings: Vec<_> = scalar_query_params
            .iter()
            .map(|(ident, _)| quote! { let #ident = __ultraapi_scalar_query.#ident; })
            .collect();

        quote! {
            let ultraapi::axum::extract::Query(__ultraapi_scalar_query): ultraapi::axum::extract::Query<#scalar_query_struct_name> =
                ultraapi::axum::extract::Query::from_request_parts(&mut parts, &state).await
                .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid query parameters: {}", e)))?;
            #(#scalar_query_bindings)*
        }
    };

    let body_extraction = if has_form_body || has_multipart_body {
        // Form and Multipart bodies are handled in the dep_extractions loop
        quote! {}
    } else if has_body {
        let bty = body_type.unwrap();
        let bpat = input_fn
            .sig
            .inputs
            .iter()
            .find_map(|arg| {
                if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
                    if !is_dep_type(ty)
                        && !is_scalar_query_type(ty)
                        && !is_query_type(ty)
                        && !is_header_type(ty)
                        && !is_header_map_type(ty)
                        && !is_request_type(ty)
                        && !is_cookie_type(ty)
                        && !is_form_type(ty)
                        && !is_multipart_type(ty)
                        && !is_oauth2_password_bearer_type(ty)
                        && !is_optional_oauth2_password_bearer_type(ty)
                        && !is_oauth2_auth_code_bearer_type(ty)
                        && !is_optional_oauth2_auth_code_bearer_type(ty)
                        && !is_session_type(ty)
                    {
                        let n = quote!(#pat).to_string();
                        if !path_params.contains(&n) {
                            return Some(pat.clone());
                        }
                    }
                }
                None
            })
            .unwrap();
        if should_capture_request_field_set {
            quote! {
                let ultraapi::axum::Json(__ultraapi_raw_body): ultraapi::axum::Json<ultraapi::serde_json::Value> =
                    ultraapi::axum::Json::from_request(req, &state).await
                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid body: {}", e)))?;
                __ultraapi_response_field_set = Some(ultraapi::collect_present_field_paths(&__ultraapi_raw_body));
                let #bpat: #bty = ultraapi::serde_json::from_value(__ultraapi_raw_body)
                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid body: {}", e)))?;
                #bpat.validate().map_err(|e| ultraapi::ApiError::validation_error(e))?;
            }
        } else {
            quote! {
                let ultraapi::axum::Json(#bpat): ultraapi::axum::Json<#bty> =
                    ultraapi::axum::Json::from_request(req, &state).await
                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid body: {}", e)))?;
                #bpat.validate().map_err(|e| ultraapi::ApiError::validation_error(e))?;
            }
        }
    } else if has_request_extractor {
        quote! {}
    } else {
        quote! { let _ = req; }
    };

    let response_field_set_init = if should_capture_request_field_set {
        quote! {
            let mut __ultraapi_response_field_set: Option<std::collections::HashSet<String>> = None;
        }
    } else {
        quote! {}
    };

    // Generate response based on status code
    let status_lit = proc_macro2::Literal::u16_unsuffixed(success_status);

    // Generate the response shaping code if response_model attribute was used
    // This ensures shaping runs even when by_alias=false is explicitly set
    let response_shaping_expr = if has_response_model {
        let include_expr = if let Some(ref fields) = include_fields {
            let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
            quote! { Some(&[#(#field_refs),*] as &'static [&'static str]) }
        } else {
            quote! { None }
        };

        let exclude_expr = if let Some(ref fields) = exclude_fields {
            let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
            quote! { Some(&[#(#field_refs),*] as &'static [&'static str]) }
        } else {
            quote! { None }
        };

        let by_alias_expr = if by_alias {
            quote! { true }
        } else {
            quote! { false }
        };

        // Get the return type name for alias lookup
        let type_name_expr = quote! { Some(#return_type_name) };
        let field_set_expr = if should_capture_request_field_set {
            quote! { __ultraapi_response_field_set.as_ref() }
        } else {
            quote! { None }
        };

        quote! {
            let shaping_options = ultraapi::ResponseModelOptions {
                include: #include_expr,
                exclude: #exclude_expr,
                by_alias: #by_alias_expr,
                exclude_none: #exclude_none,
                exclude_unset: #exclude_unset,
                exclude_defaults: #exclude_defaults,
                content_type: None, // content_type only affects OpenAPI, not runtime
            };
            let value = shaping_options.apply_with_aliases_and_field_set(
                value,
                #type_name_expr,
                #by_alias,
                #field_set_expr,
            );
        }
    } else {
        quote! { /* No response model shaping */ }
    };

    // Generate response based on response_class
    let response_expr = match response_class.as_deref() {
        // HTML response
        Some("html") => {
            if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "text/html")],
                        body,
                    ).into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "text/html")],
                        body,
                    ).into_response())
                }
            }
        }
        // Plain text response
        Some("text") => {
            if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "text/plain")],
                        body,
                    ).into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "text/plain")],
                        body,
                    ).into_response())
                }
            }
        }
        // Binary/Stream response
        Some("binary") | Some("stream") => {
            if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "application/octet-stream")],
                        body,
                    ).into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "application/octet-stream")],
                        body,
                    ).into_response())
                }
            }
        }
        // XML response
        Some("xml") => {
            if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "application/xml")],
                        body,
                    ).into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    let body = result;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        [("content-type", "application/xml")],
                        body,
                    ).into_response())
                }
            }
        }
        // File response - returns FileResponse which handles content-type and content-disposition
        Some("file") => {
            if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        result,
                    ).into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        result,
                    ).into_response())
                }
            }
        }
        // Redirect response - returns RedirectResponse which handles Location header
        Some("redirect") => {
            if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    Ok(result.into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    Ok(result.into_response())
                }
            }
        }
        // Cookie response - returns CookieResponse which handles Set-Cookie headers
        // The return type should implement IntoResponse directly
        Some("cookie") => {
            if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    Ok(result.into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    Ok(result.into_response())
                }
            }
        }
        // JSON response (default)
        _ => {
            if success_status == 204 {
                if is_result_return {
                    quote! {
                        let _ = #fn_name(#(#call_args),*).await?;
                        Ok((ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),).into_response())
                    }
                } else {
                    quote! {
                        let _ = #fn_name(#(#call_args),*).await;
                        Ok((ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),).into_response())
                    }
                }
            } else if is_result_return {
                quote! {
                    let result = #fn_name(#(#call_args),*).await?;
                    let value = ultraapi::serde_json::to_value(&result)
                        .map_err(|e| ultraapi::ApiError::internal(format!("Response serialization failed: {}", e)))?;
                    #response_shaping_expr
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        ultraapi::axum::Json(value),
                    ).into_response())
                }
            } else {
                quote! {
                    let result = #fn_name(#(#call_args),*).await;
                    let value = ultraapi::serde_json::to_value(&result)
                        .map_err(|e| ultraapi::ApiError::internal(format!("Response serialization failed: {}", e)))?;
                    #response_shaping_expr
                    Ok((
                        ultraapi::axum::http::StatusCode::from_u16(#status_lit).unwrap(),
                        ultraapi::axum::Json(value),
                    ).into_response())
                }
            }
        }
    };

    let path_param_schemas: Vec<_> = path_params
        .iter()
        .map(|p| {
            // Find the type of this path param
            let openapi_type = path_param_types
                .iter()
                .find(|(name, _)| name.to_string() == *p)
                .map(|(_, ty)| {
                    let type_name = get_type_name(ty);
                    match type_name.as_str() {
                        "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64"
                        | "u128" => "integer",
                        "f32" | "f64" => "number",
                        "String" => "string",
                        "bool" => "boolean",
                        _ => "string",
                    }
                })
                .unwrap_or("string");
            quote! {
                ultraapi::openapi::Parameter {
                    name: #p,
                    location: "path",
                    required: true,
                    schema: ultraapi::openapi::SchemaObject::new_type(#openapi_type),
                    description: None,
                    style: Some("simple"),
                    explode: Some(false),
                    example: None,
                    examples: None,
                }
            }
        })
        .collect();

    let body_type_name = if has_multipart_body {
        "Multipart".to_string()
    } else {
        body_type.map(get_type_name).unwrap_or_default()
    };
    let request_body_content_type = if has_form_body {
        "application/x-www-form-urlencoded"
    } else if has_multipart_body {
        "multipart/form-data"
    } else {
        "application/json"
    };
    let fn_name_str = fn_name.to_string();

    // Generate ResponseModelOptions expression
    let include_expr = if let Some(ref fields) = include_fields {
        let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        quote! { Some(&[#(#field_refs),*] as &'static [&'static str]) }
    } else {
        quote! { None }
    };

    let exclude_expr = if let Some(ref fields) = exclude_fields {
        let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        quote! { Some(&[#(#field_refs),*] as &'static [&'static str]) }
    } else {
        quote! { None }
    };

    let by_alias_expr = if by_alias {
        quote! { true }
    } else {
        quote! { false }
    };

    let response_model_content_type_expr = match &response_model_content_type {
        Some(ct) => quote! { Some(#ct) },
        None => quote! { None },
    };

    let response_model_options_expr = quote! {
        ultraapi::ResponseModelOptions {
            include: #include_expr,
            exclude: #exclude_expr,
            by_alias: #by_alias_expr,
            exclude_none: #exclude_none,
            exclude_unset: #exclude_unset,
            exclude_defaults: #exclude_defaults,
            content_type: #response_model_content_type_expr,
        }
    };

    // Generate response_class based on attribute
    let response_class_expr = match response_class.as_deref() {
        Some("html") => quote! { ultraapi::ResponseClass::Html },
        Some("text") => quote! { ultraapi::ResponseClass::Text },
        Some("binary") => quote! { ultraapi::ResponseClass::Binary },
        Some("stream") => quote! { ultraapi::ResponseClass::Stream },
        Some("xml") => quote! { ultraapi::ResponseClass::Xml },
        Some("file") => quote! { ultraapi::ResponseClass::File },
        Some("redirect") => quote! { ultraapi::ResponseClass::Redirect },
        Some("cookie") => quote! { ultraapi::ResponseClass::Json }, // Cookies still return JSON body
        Some("json") | None => quote! { ultraapi::ResponseClass::Json },
        other => quote! {
            compile_error!(concat!("Invalid response_class: ", #other, ". Valid values are: json, html, text, binary, stream, xml, file, redirect, cookie"))
        },
    };

    let has_query_params = query_type.is_some() || !scalar_query_params.is_empty();

    let query_param_generation_expr = if let Some(qt) = query_type {
        if scalar_query_params.is_empty() {
            quote! {
                let root = ultraapi::schemars::schema_for!(#qt);
                params.extend(ultraapi::openapi::query_params_from_schema(&root));
            }
        } else {
            quote! {
                let root = ultraapi::schemars::schema_for!(#qt);
                params.extend(ultraapi::openapi::query_params_from_schema(&root));
                let scalar_root = ultraapi::schemars::schema_for!(#scalar_query_struct_name);
                params.extend(ultraapi::openapi::query_params_from_schema(&scalar_root));
            }
        }
    } else if scalar_query_params.is_empty() {
        quote! {}
    } else {
        quote! {
            let root = ultraapi::schemars::schema_for!(#scalar_query_struct_name);
            params.extend(ultraapi::openapi::query_params_from_schema(&root));
        }
    };

    let query_params_fn_expr = if has_query_params || !openapi_dynamic_params.is_empty() {
        quote! { Some(|| {
            let mut params = Vec::new();
            #query_param_generation_expr
            #(#openapi_dynamic_params)*
            params
        }) }
    } else {
        quote! { None }
    };

    // Generate TokenStream for optional OpenAPI fields (summary, external_docs_url, external_docs_description)
    let summary_expr = match &summary {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };
    let external_docs_url_expr = match &external_docs_url {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };
    let external_docs_description_expr = match &external_docs_description {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };

    // Generate per-request scope/cache setup for Depends resolution.
    let scope_creation = if has_depends_params {
        quote! {
            let dep_scope = std::sync::Arc::new(ultraapi::DependencyScope::new());
            let depends_cache = ultraapi::RequestDependsCache::new();
        }
    } else {
        quote! {}
    };

    // Generate cleanup wrapping around response
    // Function cleanup runs BEFORE response is returned (both success and error)
    // Request cleanup runs AFTER response is sent (spawned task)
    let cleanup_wrapper = if has_generator_deps {
        quote! {
            // Clone scope for request cleanup (runs after response is sent)
            let dep_scope_for_request = dep_scope.clone();

            // Run function-scope cleanup - capture result to return after cleanup
            let result: Result<ultraapi::axum::response::Response, ultraapi::ApiError> = { #response_expr };

            // Run function cleanup BEFORE returning (both success and error cases)
            dep_scope.run_function_cleanup().await;

            // Spawn request-scope cleanup to run AFTER response is handled
            // This runs regardless of success or error
            tokio::spawn(async move {
                // Small delay to ensure response is fully sent
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                dep_scope_for_request.run_request_cleanup().await;
            });

            result
        }
    } else {
        quote! {
            #response_expr
        }
    };

    let output = quote! {
        #(#clean_attrs)*
        #fn_vis #fn_sig #fn_block

        #scalar_query_struct_def

        #[doc(hidden)]
        async fn #wrapper_name(
            ultraapi::axum::extract::State(state): ultraapi::axum::extract::State<ultraapi::AppState>,
            mut parts: ultraapi::axum::http::request::Parts,
            req: ultraapi::axum::http::Request<ultraapi::axum::body::Body>,
        ) -> Result<ultraapi::axum::response::Response, ultraapi::ApiError> {
            use ultraapi::axum::extract::FromRequest;
            use ultraapi::axum::extract::FromRequestParts;
            use ultraapi::axum::response::IntoResponse;
            use ultraapi::Validate;

            #scope_creation

            #path_extraction
            #query_extraction
            #scalar_query_extraction
            #(#route_dependency_extractions)*
            #(#dep_extractions)*
            #response_field_set_init
            #body_extraction

            #cleanup_wrapper
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #route_info_name: ultraapi::RouteInfo = ultraapi::RouteInfo {
            path: #path,
            axum_path: #axum_path,
            method: #method_upper,
            handler_name: #fn_name_str,
            response_type_name: #return_type_name,
            is_result_return: #is_result_return,
            is_vec_response: #is_vec_response,
            is_sse: false,
            is_websocket: false,
            vec_inner_type_name: #vec_inner_type_name,
            parameters: &[#(#path_param_schemas),*],
            has_body: #has_body,
            body_type_name: #body_type_name,
            request_body_content_type: #request_body_content_type,
            success_status: #status_lit,
            description: #description,
            tags: &[#(#tags),*],
            security: &[#(#security_schemes),*],
            dependencies: &[#(#route_dependency_type_names),*],
            query_params_fn: #query_params_fn_expr,
            has_query_params: #has_query_params,
            response_model_options: #response_model_options_expr,
            response_class: #response_class_expr,
            summary: #summary_expr,
            deprecated: #deprecated,
            external_docs_url: #external_docs_url_expr,
            external_docs_description: #external_docs_description_expr,
            register_fn: |app: ultraapi::axum::Router<ultraapi::AppState>| {
                app.route(#axum_path, ultraapi::axum::routing::#method_ident(#wrapper_name))
            },
            method_router_fn: || {
                ultraapi::axum::routing::#method_ident(#wrapper_name)
            },
        };

        #[doc(hidden)]
        pub static #route_ref_name: &ultraapi::RouteInfo = &#route_info_name;

        // Backward compatibility alias (deprecated)
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub static #hayai_route_ref_name: &ultraapi::RouteInfo = &#route_info_name;

        ultraapi::inventory::submit! { &#route_info_name }

        // Generate inventory::submit! for callbacks defined via #[callback(...)] attribute
        #(#callback_submits)*
    };

    output.into()
}

#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("get", attr, item)
}

#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("post", attr, item)
}

#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("put", attr, item)
}

#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("delete", attr, item)
}

#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("patch", attr, item)
}

#[proc_macro_attribute]
pub fn head(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("head", attr, item)
}

#[proc_macro_attribute]
pub fn options(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("options", attr, item)
}

#[proc_macro_attribute]
pub fn trace(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_impl("trace", attr, item)
}

/// SSE endpoint macro - creates Server-Sent Events endpoints
///
/// # Example
/// ```text
/// use ultraapi::prelude::*;
/// use ultraapi::axum::response::sse::Event;
///
/// #[sse("/stream")]
/// async fn stream_events() -> impl Stream<Item = Result<Event, Infallible>> {
///     // stream events ...
/// }
/// ```
#[proc_macro_attribute]
pub fn sse(attr: TokenStream, item: TokenStream) -> TokenStream {
    sse_macro_impl(attr, item)
}

/// WebSocket endpoint macro - creates WebSocket upgrade handlers
///
/// Note: WebSocket upgrade routes are intentionally excluded from OpenAPI paths
/// (aligned with FastAPI behavior), because OpenAPI does not model WS upgrades
/// as regular HTTP operations.
///
/// # Example
/// ```text
/// use ultraapi::prelude::*;
/// use ultraapi::axum::extract::ws::{Message, WebSocket};
///
/// #[ws("/ws")]
/// async fn ws_handler(
///     ws: ultraapi::axum::extract::ws::WebSocketUpgrade,
///     State(state): State<AppState>,
/// ) -> ultraapi::axum::response::Response {
///     ws.on_upgrade(move |socket: WebSocket| handle_socket(socket, state))
/// }
///
/// async fn handle_socket(mut socket: WebSocket, _state: AppState) {
///     while let Some(msg) = socket.recv().await {
///         if let Ok(Message::Text(text)) = msg {
///             // Handle message
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn ws(attr: TokenStream, item: TokenStream) -> TokenStream {
    ws_macro_impl(attr, item)
}

fn ws_macro_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr).value();
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    // Parse custom attributes
    let mut tags: Vec<String> = Vec::new();
    let mut security_schemes: Vec<String> = Vec::new();
    let description = extract_doc_comment(&input_fn.attrs);

    let mut clean_attrs: Vec<&syn::Attribute> = Vec::new();
    for attr in &input_fn.attrs {
        if attr.path().is_ident("tag") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    tags.push(lit.value());
                }
            }
        } else if attr.path().is_ident("security") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    security_schemes.push(lit.value());
                }
            }
        } else {
            clean_attrs.push(attr);
        }
    }

    let axum_path = path.clone();
    let wrapper_name = format_ident!("__ws_axum_handler_{}", fn_name);
    let route_info_name = format_ident!("__ws_route_info_{}", fn_name);
    let route_ref_name = format_ident!("__ULTRAAPI_WS_{}", fn_name.to_string().to_uppercase());

    // Extract dependencies from function arguments
    let mut dep_extractions = Vec::new();
    let mut call_args = Vec::new();

    for arg in &input_fn.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if is_dep_type(ty) {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::Dep<#inner> = ultraapi::Dep::from_app_state(&state)?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_state_type(ty) {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::State<#inner> = ultraapi::State::from_app_state(&state)?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_depends_type(ty) {
                // FastAPI-style Depends<T> for WebSocket
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::Depends<#inner> = if let Some(cached) = depends_cache.get::<#inner>() {
                                    ultraapi::Depends(cached)
                                } else if let Some(resolver) = state.get_depends_resolver() {
                                    match resolver.resolve_with_cache::<#inner>(&state, &depends_cache).await {
                                        Ok(dep) => ultraapi::Depends(dep),
                                        Err(e) => return Err(ultraapi::ApiError::internal(e.to_string()).into_response()),
                                    }
                                } else {
                                    let dep = state
                                        .get::<#inner>()
                                        .ok_or_else(|| ultraapi::ApiError::internal(
                                            format!("Dependency not registered: {}", std::any::type_name::<#inner>())
                                        ).into_response())?;
                                    depends_cache.insert(dep.clone());
                                    ultraapi::Depends(dep)
                                };
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else {
                // Other arguments (like WebSocketUpgrade) are passed as-is
                call_args.push(quote!(#pat));
            }
        }
    }

    let fn_name_str = fn_name.to_string();

    let output = quote! {
        #(#clean_attrs)*
        #fn_vis #fn_sig #fn_block

        #[doc(hidden)]
        async fn #wrapper_name(
            ws: ultraapi::axum::extract::ws::WebSocketUpgrade,
            ultraapi::axum::extract::State(state): ultraapi::axum::extract::State<ultraapi::AppState>,
        ) -> ultraapi::axum::response::Response {
            use ultraapi::axum::response::IntoResponse;

            let depends_cache = ultraapi::RequestDependsCache::new();
            #(#dep_extractions)*

            // Call the user's handler, passing the WebSocketUpgrade and any extracted deps
            #fn_name(#(#call_args),*).await.into_response()
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #route_info_name: ultraapi::RouteInfo = ultraapi::RouteInfo {
            path: #path,
            axum_path: #axum_path,
            method: "GET",
            handler_name: #fn_name_str,
            response_type_name: "WebSocket",
            is_result_return: false,
            is_vec_response: false,
            is_sse: false,
            is_websocket: true,
            vec_inner_type_name: "",
            parameters: &[],
            has_body: false,
            body_type_name: "",
            request_body_content_type: "",
            success_status: 101,
            description: #description,
            tags: &[#(#tags),*],
            security: &[#(#security_schemes),*],
            dependencies: &[],
            query_params_fn: None,
            has_query_params: false,
            response_model_options: ultraapi::ResponseModelOptions {
                include: None,
                exclude: None,
                by_alias: false,
                exclude_none: false,
                exclude_unset: false,
                exclude_defaults: false,
                content_type: None,
            },
            response_class: ultraapi::ResponseClass::Binary, // WebSocket upgrades don't really have a content-type
            summary: None,
            deprecated: false,
            external_docs_url: None,
            external_docs_description: None,
            register_fn: |app: ultraapi::axum::Router<ultraapi::AppState>| {
                app.route(#axum_path, ultraapi::axum::routing::get(#wrapper_name))
            },
            method_router_fn: || {
                ultraapi::axum::routing::get(#wrapper_name)
            },
        };

        #[doc(hidden)]
        pub static #route_ref_name: &ultraapi::RouteInfo = &#route_info_name;

        ultraapi::inventory::submit! { &#route_info_name }
    };

    output.into()
}

fn sse_macro_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr).value();
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    // Parse custom attributes: #[status(N)], #[tag("x")], #[security("x")], doc comments
    let mut status_code: Option<u16> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut security_schemes: Vec<String> = Vec::new();
    let description = extract_doc_comment(&input_fn.attrs);

    let mut clean_attrs: Vec<&syn::Attribute> = Vec::new();
    for attr in &input_fn.attrs {
        if attr.path().is_ident("status") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitInt>(tokens) {
                    status_code = Some(lit.base10_parse().unwrap());
                }
            }
        } else if attr.path().is_ident("tag") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    tags.push(lit.value());
                }
            }
        } else if attr.path().is_ident("security") {
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                if let Ok(lit) = syn::parse2::<LitStr>(tokens) {
                    security_schemes.push(lit.value());
                }
            }
        } else {
            clean_attrs.push(attr);
        }
    }

    // SSE always returns 200
    let success_status = status_code.unwrap_or(200);

    let path_params: Vec<String> = path
        .split('/')
        .filter(|s| s.starts_with('{') && s.ends_with('}'))
        .map(|s| s[1..s.len() - 1].to_string())
        .collect();

    let axum_path = path.clone();
    let wrapper_name = format_ident!("__sse_axum_handler_{}", fn_name);
    let route_info_name = format_ident!("__sse_route_info_{}", fn_name);
    let route_ref_name = format_ident!("__ULTRAAPI_SSE_{}", fn_name.to_string().to_uppercase());
    let hayai_route_ref_name = format_ident!("__HAYAI_SSE_{}", fn_name.to_string().to_uppercase());

    // Extract dependencies from function arguments
    let mut dep_extractions = Vec::new();
    let mut call_args = Vec::new();
    let mut path_param_types: Vec<(&syn::Ident, &Type)> = Vec::new();

    for arg in &input_fn.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            let param_name = quote!(#pat).to_string();
            if is_dep_type(ty) {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::Dep<#inner> = ultraapi::Dep::from_app_state(&state)?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_state_type(ty) {
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::State<#inner> = ultraapi::State::from_app_state(&state)?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_depends_type(ty) {
                // FastAPI-style Depends<T> for SSE
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::Depends<#inner> = if let Some(cached) = depends_cache.get::<#inner>() {
                                    ultraapi::Depends(cached)
                                } else if let Some(resolver) = state.get_depends_resolver() {
                                    match resolver.resolve_with_cache::<#inner>(&state, &depends_cache).await {
                                        Ok(dep) => ultraapi::Depends(dep),
                                        Err(e) => return Err(ultraapi::ApiError::internal(e.to_string())),
                                    }
                                } else {
                                    let dep = state
                                        .get::<#inner>()
                                        .ok_or_else(|| ultraapi::ApiError::internal(
                                            format!("Dependency not registered: {}", std::any::type_name::<#inner>())
                                        ))?;
                                    depends_cache.insert(dep.clone());
                                    ultraapi::Depends(dep)
                                };
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if path_params.contains(&param_name) {
                if let syn::Pat::Ident(pi) = pat.as_ref() {
                    path_param_types.push((&pi.ident, ty));
                    call_args.push(quote!(#pat));
                }
            } else {
                call_args.push(quote!(#pat));
            }
        }
    }

    let path_extraction = if !path_param_types.is_empty() {
        let names: Vec<_> = path_param_types.iter().map(|(n, _)| *n).collect();
        let types: Vec<_> = path_param_types.iter().map(|(_, t)| *t).collect();
        if path_param_types.len() == 1 {
            let n = names[0];
            let t = types[0];
            quote! {
                let ultraapi::axum::extract::Path(#n): ultraapi::axum::extract::Path<#t> =
                    ultraapi::axum::extract::Path::from_request_parts(&mut parts, &state).await
                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid path param: {}", e)))?;
            }
        } else {
            quote! {
                let ultraapi::axum::extract::Path((#(#names),*)): ultraapi::axum::extract::Path<(#(#types),*)> =
                    ultraapi::axum::extract::Path::from_request_parts(&mut parts, &state).await
                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid path params: {}", e)))?;
            }
        }
    } else {
        quote! {}
    };

    // Path param schemas
    let path_param_schemas: Vec<_> = path_params
        .iter()
        .map(|p| {
            let openapi_type = path_param_types
                .iter()
                .find(|(name, _)| name.to_string() == *p)
                .map(|(_, ty)| {
                    let type_name = get_type_name(ty);
                    match type_name.as_str() {
                        "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64"
                        | "u128" => "integer",
                        "f32" | "f64" => "number",
                        "String" => "string",
                        "bool" => "boolean",
                        _ => "string",
                    }
                })
                .unwrap_or("string");
            quote! {
                ultraapi::openapi::Parameter {
                    name: #p,
                    location: "path",
                    required: true,
                    schema: ultraapi::openapi::SchemaObject::new_type(#openapi_type),
                    description: None,
                    style: Some("simple"),
                    explode: Some(false),
                    example: None,
                    examples: None,
                }
            }
        })
        .collect();

    let return_type_name = "Sse".to_string();
    let fn_name_str = fn_name.to_string();
    let status_lit = proc_macro2::Literal::u16_unsuffixed(success_status);

    let output = quote! {
        #(#clean_attrs)*
        #fn_vis #fn_sig #fn_block

        #[doc(hidden)]
        async fn #wrapper_name(
            ultraapi::axum::extract::State(state): ultraapi::axum::extract::State<ultraapi::AppState>,
            mut parts: ultraapi::axum::http::request::Parts,
            _req: ultraapi::axum::http::Request<ultraapi::axum::body::Body>,
        ) -> Result<ultraapi::axum::response::Response, ultraapi::ApiError> {
            use ultraapi::axum::extract::FromRequestParts;
            use ultraapi::axum::response::IntoResponse;
            use ultraapi::axum::response::sse::Sse;
            use std::convert::Infallible;

            let depends_cache = ultraapi::RequestDependsCache::new();
            #path_extraction
            #(#dep_extractions)*

            let stream = #fn_name(#(#call_args),*).await;
            let sse = Sse::new(stream);
            Ok(sse.into_response())
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #route_info_name: ultraapi::RouteInfo = ultraapi::RouteInfo {
            path: #path,
            axum_path: #axum_path,
            method: "GET",
            handler_name: #fn_name_str,
            response_type_name: #return_type_name,
            is_result_return: false,
            is_vec_response: false,
            is_sse: true,
            is_websocket: false,
            vec_inner_type_name: "",
            parameters: &[#(#path_param_schemas),*],
            has_body: false,
            body_type_name: "",
            request_body_content_type: "",
            success_status: #status_lit,
            description: #description,
            tags: &[#(#tags),*],
            security: &[#(#security_schemes),*],
            dependencies: &[],
            query_params_fn: None,
            has_query_params: false,
            response_model_options: ultraapi::ResponseModelOptions {
                include: None,
                exclude: None,
                by_alias: false,
                exclude_none: false,
                exclude_unset: false,
                exclude_defaults: false,
                content_type: None,
            },
            response_class: ultraapi::ResponseClass::Sse,
            summary: None,
            deprecated: false,
            external_docs_url: None,
            external_docs_description: None,
            register_fn: |app: ultraapi::axum::Router<ultraapi::AppState>| {
                app.route(#axum_path, ultraapi::axum::routing::get(#wrapper_name))
            },
            method_router_fn: || {
                ultraapi::axum::routing::get(#wrapper_name)
            },
        };

        #[doc(hidden)]
        pub static #route_ref_name: &ultraapi::RouteInfo = &#route_info_name;

        #[doc(hidden)]
        pub static #hayai_route_ref_name: &ultraapi::RouteInfo = &#route_info_name;

        ultraapi::inventory::submit! { &#route_info_name }
    };

    output.into()
}

#[proc_macro_attribute]
pub fn api_model(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse optional model-level custom validator: #[api_model(validate(custom = "my_fn"))]
    let mut custom_validation_fn: Option<Path> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("validate") {
            meta.parse_nested_meta(|nested| {
                if nested.path.is_ident("custom") {
                    let value = nested.value()?;
                    let lit: LitStr = value.parse()?;
                    let parsed_path = lit.parse::<Path>().map_err(|_| {
                        nested.error("custom validator must be a valid path string")
                    })?;
                    custom_validation_fn = Some(parsed_path);
                }
                Ok(())
            })?;
        }
        Ok(())
    });

    if let Err(err) = parser.parse(attr) {
        return err.to_compile_error().into();
    }

    let item_clone = item.clone();
    if let Ok(input) = syn::parse::<ItemStruct>(item) {
        api_model_struct(input, custom_validation_fn)
    } else if let Ok(input) = syn::parse::<ItemEnum>(item_clone) {
        api_model_enum(input)
    } else {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "api_model only supports structs and enums",
        )
        .to_compile_error()
        .into()
    }
}

fn api_model_enum(input: ItemEnum) -> TokenStream {
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let variants = &input.variants;
    let description = extract_doc_comment(attrs);

    let variant_names: Vec<String> = variants.iter().map(|v| v.ident.to_string()).collect();

    let name_str = name.to_string();

    let desc_expr = if description.is_empty() {
        quote! { None }
    } else {
        quote! { Some(#description.to_string()) }
    };

    let output = quote! {
        #(#attrs)*
        #[derive(ultraapi::serde::Serialize, ultraapi::serde::Deserialize, ultraapi::schemars::JsonSchema)]
        #[serde(crate = "ultraapi::serde")]
        #[schemars(crate = "ultraapi::schemars")]
        #vis enum #name {
            #variants
        }

        impl ultraapi::Validate for #name {
            fn validate(&self) -> Result<(), Vec<String>> { Ok(()) }
        }

        impl ultraapi::HasValidate for #name {}

        // Register validator in inventory for ValidatedWrapper to find at runtime
        // Use the simple struct name (without module path) for matching
        ultraapi::inventory::submit! {
            ultraapi::ValidatorInfo {
                type_name: stringify!(#name),
                validate_fn: |any: &dyn std::any::Any| {
                    if let Some(val) = any.downcast_ref::<#name>() {
                        <#name as ultraapi::Validate>::validate(val)
                    } else {
                        Err(vec!["Internal validation error: type mismatch".to_string()])
                    }
                },
            }
        }

        ultraapi::inventory::submit! {
            ultraapi::SchemaInfo {
                name: #name_str,
                schema_fn: || {
                    static CACHE: std::sync::OnceLock<ultraapi::openapi::Schema> = std::sync::OnceLock::new();
                    CACHE.get_or_init(|| {
                        ultraapi::openapi::Schema {
                            type_name: "string".to_string(),
                            properties: std::collections::HashMap::new(),
                            required: vec![],
                            description: #desc_expr,
                            enum_values: Some(vec![#(#variant_names.to_string()),*]),
                            example: None,
                            one_of: None,
                            discriminator: None,
                        }
                    }).clone()
                },
                nested_fn: || {
                    static CACHE: std::sync::OnceLock<std::collections::HashMap<String, ultraapi::openapi::Schema>> = std::sync::OnceLock::new();
                    CACHE.get_or_init(|| std::collections::HashMap::new()).clone()
                },
            }
        }
    };

    output.into()
}

fn api_model_struct(input: ItemStruct, custom_validation_fn: Option<Path>) -> TokenStream {
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;
    let struct_description = extract_doc_comment(attrs);

    let fields = match &input.fields {
        syn::Fields::Named(fields) => &fields.named,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "api_model only supports structs with named fields",
            )
            .to_compile_error()
            .into()
        }
    };

    let mut validation_checks = Vec::new();
    let mut schema_patches = Vec::new();
    let mut clean_fields = Vec::new();
    let mut serde_attrs_for_fields = Vec::new();
    // Collect field name -> alias mappings for response_model shaping
    let mut field_aliases: Vec<(String, String)> = Vec::new();
    // Collect field name -> default value expressions for exclude_defaults shaping
    let mut field_defaults: Vec<(String, TokenStream2)> = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let mut schema_field_name_str = field_name.to_string();
        let mut skip_field = false;
        let mut skip_serializing = false;
        let mut skip_deserializing = false;
        let mut read_only = false;
        let mut write_only = false;
        let mut field_deprecated = false;
        let mut has_custom_rename = false; // from #[alias(...)]
        let mut field_default_expr: Option<TokenStream2> = None;

        // Collect passthrough serde items (not managed by us: default, flatten, with, etc.)
        let mut passthrough_serde_items: Vec<proc_macro2::TokenStream> = Vec::new();
        // Track and merge existing #[serde(...)] managed items plus passthrough items.

        // Parse custom attributes and serde attributes together.
        // We strip all #[serde(...)] from the clean field and rebuild a merged one.
        for attr in &field.attrs {
            if attr.path().is_ident("alias") {
                // Custom #[alias("name")] attribute
                if let syn::Meta::List(list) = &attr.meta {
                    let alias_str: Result<syn::LitStr, _> = list.parse_args();
                    if let Ok(lit) = alias_str {
                        schema_field_name_str = lit.value();
                        has_custom_rename = true;
                    }
                }
            } else if attr.path().is_ident("skip") {
                skip_field = true;
                skip_serializing = true;
                skip_deserializing = true;
            } else if attr.path().is_ident("skip_serializing") {
                skip_serializing = true;
            } else if attr.path().is_ident("skip_deserializing") {
                skip_deserializing = true;
            } else if attr.path().is_ident("read_only") {
                read_only = true;
                skip_deserializing = true;
            } else if attr.path().is_ident("write_only") {
                write_only = true;
                skip_serializing = true;
            } else if attr.path().is_ident("deprecated") {
                field_deprecated = true;
            } else if attr.path().is_ident("serde") {
                // Parse existing serde attr: extract managed items and keep the rest as passthrough.
                if let syn::Meta::List(list) = &attr.meta {
                    let parser =
                        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
                    if let Ok(items) = parser.parse2(list.tokens.clone()) {
                        for item in &items {
                            match item {
                                syn::Meta::Path(p) if p.is_ident("skip") => {
                                    skip_field = true;
                                    skip_serializing = true;
                                    skip_deserializing = true;
                                }
                                syn::Meta::Path(p) if p.is_ident("skip_serializing") => {
                                    skip_serializing = true;
                                }
                                syn::Meta::Path(p) if p.is_ident("skip_deserializing") => {
                                    skip_deserializing = true;
                                }
                                syn::Meta::Path(p) if p.is_ident("default") => {
                                    let field_ty = &field.ty;
                                    field_default_expr = Some(
                                        quote! { <#field_ty as ::core::default::Default>::default() },
                                    );
                                    passthrough_serde_items.push(quote! { default });
                                }
                                syn::Meta::NameValue(nv) if nv.path.is_ident("default") => {
                                    match &nv.value {
                                        syn::Expr::Lit(syn::ExprLit {
                                            lit: syn::Lit::Str(s),
                                            ..
                                        }) => {
                                            if let Ok(default_fn) =
                                                syn::parse_str::<syn::Path>(&s.value())
                                            {
                                                field_default_expr = Some(quote! { #default_fn() });
                                            }
                                        }
                                        syn::Expr::Path(path_expr) => {
                                            let default_fn = &path_expr.path;
                                            field_default_expr = Some(quote! { #default_fn() });
                                        }
                                        syn::Expr::Call(call_expr) => {
                                            let call = &call_expr;
                                            field_default_expr = Some(quote! { #call });
                                        }
                                        _ => {}
                                    }
                                    // Kept as passthrough so it gets re-emitted
                                    passthrough_serde_items.push(quote! { #item });
                                }
                                syn::Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                                    if let syn::Expr::Lit(syn::ExprLit {
                                        lit: syn::Lit::Str(s),
                                        ..
                                    }) = &nv.value
                                    {
                                        schema_field_name_str = s.value();
                                    }
                                    // Kept as passthrough so it gets re-emitted
                                    passthrough_serde_items.push(quote! { #item });
                                }
                                other => {
                                    // Unknown serde item — keep as passthrough
                                    passthrough_serde_items.push(quote! { #other });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Option<T> fields deserialize from missing key as None by default.
        // Register this for exclude_defaults parity even without explicit serde(default).
        if field_default_expr.is_none() {
            if let Some(inner_ty) = option_inner_type(&field.ty) {
                field_default_expr = Some(quote! { ::core::option::Option::<#inner_ty>::None });
            }
        }

        // Build merged serde attribute from passthrough items + managed state.
        // We strip ALL original #[serde(...)] from clean_field and emit a single rebuilt one.
        let mut merged_serde_parts: Vec<proc_macro2::TokenStream> = Vec::new();

        // 1. Passthrough items (except rename if we have a custom rename override)
        if has_custom_rename {
            // Custom #[alias] overrides any existing serde(rename)
            merged_serde_parts.extend(
                passthrough_serde_items
                    .iter()
                    .filter(|t| !t.to_string().starts_with("rename"))
                    .cloned(),
            );
            merged_serde_parts.push(quote! { rename = #schema_field_name_str });
        } else {
            merged_serde_parts.extend(passthrough_serde_items);
        }

        // 2. Skip attributes — emit exactly one skip variant based on final state.
        //    Both existing serde attrs and custom attrs feed into the booleans above.
        if skip_field {
            merged_serde_parts.push(quote! { skip });
        } else {
            if skip_serializing {
                merged_serde_parts.push(quote! { skip_serializing });
            }
            if skip_deserializing {
                merged_serde_parts.push(quote! { skip_deserializing });
            }
        }

        // Emit merged serde attribute if there are any parts
        if !merged_serde_parts.is_empty() {
            let serde_attr = quote! { #[serde(#(#merged_serde_parts),*)] };
            serde_attrs_for_fields.push((field_name.clone(), serde_attr));
        }

        // Collect alias mapping: field_name -> alias (when they differ)
        // This is used by response_model shaping to convert between field names and aliases
        let rust_field_name = field_name.to_string();
        if rust_field_name != schema_field_name_str {
            field_aliases.push((rust_field_name.clone(), schema_field_name_str.clone()));
        }

        if !skip_field && !skip_serializing {
            if let Some(default_expr) = field_default_expr.clone() {
                field_defaults.push((rust_field_name.clone(), default_expr));
            }
        }

        // Extract doc comment for field description
        let field_desc = extract_doc_comment(&field.attrs);
        if !field_desc.is_empty() {
            schema_patches.push(quote! {
                if let Some(prop) = props.get_mut(#schema_field_name_str) {
                    prop.description = Some(#field_desc.to_string());
                }
            });
        }

        // Add read_only and write_only schema patches for OpenAPI spec
        if read_only {
            schema_patches.push(quote! {
                if let Some(prop) = props.get_mut(#schema_field_name_str) {
                    prop.read_only = Some(true);
                }
            });
        }
        if write_only {
            schema_patches.push(quote! {
                if let Some(prop) = props.get_mut(#schema_field_name_str) {
                    prop.write_only = Some(true);
                }
            });
        }
        for attr in &field.attrs {
            if attr.path().is_ident("validate") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("min_length") {
                        let value = meta.value()?;
                        let lit: syn::LitInt = value.parse()?;
                        let min: usize = lit.base10_parse()?;
                        validation_checks.push(quote! {
                            if self.#field_name.len() < #min {
                                errors.push(format!("{}: must be at least {} characters", #schema_field_name_str, #min));
                            }
                        });
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.min_length = Some(#min);
                            }
                        });
                    } else if meta.path.is_ident("max_length") {
                        let value = meta.value()?;
                        let lit: syn::LitInt = value.parse()?;
                        let max: usize = lit.base10_parse()?;
                        validation_checks.push(quote! {
                            if self.#field_name.len() > #max {
                                errors.push(format!("{}: must be at most {} characters", #schema_field_name_str, #max));
                            }
                        });
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.max_length = Some(#max);
                            }
                        });
                    } else if meta.path.is_ident("email") {
                        validation_checks.push(quote! {
                            {
                                let email = &self.#field_name;
                                let at_count = email.chars().filter(|&c| c == '@').count();
                                let valid = at_count == 1
                                    && !email.starts_with('@')
                                    && !email.ends_with('@')
                                    && {
                                        if let Some(at_pos) = email.find('@') {
                                            let domain = &email[at_pos + 1..];
                                            !domain.is_empty() && domain.contains('.')
                                                && !domain.starts_with('.') && !domain.ends_with('.')
                                        } else {
                                            false
                                        }
                                    };
                                if !valid {
                                    errors.push(format!("{}: must be a valid email address", #schema_field_name_str));
                                }
                            }
                        });
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.format = Some("email".to_string());
                            }
                        });
                    } else if meta.path.is_ident("minimum") {
                        let value = meta.value()?;
                        let lit: syn::LitInt = value.parse()?;
                        let min: i64 = lit.base10_parse()?;
                        let min_f64 = min as f64;
                        validation_checks.push(quote! {
                            if (self.#field_name as f64) < #min_f64 {
                                errors.push(format!("{}: must be at least {}", #schema_field_name_str, #min));
                            }
                        });
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.minimum = Some(#min_f64);
                            }
                        });
                    } else if meta.path.is_ident("maximum") {
                        let value = meta.value()?;
                        let lit: syn::LitInt = value.parse()?;
                        let max: i64 = lit.base10_parse()?;
                        let max_f64 = max as f64;
                        validation_checks.push(quote! {
                            if (self.#field_name as f64) > #max_f64 {
                                errors.push(format!("{}: must be at most {}", #schema_field_name_str, #max));
                            }
                        });
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.maximum = Some(#max_f64);
                            }
                        });
                    } else if meta.path.is_ident("pattern") {
                        let value = meta.value()?;
                        let lit: syn::LitStr = value.parse()?;
                        let pat = lit.value();
                        validation_checks.push(quote! {
                            {
                                static RE: std::sync::OnceLock<ultraapi::regex::Regex> = std::sync::OnceLock::new();
                                let re = RE.get_or_init(|| ultraapi::regex::Regex::new(#pat).expect("Invalid regex"));
                                if !re.is_match(&self.#field_name) {
                                    errors.push(format!("{}: must match pattern {}", #schema_field_name_str, #pat));
                                }
                            }
                        });
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.pattern = Some(#pat.to_string());
                            }
                        });
                    } else if meta.path.is_ident("min_items") {
                        let value = meta.value()?;
                        let lit: syn::LitInt = value.parse()?;
                        let min: usize = lit.base10_parse()?;
                        validation_checks.push(quote! {
                            if self.#field_name.len() < #min {
                                errors.push(format!("{}: must have at least {} items", #schema_field_name_str, #min));
                            }
                        });
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.min_items = Some(#min);
                            }
                        });
                    }
                    Ok(())
                });
            } else if attr.path().is_ident("schema") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("example") {
                        let value = meta.value()?;
                        let lit: syn::LitStr = value.parse()?;
                        let example_val = lit.value();
                        schema_patches.push(quote! {
                            if let Some(prop) = props.get_mut(#schema_field_name_str) {
                                prop.example = Some(#example_val.to_string());
                            }
                        });
                    } else if meta.path.is_ident("deprecated") {
                        // Support #[schema(deprecated)] and #[schema(deprecated = true/false)]
                        let mut deprecated_value = true;
                        if meta.input.peek(syn::Token![=]) {
                            let value = meta.value()?;
                            let lit: syn::LitBool = value.parse()?;
                            deprecated_value = lit.value;
                        }
                        if deprecated_value {
                            field_deprecated = true;
                        }
                    }
                    Ok(())
                });
            }
        }

        if field_deprecated {
            schema_patches.push(quote! {
                if let Some(prop) = props.get_mut(#schema_field_name_str) {
                    prop.deprecated = Some(true);
                }
            });
        }

        let mut clean_field = field.clone();
        clean_field.attrs.retain(|a| {
            !a.path().is_ident("validate")
                && !a.path().is_ident("schema")
                && !a.path().is_ident("alias")
                && !a.path().is_ident("skip")
                && !a.path().is_ident("skip_serializing")
                && !a.path().is_ident("skip_deserializing")
                && !a.path().is_ident("read_only")
                && !a.path().is_ident("write_only")
                && !a.path().is_ident("serde") // stripped; rebuilt as merged attr
        });
        clean_fields.push((field_name.clone(), clean_field));
    }

    let name_str = name.to_string();

    let desc_expr = if struct_description.is_empty() {
        quote! { None }
    } else {
        quote! { Some(#struct_description.to_string()) }
    };

    let custom_validation_check = if let Some(custom_fn) = custom_validation_fn {
        quote! {
            if let Err(custom_errors) = #custom_fn(self) {
                errors.extend(custom_errors);
            }
        }
    } else {
        quote! {}
    };

    // Generate field definitions with optional serde attributes
    // Use proc_macro2::TokenStream for quote! compatibility, then convert at the end
    let field_defs: Vec<proc_macro2::TokenStream> = clean_fields
        .iter()
        .map(|(field_name, field)| {
            let serde_attr = serde_attrs_for_fields
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, attr)| attr.clone());

            match serde_attr {
                Some(attr) => quote! {
                    #attr
                    #field
                },
                None => quote! { #field },
            }
        })
        .collect();

    // Build the alias registration code - only if there are aliases
    // We register a function that returns the alias mapping for runtime lookup
    let alias_registration = if !field_aliases.is_empty() {
        // Generate entries as (&str, &str) const pairs
        let alias_entries: Vec<_> = field_aliases
            .iter()
            .map(|(field, alias)| {
                let field_lit = field.as_str();
                let alias_lit = alias.as_str();
                quote! { (#field_lit, #alias_lit) }
            })
            .collect();

        Some(quote! {
            ultraapi::inventory::submit! {
                ultraapi::FieldAliasInfo {
                    type_name: #name_str,
                    get_aliases: || {
                        // Build the HashMap at runtime using const data
                        const PAIRS: &[(&str, &str)] = &[#(#alias_entries),*];
                        static MAP: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();
                        MAP.get_or_init(|| {
                            let mut m = std::collections::HashMap::new();
                            for (k, v) in PAIRS {
                                m.insert(k.to_string(), v.to_string());
                            }
                            m
                        })
                    },
                }
            }
        })
    } else {
        None
    };

    let default_registration = if !field_defaults.is_empty() {
        let default_inserts: Vec<_> = field_defaults
            .iter()
            .map(|(field_name, default_expr)| {
                let field_name_lit = field_name.as_str();
                quote! {
                    if let Ok(default_value) = ultraapi::serde_json::to_value(#default_expr) {
                        m.insert(#field_name_lit.to_string(), default_value);
                    }
                }
            })
            .collect();

        Some(quote! {
            ultraapi::inventory::submit! {
                ultraapi::FieldDefaultInfo {
                    type_name: #name_str,
                    get_defaults: || {
                        static MAP: std::sync::OnceLock<std::collections::HashMap<String, ultraapi::serde_json::Value>> = std::sync::OnceLock::new();
                        MAP.get_or_init(|| {
                            let mut m = std::collections::HashMap::new();
                            #(#default_inserts)*
                            m
                        })
                    },
                }
            }
        })
    } else {
        None
    };

    let output = quote! {
        #(#attrs)*
        #[derive(ultraapi::serde::Serialize, ultraapi::serde::Deserialize, ultraapi::schemars::JsonSchema)]
        #[serde(crate = "ultraapi::serde")]
        #[schemars(crate = "ultraapi::schemars")]
        #vis struct #name #generics {
            #(#field_defs),*
        }

        impl ultraapi::Validate for #name {
            fn validate(&self) -> Result<(), Vec<String>> {
                let mut errors = Vec::new();
                #(#validation_checks)*
                #custom_validation_check
                if errors.is_empty() { Ok(()) } else { Err(errors) }
            }
        }

        impl ultraapi::HasValidate for #name {}

        // Register validator in inventory for ValidatedWrapper to find at runtime
        // Use the simple struct name (without module path) for matching
        ultraapi::inventory::submit! {
            ultraapi::ValidatorInfo {
                type_name: stringify!(#name),
                validate_fn: |any: &dyn std::any::Any| {
                    if let Some(val) = any.downcast_ref::<#name>() {
                        <#name as ultraapi::Validate>::validate(val)
                    } else {
                        Err(vec!["Internal validation error: type mismatch".to_string()])
                    }
                },
            }
        }

        impl ultraapi::HasSchemaPatches for #name {
            fn patch_schema(props: &mut std::collections::HashMap<String, ultraapi::openapi::PropertyPatch>) {
                #(#schema_patches)*
            }
        }

        ultraapi::inventory::submit! {
            ultraapi::SchemaInfo {
                name: #name_str,
                schema_fn: || {
                    static CACHE: std::sync::OnceLock<ultraapi::openapi::Schema> = std::sync::OnceLock::new();
                    CACHE.get_or_init(|| {
                        let base = ultraapi::schemars::schema_for!(#name);
                        let result = ultraapi::openapi::schema_from_schemars_full(#name_str, &base);
                        let mut schema = result.schema;
                        schema.description = #desc_expr;
                        let mut patches = std::collections::HashMap::new();
                        for (name, _) in &schema.properties {
                            patches.insert(name.clone(), ultraapi::openapi::PropertyPatch::default());
                        }
                        <#name as ultraapi::HasSchemaPatches>::patch_schema(&mut patches);
                        for (name, patch) in patches {
                            if let Some(prop) = schema.properties.get_mut(&name) {
                                if patch.min_length.is_some() { prop.min_length = patch.min_length; }
                                if patch.max_length.is_some() { prop.max_length = patch.max_length; }
                                if patch.format.is_some() { prop.format = patch.format.clone(); }
                                if patch.minimum.is_some() { prop.minimum = patch.minimum; }
                                if patch.maximum.is_some() { prop.maximum = patch.maximum; }
                                if patch.pattern.is_some() { prop.pattern = patch.pattern.clone(); }
                                if patch.min_items.is_some() { prop.min_items = patch.min_items; }
                                if patch.description.is_some() { prop.description = patch.description.clone(); }
                                if patch.example.is_some() { prop.example = patch.example.clone(); }
                                if patch.read_only.is_some() { prop.read_only = patch.read_only.unwrap_or(false); }
                                if patch.write_only.is_some() { prop.write_only = patch.write_only.unwrap_or(false); }
                                if patch.deprecated.is_some() { prop.deprecated = patch.deprecated.unwrap_or(false); }
                            }
                        }
                        schema
                    }).clone()
                },
                nested_fn: || {
                    static CACHE: std::sync::OnceLock<std::collections::HashMap<String, ultraapi::openapi::Schema>> = std::sync::OnceLock::new();
                    CACHE.get_or_init(|| {
                        let base = ultraapi::schemars::schema_for!(#name);
                        let result = ultraapi::openapi::schema_from_schemars_full(#name_str, &base);
                        result.nested
                    }).clone()
                },
            }
        }

        #alias_registration
        #default_registration
    };

    output.into()
}
