use serde::{Deserialize, Serialize};
use reqwest::Client;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, TokenUrl,
    RedirectUrl, Scope, AuthorizationCode, TokenResponse,
    basic::BasicClient, reqwest::async_http_client,
};
use crate::config::GoogleOAuthConfig;
use crate::workflow::errors::SwissPipeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
    pub locale: Option<String>,
    pub hd: Option<String>, // Hosted domain for G Suite users
}

#[derive(Debug, Clone)]
pub struct GoogleOAuthService {
    client: BasicClient,
    config: GoogleOAuthConfig,
    http_client: Client,
}

impl GoogleOAuthService {
    pub fn new(config: GoogleOAuthConfig) -> Result<Self, SwissPipeError> {
        let google_client_id = ClientId::new(config.client_id.clone());
        let google_client_secret = ClientSecret::new(config.client_secret.clone());
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string())
            .map_err(|e| SwissPipeError::Config(format!("Invalid auth URL: {e}")))?;
        let token_url = TokenUrl::new("https://accounts.google.com/o/oauth2/token".to_string())
            .map_err(|e| SwissPipeError::Config(format!("Invalid token URL: {e}")))?;
        let redirect_url = RedirectUrl::new(config.redirect_url.clone())
            .map_err(|e| SwissPipeError::Config(format!("Invalid redirect URL: {e}")))?;

        let client = BasicClient::new(
            google_client_id,
            Some(google_client_secret),
            auth_url,
            Some(token_url)
        ).set_redirect_uri(redirect_url);

        let http_client = Client::new();

        Ok(Self {
            client,
            config,
            http_client,
        })
    }

    /// Generate authorization URL for Google OAuth flow
    pub fn get_authorization_url(&self) -> (String, CsrfToken) {
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .url();

        (auth_url.to_string(), csrf_token)
    }

    /// Exchange authorization code for access token and get user info
    pub async fn exchange_code_and_get_user_info(
        &self,
        code: &str,
        state: &str,
        expected_csrf_token: &str,
    ) -> Result<GoogleUserInfo, SwissPipeError> {
        // Validate CSRF token to prevent state fixation attacks
        if state != expected_csrf_token {
            return Err(SwissPipeError::Auth(
                "Invalid CSRF token - potential security attack detected".to_string()
            ));
        }
        // Exchange the code for an access token
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| SwissPipeError::Auth(format!("Failed to exchange OAuth code: {e}")))?;

        let access_token = token_response.access_token().secret();

        // Use the access token to get user info
        let user_info = self.get_user_info(access_token).await?;

        // Validate domain restrictions if configured
        self.validate_user_domain(&user_info)?;

        Ok(user_info)
    }

    /// Get user info from Google API using access token
    async fn get_user_info(&self, access_token: &str) -> Result<GoogleUserInfo, SwissPipeError> {
        let response = self
            .http_client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| SwissPipeError::Auth(format!("Failed to fetch user info: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(SwissPipeError::Auth(format!(
                "Google API returned error {status}: {text}"
            )));
        }

        let user_info: GoogleUserInfo = response
            .json()
            .await
            .map_err(|e| SwissPipeError::Auth(format!("Failed to parse user info: {e}")))?;

        Ok(user_info)
    }

    /// Validate that the user's email domain is allowed
    fn validate_user_domain(&self, user_info: &GoogleUserInfo) -> Result<(), SwissPipeError> {
        // If no domain restrictions are configured, allow all users
        if self.config.allowed_domains.is_empty() {
            return Ok(());
        }

        // Extract domain from email
        let email_domain = user_info.email
            .split('@')
            .nth(1)
            .ok_or_else(|| SwissPipeError::Auth("Invalid email format".to_string()))?;

        // Check if the domain is in the allowed list
        if !self.config.allowed_domains.contains(&email_domain.to_string()) {
            tracing::warn!(
                "User {} from domain {} attempted login but domain not allowed. Allowed domains: {:?}",
                user_info.email,
                email_domain,
                self.config.allowed_domains
            );
            return Err(SwissPipeError::Auth(format!(
                "Domain '{email_domain}' is not allowed for authentication"
            )));
        }

        // Additional check for Google Workspace hosted domain
        if let Some(hd) = &user_info.hd {
            if !self.config.allowed_domains.contains(hd) {
                tracing::warn!(
                    "User {} from hosted domain {} attempted login but hosted domain not allowed",
                    user_info.email,
                    hd
                );
                return Err(SwissPipeError::Auth(format!(
                    "Hosted domain '{hd}' is not allowed for authentication"
                )));
            }
        }

        tracing::info!(
            "User {} from domain {} successfully authenticated",
            user_info.email,
            email_domain
        );

        Ok(())
    }

}