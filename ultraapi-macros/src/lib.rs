use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::Parser, parse_macro_input, FnArg, ItemEnum, ItemFn, ItemStruct, LitInt, LitStr, PatType,
    Path, PathSegment, Type,
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

fn is_header_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "TypedHeader";
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

fn route_macro_impl(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr).value();
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;

    // Parse custom attributes: #[status(N)], #[tag("x")], #[security("x")],
    // #[response_model(include={"a","b"})], #[response_model(exclude={"a","b"})],
    // #[response_model(by_alias=true)], #[response_class("json"|"html"|"text"|"binary"|"stream"|"xml")], doc comments
    let mut status_code: Option<u16> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut security_schemes: Vec<String> = Vec::new();
    // Response model shaping options
    let mut has_response_model: bool = false; // Track if #[response_model(...)] was used
    let mut include_fields: Option<Vec<String>> = None;
    let mut exclude_fields: Option<Vec<String>> = None;
    let mut by_alias: bool = false;
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
        } else if attr.path().is_ident("response_model") {
            // Mark that response_model attribute was used
            has_response_model = true;
            // Parse response_model(include={"a","b"}, exclude={"c"}, by_alias=true)
            // Simplified parsing: look for key=value patterns in tokens
            if let syn::Meta::List(list) = &attr.meta {
                let tokens = list.tokens.clone();
                let tokens_str = tokens.to_string();

                // Extract include fields
                if let Some(start) = tokens_str.find("include") {
                    let after_include = &tokens_str[start..];
                    if let Some(eq_pos) = after_include.find('=') {
                        let after_eq = &after_include[eq_pos + 1..];
                        let end_pos = after_eq
                            .find(|c: char| {
                                !c.is_alphanumeric()
                                    && c != '"'
                                    && c != '{'
                                    && c != '}'
                                    && c != ','
                                    && c != ' '
                            })
                            .map(|p| p + eq_pos + start + 1)
                            .unwrap_or(tokens_str.len());
                        let fields_str =
                            &tokens_str[eq_pos + 1 + start..end_pos.min(tokens_str.len())];
                        let fields: Vec<String> = fields_str
                            .split(',')
                            .map(|s| {
                                s.trim()
                                    .trim_matches('"')
                                    .trim_matches('{')
                                    .trim_matches('}')
                            })
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect();
                        if !fields.is_empty() {
                            include_fields = Some(fields);
                        }
                    }
                }

                // Extract exclude fields
                if let Some(start) = tokens_str.find("exclude") {
                    let after_exclude = &tokens_str[start..];
                    if let Some(eq_pos) = after_exclude.find('=') {
                        let after_eq = &after_exclude[eq_pos + 1..];
                        let end_pos = after_eq
                            .find(|c: char| {
                                !c.is_alphanumeric()
                                    && c != '"'
                                    && c != '{'
                                    && c != '}'
                                    && c != ','
                                    && c != ' '
                            })
                            .map(|p| p + eq_pos + start + 1)
                            .unwrap_or(tokens_str.len());
                        let fields_str =
                            &tokens_str[eq_pos + 1 + start..end_pos.min(tokens_str.len())];
                        let fields: Vec<String> = fields_str
                            .split(',')
                            .map(|s| {
                                s.trim()
                                    .trim_matches('"')
                                    .trim_matches('{')
                                    .trim_matches('}')
                            })
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect();
                        if !fields.is_empty() {
                            exclude_fields = Some(fields);
                        }
                    }
                }

                // Extract by_alias
                if tokens_str.contains("by_alias") {
                    if tokens_str.contains("by_alias=true")
                        || tokens_str.contains("by_alias = true")
                    {
                        by_alias = true;
                    } else if tokens_str.contains("by_alias=false")
                        || tokens_str.contains("by_alias = false")
                    {
                        by_alias = false;
                    }
                }

                // Extract content_type
                if let Some(start) = tokens_str.find("content_type") {
                    let after_ct = &tokens_str[start..];
                    if let Some(eq_pos) = after_ct.find('=') {
                        let after_eq = &after_ct[eq_pos + 1..];
                        // Find the end of the string value (next comma, closing paren, or end)
                        let end_pos = after_eq
                            .find(|c: char| c == ',' || c == ')')
                            .map(|p| p)
                            .unwrap_or(after_eq.len());
                        let ct_value = after_eq[..end_pos].trim().trim_matches('"').trim();
                        if !ct_value.is_empty() {
                            response_model_content_type = Some(ct_value.to_string());
                        }
                    }
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
            // Not doc, not status/tag/security/response_model/response_class/callback - keep it (e.g. serde, schemars)
            clean_attrs.push(attr);
        }
    }

    // Default status codes
    let default_status: u16 = match method {
        "post" => 201,
        "delete" => 204,
        _ => 200,
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
    let mut has_generator_deps = false;
    let mut body_type: Option<&Type> = None;
    let mut path_param_types: Vec<(&syn::Ident, &Type)> = Vec::new();
    let mut query_type: Option<&Type> = None;
    let mut query_extraction = quote! {};

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
                            // Track that we have generator dependencies - will create shared scope at handler start
                            has_generator_deps = true;

                            dep_extractions.push(quote! {
                                // Check if it's a registered generator (yield-based dependency)
                                // Uses shared dep_scope created at handler start for proper cleanup tracking
                                let #pat: ultraapi::Depends<#inner> = if let Some(resolver) = state.get_depends_resolver() {
                                    if resolver.is_generator::<#inner>() {
                                        match resolver.resolve_generator::<#inner>(&state, &dep_scope).await {
                                            Ok(dep) => {
                                                // Cast from Arc<dyn Any> back to Arc<#inner>
                                                let dep_any = dep.clone();
                                                let dep_typed: std::sync::Arc<#inner> = dep_any.downcast()
                                                    .map_err(|_| ultraapi::ApiError::internal(
                                                        format!("Type mismatch for generator: {}", std::any::type_name::<#inner>())
                                                    ))?;
                                                ultraapi::Depends(dep_typed)
                                            }
                                            Err(e) => return Err(ultraapi::ApiError::internal(e.to_string())),
                                        }
                                    } else {
                                        // Not a generator - use regular resolve
                                        match resolver.resolve::<#inner>(&state).await {
                                            Ok(dep) => ultraapi::Depends(dep),
                                            Err(e) => return Err(ultraapi::ApiError::internal(e.to_string())),
                                        }
                                    }
                                } else {
                                    // Fallback: try direct AppState resolution for simple cases
                                    state.get::<#inner>()
                                        .map(ultraapi::Depends)
                                        .ok_or_else(|| ultraapi::ApiError::internal(
                                            format!("Dependency not registered: {}", std::any::type_name::<#inner>())
                                        ))?
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
                // TypedHeader<T> extractor
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::axum_extra::extract::TypedHeader<#inner> =
                                    ultraapi::axum_extra::extract::TypedHeader::from_request_parts(&mut parts, &state).await
                                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid header: {}", e)))?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
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
                        call_args.push(quote!(#pat));
                    }
                }
            } else if is_form_type(ty) {
                // Form<T> extractor for application/x-www-form-urlencoded
                has_form_body = true;
                if let Type::Path(tp) = ty.as_ref() {
                    if let Some(seg) = tp.path.segments.last() {
                        if let Some(inner) = extract_inner_type(seg) {
                            dep_extractions.push(quote! {
                                let #pat: ultraapi::axum::extract::Form<#inner> =
                                    ultraapi::axum::extract::Form::from_request(req, &state).await
                                    .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid form data: {}", e)))?;
                                #pat.validate().map_err(|e| ultraapi::ApiError::validation_error(e))?;
                            });
                            call_args.push(quote!(#pat));
                        }
                    }
                }
            } else if is_multipart_type(ty) {
                // Multipart extractor for file uploads
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
            } else if path_params.contains(&param_name) {
                if let syn::Pat::Ident(pi) = pat.as_ref() {
                    path_param_types.push((&pi.ident, ty));
                    call_args.push(quote!(#pat));
                }
            } else if !is_primitive_type(ty)
                && !is_header_type(ty)
                && !is_cookie_type(ty)
                && !is_form_type(ty)
                && !is_multipart_type(ty)
                && !is_oauth2_password_bearer_type(ty)
                && !is_optional_oauth2_password_bearer_type(ty)
                && !is_oauth2_auth_code_bearer_type(ty)
                && !is_optional_oauth2_auth_code_bearer_type(ty)
                && !is_session_type(ty)
            {
                has_body = true;
                body_type = Some(ty);
                call_args.push(quote!(#pat));
            } else {
                call_args.push(quote!(#pat));
            }
        }
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

    let body_extraction = if has_form_body {
        // Form body is handled in the dep_extractions loop
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
                        && !is_primitive_type(ty)
                        && !is_query_type(ty)
                        && !is_header_type(ty)
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
        quote! {
            let ultraapi::axum::Json(#bpat): ultraapi::axum::Json<#bty> =
                ultraapi::axum::Json::from_request(req, &state).await
                .map_err(|e| ultraapi::ApiError::bad_request(format!("Invalid body: {}", e)))?;
            #bpat.validate().map_err(|e| ultraapi::ApiError::validation_error(e))?;
        }
    } else {
        quote! { let _ = req; }
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

        quote! {
            let shaping_options = ultraapi::ResponseModelOptions {
                include: #include_expr,
                exclude: #exclude_expr,
                by_alias: #by_alias_expr,
                content_type: None, // content_type only affects OpenAPI, not runtime
            };
            let value = shaping_options.apply_with_aliases(value, #type_name_expr, #by_alias);
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
                }
            }
        })
        .collect();

    let body_type_name = body_type.map(get_type_name).unwrap_or_default();
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

    let query_params_fn_expr = if let Some(qt) = query_type {
        quote! { Some(|| {
            let root = ultraapi::schemars::schema_for!(#qt);
            ultraapi::openapi::query_params_from_schema(&root)
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

    // Generate conditional scope creation based on whether there are generator dependencies
    let scope_creation = if has_generator_deps {
        quote! {
            // Create shared dependency scope for cleanup tracking (yield-based dependencies)
            let dep_scope = std::sync::Arc::new(ultraapi::DependencyScope::new());
        }
    } else {
        quote! {
            // No generator dependencies - create dummy scope for compatibility
            let dep_scope = std::sync::Arc::new(ultraapi::DependencyScope::new());
        }
    };

    // Generate cleanup wrapping around response
    // Function cleanup runs BEFORE response is returned (both success and error)
    // Request cleanup runs AFTER response is sent (spawned task)
    let cleanup_wrapper = if has_generator_deps {
        quote! {
            // Clone scope for request cleanup (runs after response is sent)
            let dep_scope_for_request = dep_scope.clone();

            // Run function-scope cleanup - capture result to return after cleanup
            let result: Result<ultraapi::axum::response::Response, ultraapi::ApiError> = #response_expr;

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
            #(#dep_extractions)*
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
            success_status: #status_lit,
            description: #description,
            tags: &[#(#tags),*],
            security: &[#(#security_schemes),*],
            query_params_fn: #query_params_fn_expr,
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
                                let #pat: ultraapi::Depends<#inner> = if let Some(resolver) = state.get_depends_resolver() {
                                    match resolver.resolve::<#inner>(&state).await {
                                        Ok(dep) => ultraapi::Depends(dep),
                                        Err(e) => return Err(ultraapi::ApiError::internal(e.to_string()).into_response()),
                                    }
                                } else {
                                    state.get::<#inner>()
                                        .map(ultraapi::Depends)
                                        .ok_or_else(|| ultraapi::ApiError::internal(
                                            format!("Dependency not registered: {}", std::any::type_name::<#inner>())
                                        ).into_response())?
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
            success_status: 101,
            description: #description,
            tags: &[#(#tags),*],
            security: &[#(#security_schemes),*],
            query_params_fn: None,
            response_model_options: ultraapi::ResponseModelOptions {
                include: None,
                exclude: None,
                by_alias: false,
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
                                let #pat: ultraapi::Depends<#inner> = if let Some(resolver) = state.get_depends_resolver() {
                                    match resolver.resolve::<#inner>(&state).await {
                                        Ok(dep) => ultraapi::Depends(dep),
                                        Err(e) => return Err(ultraapi::ApiError::internal(e.to_string())),
                                    }
                                } else {
                                    state.get::<#inner>()
                                        .map(ultraapi::Depends)
                                        .ok_or_else(|| ultraapi::ApiError::internal(
                                            format!("Dependency not registered: {}", std::any::type_name::<#inner>())
                                        ))?
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
            success_status: #status_lit,
            description: #description,
            tags: &[#(#tags),*],
            security: &[#(#security_schemes),*],
            query_params_fn: None,
            response_model_options: ultraapi::ResponseModelOptions {
                include: None,
                exclude: None,
                by_alias: false,
                content_type: None,
            },
            response_class: ultraapi::ResponseClass::Stream, // SSE uses stream
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

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let mut schema_field_name_str = field_name.to_string();
        let mut field_serde_attrs = Vec::new();
        let mut skip_field = false;
        let mut skip_serializing = false;
        let mut skip_deserializing = false;
        let mut read_only = false;
        let mut write_only = false;

        // Parse custom attributes: alias, skip, skip_serializing, skip_deserializing, read_only, write_only
        // Also support standard serde attributes
        for attr in &field.attrs {
            if attr.path().is_ident("alias") {
                // Custom #[alias("name")] attribute - parse directly from meta
                if let syn::Meta::List(list) = &attr.meta {
                    // Parse the tokens as a lit_str
                    let alias_str: Result<syn::LitStr, _> = list.parse_args();
                    if let Ok(lit) = alias_str {
                        schema_field_name_str = lit.value();
                        field_serde_attrs.push(quote! { rename = #schema_field_name_str });
                    }
                }
            } else if attr.path().is_ident("skip") {
                // Custom #[skip] attribute - skip both serialization and deserialization
                skip_field = true;
                skip_serializing = true;
                skip_deserializing = true;
            } else if attr.path().is_ident("skip_serializing") {
                // Custom #[skip_serializing] attribute
                skip_serializing = true;
            } else if attr.path().is_ident("skip_deserializing") {
                // Custom #[skip_deserializing] attribute
                skip_deserializing = true;
            } else if attr.path().is_ident("read_only") {
                // Custom #[read_only] attribute - field appears in response but not in request
                read_only = true;
                skip_deserializing = true; // Don't accept in request body
            } else if attr.path().is_ident("write_only") {
                // Custom #[write_only] attribute - field appears in request but not in response
                write_only = true;
                skip_serializing = true; // Don't include in response
            } else if attr.path().is_ident("serde") {
                // Standard serde attributes (rename, skip, etc.)
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value = meta.value()?;
                        let lit: syn::LitStr = value.parse()?;
                        schema_field_name_str = lit.value();
                        field_serde_attrs.push(quote! { rename = #schema_field_name_str });
                    } else if meta.path.is_ident("skip") {
                        skip_field = true;
                        skip_serializing = true;
                        skip_deserializing = true;
                    } else if meta.path.is_ident("skip_serializing") {
                        skip_serializing = true;
                    } else if meta.path.is_ident("skip_deserializing") {
                        skip_deserializing = true;
                    }
                    Ok(())
                });
            }
        }

        // Build serde attribute for this field if needed
        // Only add if not already present (check if field already has a serde attribute)
        let has_serde_attr = field.attrs.iter().any(|a| a.path().is_ident("serde"));

        // Collect all serde-related parts: rename from alias + skip attributes
        let mut all_serde_parts: Vec<proc_macro2::TokenStream> = Vec::new();

        // Add rename from alias if present
        for attr in &field_serde_attrs {
            all_serde_parts.push(attr.clone());
        }

        // Add skip attributes - but not twice if read_only/write_only already set them
        // Note: read_only sets skip_deserializing, write_only sets skip_serializing
        if skip_serializing && !has_serde_attr {
            all_serde_parts.push(quote! { skip_serializing });
        }
        if skip_deserializing && !has_serde_attr {
            all_serde_parts.push(quote! { skip_deserializing });
        }

        // Only add serde attribute if there are parts to add AND no existing serde attribute
        if !all_serde_parts.is_empty() && !has_serde_attr {
            let serde_attr = quote! { #[serde(#(#all_serde_parts),*)] };
            serde_attrs_for_fields.push((field_name.clone(), serde_attr));
        }

        // Collect alias mapping: field_name -> alias (when they differ)
        // This is used by response_model shaping to convert between field names and aliases
        let rust_field_name = field_name.to_string();
        if rust_field_name != schema_field_name_str {
            field_aliases.push((rust_field_name, schema_field_name_str.clone()));
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
                    }
                    Ok(())
                });
            }
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
    };

    output.into()
}
