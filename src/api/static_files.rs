use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{Html, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use crate::AppState;

#[derive(RustEmbed)]
#[folder = "dist"]
struct Assets;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/assets/*path", get(static_handler))
        .route("/favicon.ico", get(favicon))
        .route("/openapi.yaml", get(openapi_spec))
        .route("/monacoeditorwork/*path", get(monaco_handler))
        // Catch-all for SPA routing - serve index.html for any unmatched route
        .route("/*path", get(spa_handler))
}

async fn index() -> Html<String> {
    match Assets::get("index.html") {
        Some(content) => {
            let body = std::str::from_utf8(content.data.as_ref()).unwrap();
            Html(body.to_string())
        }
        None => Html("<h1>Index not found</h1>".to_string()),
    }
}

async fn static_handler(Path(path): Path<String>) -> Result<Response, StatusCode> {
    let path = format!("assets/{path}");
    serve_static_file(&path).await
}

async fn favicon() -> Result<Response, StatusCode> {
    serve_static_file("favicon.ico").await
}

async fn openapi_spec() -> Result<Response, StatusCode> {
    serve_static_file("openapi.yaml").await
}

async fn monaco_handler(Path(path): Path<String>) -> Result<Response, StatusCode> {
    let path = format!("monacoeditorwork/{path}");
    serve_static_file(&path).await
}

async fn spa_handler(Path(_path): Path<String>) -> Html<String> {
    // For SPA routing, always serve index.html for unmatched routes
    match Assets::get("index.html") {
        Some(content) => {
            let body = std::str::from_utf8(content.data.as_ref()).unwrap();
            Html(body.to_string())
        }
        None => Html("<h1>404 - Page not found</h1>".to_string()),
    }
}

async fn serve_static_file(path: &str) -> Result<Response, StatusCode> {
    match Assets::get(path) {
        Some(content) => {
            let mime_type = if path.ends_with(".yaml") || path.ends_with(".yml") {
                "text/yaml".to_string()
            } else {
                mime_guess::from_path(path).first_or_octet_stream().to_string()
            };
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, mime_type.parse().unwrap());

            // Add caching headers for static assets
            if path.starts_with("assets/") {
                headers.insert(header::CACHE_CONTROL, "public, max-age=31536000".parse().unwrap());
            }

            let mut response = Response::builder().status(StatusCode::OK);

            // Apply headers
            for (key, value) in headers.iter() {
                response = response.header(key, value);
            }

            Ok(response.body(content.data.into()).unwrap())
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}