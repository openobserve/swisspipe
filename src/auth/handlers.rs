use axum::{
    extract::{Query, State},
    http::{StatusCode, header::SET_COOKIE},
    response::{Json, Redirect, Response, IntoResponse},
    routing::get,
    Router,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set, ColumnTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    AppState,
    auth::google::GoogleOAuthService,
    database::{sessions, csrf_tokens},
};

#[derive(Debug, Deserialize)]
pub struct AuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub user: Option<UserInfo>,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: Option<String>,
}

/// Session expiration time in seconds (24 hours)
const SESSION_EXPIRY_SECONDS: i64 = 86400;

/// CSRF token expiration time in seconds (1 hour)
const CSRF_TOKEN_EXPIRY_SECONDS: i64 = 3600;

/// Helper function to determine if we should use secure cookies
/// In development (localhost), we skip the Secure flag to allow HTTP
fn should_use_secure_cookies() -> bool {
    // Check if we're in production or using HTTPS
    // For now, assume development if no explicit setting
    std::env::var("COOKIE_SECURE").unwrap_or_default() == "true"
}

/// Initiate Google OAuth login
pub async fn google_login(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let google_config = state.config.google_oauth
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let google_service = GoogleOAuthService::new(google_config.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (auth_url, csrf_token) = google_service.get_authorization_url();

    // Store CSRF token in database for validation
    let csrf_token_str = csrf_token.secret().to_string();
    let csrf_model = csrf_tokens::Model::new(
        csrf_token_str.clone(),
        CSRF_TOKEN_EXPIRY_SECONDS,
        None, // TODO: Extract IP from request
        None, // TODO: Extract User-Agent from request
    );

    let csrf_active_model = csrf_tokens::ActiveModel {
        id: Set(csrf_model.id.clone()),
        token: Set(csrf_model.token.clone()),
        created_at: Set(csrf_model.created_at),
        expires_at: Set(csrf_model.expires_at),
        used: Set(csrf_model.used),
        ip_address: Set(csrf_model.ip_address.clone()),
        user_agent: Set(csrf_model.user_agent.clone()),
    };

    csrf_active_model.insert(&*state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set CSRF token in secure cookie
    let mut response = Redirect::temporary(&auth_url).into_response();
    let secure_flag = if should_use_secure_cookies() { "; Secure" } else { "" };
    let cookie_value = format!(
        "csrf_token={csrf_token_str}; HttpOnly{secure_flag}; SameSite=Lax; Path=/; Max-Age={CSRF_TOKEN_EXPIRY_SECONDS}"
    );
    response.headers_mut().insert(axum::http::header::SET_COOKIE, cookie_value.parse().unwrap());

    Ok(response)
}

/// Handle Google OAuth callback
pub async fn google_callback(
    Query(params): Query<AuthCallbackQuery>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Check for error in callback
    if let Some(error) = params.error {
        tracing::warn!("OAuth callback error: {}", error);
        let base_url = state.config.google_oauth.as_ref().map(|oauth| {
            oauth.redirect_url.rsplit_once("/auth/google/callback").map(|(base, _)| base).unwrap_or("http://localhost:3700")
        }).unwrap_or(&format!("http://localhost:{}", state.config.port));
        return Ok(Redirect::temporary(&format!("{}/auth/callback?error={error}", base_url)).into_response());
    }

    let code = params.code.ok_or_else(|| {
        tracing::warn!("OAuth callback missing authorization code");
        (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            success: false,
            error: "Missing authorization code".to_string(),
            details: Some("The OAuth callback did not include the required authorization code parameter.".to_string()),
        }))
    })?;
    let oauth_state = params.state.unwrap_or_default();

    // Extract CSRF token from cookie
    let csrf_token_from_cookie = headers
        .get("cookie")
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|part| {
                    let trimmed = part.trim();
                    trimmed.strip_prefix("csrf_token=").map(|token_part| token_part.to_string())
                })
        })
        .ok_or_else(|| {
            tracing::warn!("OAuth callback missing CSRF token cookie");
            (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                success: false,
                error: "Missing CSRF token".to_string(),
                details: Some("The required CSRF token cookie was not found. Please restart the authentication process.".to_string()),
            }))
        })?;

    // Validate CSRF token from database
    let csrf_record = csrf_tokens::Entity::find()
        .filter(csrf_tokens::Column::Token.eq(&csrf_token_from_cookie))
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error validating CSRF token: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                success: false,
                error: "Database error".to_string(),
                details: Some("Unable to validate CSRF token due to database error.".to_string()),
            }))
        })?
        .ok_or_else(|| {
            tracing::warn!("CSRF token not found in database");
            (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                success: false,
                error: "Invalid CSRF token".to_string(),
                details: Some("The CSRF token was not found or has expired. Please restart the authentication process.".to_string()),
            }))
        })?;

    if !csrf_record.is_valid() {
        tracing::warn!("Invalid or expired CSRF token for OAuth callback");
        return Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            success: false,
            error: "CSRF token expired".to_string(),
            details: Some("The CSRF token has expired or has already been used. Please restart the authentication process.".to_string()),
        })));
    }

    // Mark CSRF token as used
    let mut csrf_active_model: csrf_tokens::ActiveModel = csrf_record.into();
    csrf_active_model.used = Set(true);
    csrf_active_model.update(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error updating CSRF token: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                success: false,
                error: "Database error".to_string(),
                details: Some("Unable to update CSRF token due to database error.".to_string()),
            }))
        })?;

    let google_config = state.config.google_oauth
        .as_ref()
        .ok_or_else(|| {
            tracing::error!("Google OAuth not configured");
            (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse {
                success: false,
                error: "OAuth not configured".to_string(),
                details: Some("Google OAuth is not properly configured on this server.".to_string()),
            }))
        })?;

    let google_service = GoogleOAuthService::new(google_config.clone())
        .map_err(|e| {
            tracing::error!("Failed to create Google OAuth service: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                success: false,
                error: "OAuth service error".to_string(),
                details: Some("Unable to initialize Google OAuth service.".to_string()),
            }))
        })?;

    match google_service.exchange_code_and_get_user_info(&code, &oauth_state, &csrf_token_from_cookie).await {
        Ok(user_info) => {
            // Create database session
            let session_id = Uuid::new_v4().to_string();
            let session_model = sessions::Model::from_google_user_info(
                session_id.clone(),
                &user_info,
                SESSION_EXPIRY_SECONDS,
                None, // TODO: Extract IP from request
                None, // TODO: Extract User-Agent from request
            );

            let session_active_model = sessions::ActiveModel {
                id: Set(session_model.id.clone()),
                user_id: Set(session_model.user_id),
                email: Set(session_model.email),
                name: Set(session_model.name),
                given_name: Set(session_model.given_name),
                family_name: Set(session_model.family_name),
                picture: Set(session_model.picture),
                locale: Set(session_model.locale),
                hosted_domain: Set(session_model.hosted_domain),
                verified_email: Set(session_model.verified_email),
                created_at: Set(session_model.created_at),
                last_accessed_at: Set(session_model.last_accessed_at),
                expires_at: Set(session_model.expires_at),
                ip_address: Set(session_model.ip_address),
                user_agent: Set(session_model.user_agent),
            };

            session_active_model.insert(&*state.db)
                .await
                .map_err(|e| {
                    tracing::error!("Database error creating session: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                        success: false,
                        error: "Session creation failed".to_string(),
                        details: Some("Unable to create session due to database error.".to_string()),
                    }))
                })?;

            tracing::info!("User {} successfully authenticated via Google OAuth", user_info.email);

            // Set session cookie and redirect to frontend callback
            let base_url = state.config.google_oauth.as_ref().map(|oauth| {
                oauth.redirect_url.rsplit_once("/auth/google/callback").map(|(base, _)| base).unwrap_or("http://localhost:3700")
            }).unwrap_or(&format!("http://localhost:{}", state.config.port));
            let mut response = Redirect::temporary(&format!("{}/auth/callback", base_url)).into_response();
            let secure_flag = if should_use_secure_cookies() { "; Secure" } else { "" };
            let cookie_value = format!("session_id={session_id}; HttpOnly{secure_flag}; SameSite=Lax; Path=/; Max-Age={SESSION_EXPIRY_SECONDS}");
            response.headers_mut().insert(SET_COOKIE, cookie_value.parse().unwrap());

            // Clear CSRF token cookie
            let clear_csrf_cookie = format!("csrf_token=; HttpOnly{secure_flag}; SameSite=Lax; Path=/; Max-Age=0");
            response.headers_mut().append(SET_COOKIE, clear_csrf_cookie.parse().unwrap());

            Ok(response)
        }
        Err(e) => {
            tracing::error!("Google OAuth authentication failed: {}", e);
            let base_url = state.config.google_oauth.as_ref().map(|oauth| {
                oauth.redirect_url.rsplit_once("/auth/google/callback").map(|(base, _)| base).unwrap_or("http://localhost:3700")
            }).unwrap_or(&format!("http://localhost:{}", state.config.port));
            Ok(Redirect::temporary(&format!("{}/auth/callback?error=authentication_failed", base_url)).into_response())
        }
    }
}

/// Get current user info (for authenticated requests)
pub async fn get_user_info(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract session ID from cookie
    let session_id = headers
        .get("cookie")
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|part| {
                    let trimmed = part.trim();
                    trimmed.strip_prefix("session_id=").map(|session_part| session_part.to_string())
                })
        })
        .ok_or_else(|| {
            tracing::warn!("User info request missing session cookie");
            (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                success: false,
                error: "No session".to_string(),
                details: Some("No valid session found. Please log in.".to_string()),
            }))
        })?;

    // Look up session in database
    let session_record = sessions::Entity::find()
        .filter(sessions::Column::Id.eq(&session_id))
        .one(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error looking up session: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                success: false,
                error: "Database error".to_string(),
                details: Some("Unable to look up session.".to_string()),
            }))
        })?
        .ok_or_else(|| {
            tracing::warn!("Session not found in database");
            (StatusCode::UNAUTHORIZED, Json(ErrorResponse {
                success: false,
                error: "Invalid session".to_string(),
                details: Some("Session not found or expired. Please log in again.".to_string()),
            }))
        })?;

    // Check if session is expired
    if session_record.is_expired() {
        // Delete expired session
        sessions::Entity::delete_by_id(&session_id)
            .exec(&*state.db)
            .await
            .map_err(|e| {
                tracing::error!("Database error deleting expired session: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                    success: false,
                    error: "Database error".to_string(),
                    details: Some("Unable to clean up expired session.".to_string()),
                }))
            })?;

        return Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse {
            success: false,
            error: "Session expired".to_string(),
            details: Some("Your session has expired. Please log in again.".to_string()),
        })));
    }

    // Update last accessed time
    let mut session_active_model: sessions::ActiveModel = session_record.clone().into();
    session_active_model.last_accessed_at = Set(chrono::Utc::now().timestamp());
    session_active_model.update(&*state.db)
        .await
        .map_err(|e| {
            tracing::error!("Database error updating session access time: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                success: false,
                error: "Database error".to_string(),
                details: Some("Unable to update session.".to_string()),
            }))
        })?;

    let user = UserInfo {
        id: session_record.user_id,
        email: session_record.email,
        name: session_record.name,
        picture: session_record.picture,
    };

    let response = LoginResponse {
        success: true,
        message: "User info retrieved".to_string(),
        user: Some(user),
        session_id: Some(session_id),
    };

    Ok(Json(response))
}

/// Logout endpoint
pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    // Extract session ID from cookie
    if let Some(session_id) = headers
        .get("cookie")
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|part| {
                    let trimmed = part.trim();
                    trimmed.strip_prefix("session_id=").map(|session_part| session_part.to_string())
                })
        })
    {
        // Remove session from database
        sessions::Entity::delete_by_id(&session_id)
            .exec(&*state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        tracing::info!("User session {} logged out", session_id);
    }

    let response = LoginResponse {
        success: true,
        message: "Logged out successfully".to_string(),
        user: None,
        session_id: None,
    };

    // Clear session cookie
    let mut response = Json(response).into_response();
    let secure_flag = if should_use_secure_cookies() { "; Secure" } else { "" };
    let cookie_value = format!("session_id=; HttpOnly{secure_flag}; SameSite=Lax; Path=/; Max-Age=0");
    response.headers_mut().insert(SET_COOKIE, cookie_value.parse().unwrap());

    Ok(response)
}

/// Check if a session is valid (for middleware use)
pub async fn is_session_valid(session_id: &str, db: &sea_orm::DatabaseConnection) -> bool {
    match sessions::Entity::find()
        .filter(sessions::Column::Id.eq(session_id))
        .one(db)
        .await
    {
        Ok(Some(session)) => !session.is_expired(),
        _ => false,
    }
}

/// Get user info from session (for middleware use)
#[allow(dead_code)]
pub async fn get_user_from_session(
    session_id: &str,
    db: &sea_orm::DatabaseConnection,
) -> Option<sessions::Model> {
    match sessions::Entity::find()
        .filter(sessions::Column::Id.eq(session_id))
        .one(db)
        .await
    {
        Ok(Some(session)) if !session.is_expired() => Some(session),
        _ => None,
    }
}

/// Create auth routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/google/login", get(google_login))
        .route("/google/callback", get(google_callback))
        .route("/user", get(get_user_info))
        .route("/logout", get(logout))
}