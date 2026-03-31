
use crate::memory_db::UsersStore;
use crate::shared_state::UnifiedAppState;
use argon2::{password_hash::SaltString, Argon2, PasswordHasher, PasswordVerifier};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tracing::{error, info, warn};

pub struct GoogleOAuthPending {
    pub states: Arc<StdMutex<HashMap<String, (String, Option<GoogleOAuthResult>)>>>,
    pub client_id: String,
    pub client_secret: String,
}

impl Clone for GoogleOAuthPending {
    fn clone(&self) -> Self {
        Self {
            states: Arc::clone(&self.states),
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
        }
    }
}

#[derive(Clone, Serialize, Debug)]
pub struct GoogleOAuthResult {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Clone)]
pub struct AuthState {
    pub users: UsersStore,
    pub jwt_secret: String,
    
    pub google: Option<GoogleOAuthPending>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub name: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct MeRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    pub user: Option<UserResponse>,
    pub token: Option<String>,
    pub requires_verification: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserResponse {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleInitRequest {
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GoogleStatusQuery {
    pub state: String,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    id: String,
    email: String,
    name: String,
    picture: Option<String>,
}

const JWT_EXPIRY_HOURS: i64 = 24 * 7;

fn create_jwt_token(email: &str, name: &str, secret: &str) -> Result<String, String> {
    let now = chrono::Utc::now().timestamp();
    let exp = now + (JWT_EXPIRY_HOURS * 3600);

    let claims = Claims {
        sub: email.to_string(),
        email: email.to_string(),
        name: name.to_string(),
        exp,
        iat: now,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| format!("Failed to create token: {}", e))
}

fn decode_jwt_token(token: &str, secret: &str) -> Result<TokenData<Claims>, String> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| format!("Invalid token: {}", e))
}

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Failed to hash password: {}", e))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool, String> {
    use argon2::PasswordHash;
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| format!("Invalid hash: {}", e))?;
    match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn success_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Aud.io — Authenticated</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #0f1117;
      color: #e2e8f0;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
    }
    .card {
      text-align: center;
      padding: 48px 40px;
      background: #1a1d27;
      border-radius: 16px;
      border: 1px solid rgba(255,255,255,0.08);
      max-width: 380px;
      width: 90%;
    }
    .icon {
      width: 72px; height: 72px;
      background: linear-gradient(135deg, #00d4aa, #0099ff);
      border-radius: 50%;
      display: flex; align-items: center; justify-content: center;
      margin: 0 auto 24px;
      font-size: 36px;
    }
    h1 { font-size: 22px; font-weight: 600; color: #fff; margin-bottom: 12px; }
    p { font-size: 14px; color: #94a3b8; line-height: 1.5; }
    .brand { margin-top: 24px; font-size: 12px; color: #475569; }
  </style>
</head>
<body>
  <div class="card">
    <div class="icon">✓</div>
    <h1>Authentication Successful</h1>
    <p>You're signed in! You can close this tab and return to Aud.io.</p>
    <p class="brand">Aud.io · Offline Intelligence</p>
  </div>
  <script>
    // Try to auto-close after 2 s; may not work in all browsers for security reasons
    setTimeout(() => { try { window.close(); } catch(e) {} }, 2000);
  </script>
</body>
</html>"#
        .to_string()
}

fn error_html(msg: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Aud.io — Authentication Error</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #0f1117;
      color: #e2e8f0;
      display: flex;
      align-items: center;
      justify-content: center;
      min-height: 100vh;
    }}
    .card {{
      text-align: center;
      padding: 48px 40px;
      background: #1a1d27;
      border-radius: 16px;
      border: 1px solid rgba(255,255,255,0.08);
      max-width: 380px;
      width: 90%;
    }}
    .icon {{
      width: 72px; height: 72px;
      background: linear-gradient(135deg, #ef4444, #b91c1c);
      border-radius: 50%;
      display: flex; align-items: center; justify-content: center;
      margin: 0 auto 24px;
      font-size: 36px;
    }}
    h1 {{ font-size: 22px; font-weight: 600; color: #fff; margin-bottom: 12px; }}
    p {{ font-size: 14px; color: #94a3b8; line-height: 1.5; }}
  </style>
</head>
<body>
  <div class="card">
    <div class="icon">✗</div>
    <h1>Authentication Failed</h1>
    <p>{}</p>
  </div>
</body>
</html>"#,
        msg
    )
}

fn get_auth_state(state: &UnifiedAppState) -> &AuthState {
    state
        .auth_state
        .as_ref()
        .expect("Auth state not initialized")
        .as_ref()
}

pub async fn signup(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<SignupRequest>,
) -> impl IntoResponse {
    let auth_state = get_auth_state(&state);
    let name = payload.name.trim();
    let email = payload.email.trim().to_lowercase();
    let password = payload.password;

    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse {
                success: false,
                message: "Name is required".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    if email.is_empty() || !email.contains('@') {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse {
                success: false,
                message: "Valid email is required".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    if password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse {
                success: false,
                message: "Password must be at least 6 characters".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    if let Ok(true) = auth_state.users.email_exists(&email) {
        return (
            StatusCode::CONFLICT,
            Json(AuthResponse {
                success: false,
                message: "An account with this email already exists".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    let password_hash = match hash_password(&password) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Failed to create account".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            );
        }
    };

    match auth_state
        .users
        .create_user(&email, name, &password_hash)
    {
        Ok(user_id) => {
            info!("User created with id: {}", user_id);

            let jwt = match create_jwt_token(&email, name, &auth_state.jwt_secret) {
                Ok(t) => t,
                Err(e) => {
                    error!("Failed to create JWT after signup: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(AuthResponse {
                            success: false,
                            message: "Account created but could not sign in automatically. Please log in.".to_string(),
                            user: None,
                            token: None,
                            requires_verification: false,
                        }),
                    );
                }
            };

            let welcome_name = name.to_string();
            let welcome_email = email.clone();
            tokio::spawn(async move {
                if let Err(e) = send_welcome_email(&welcome_name, &welcome_email).await {
                    warn!("Failed to send welcome email: {}", e);
                }
            });

            (
                StatusCode::CREATED,
                Json(AuthResponse {
                    success: true,
                    message: "Account created! Welcome to _Aud.io Chat Interface.".to_string(),
                    user: Some(UserResponse {
                        id: user_id,
                        name: name.to_string(),
                        email: email.clone(),
                        email_verified: true,
                        avatar_url: None,
                    }),
                    token: Some(jwt),
                    requires_verification: false,
                }),
            )
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Failed to create account".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            )
        }
    }
}

pub async fn login(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let auth_state = get_auth_state(&state);
    let email = payload.email.trim().to_lowercase();
    let password = payload.password;

    if email.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse {
                success: false,
                message: "Email and password are required".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    let user = match auth_state.users.get_user_by_email(&email) {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthResponse {
                    success: false,
                    message: "Invalid email or password".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            )
        }
        Err(e) => {
            error!("Failed to get user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Login failed".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            );
        }
    };

    if user.password_hash == "google-oauth-user" {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse {
                success: false,
                message: "This account uses Google sign-in. Please use \"Sign in with Google\"."
                    .to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    let password_valid = match verify_password(&password, &user.password_hash) {
        Ok(valid) => valid,
        Err(e) => {
            error!("Failed to verify password: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Login failed".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            );
        }
    };

    if !password_valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse {
                success: false,
                message: "Invalid email or password".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    let token = match create_jwt_token(&email, &user.name, &auth_state.jwt_secret) {
        Ok(token) => token,
        Err(e) => {
            error!("Failed to create JWT token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Login failed".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            );
        }
    };

    info!("User logged in: {}", email);
    (
        StatusCode::OK,
        Json(AuthResponse {
            success: true,
            message: "Login successful".to_string(),
            user: Some(UserResponse {
                id: user.id,
                name: user.name,
                email: user.email,
                email_verified: user.email_verified,
                avatar_url: user.avatar_url,
            }),
            token: Some(token),
            requires_verification: false,
        }),
    )
}

pub async fn verify_email(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<VerifyEmailRequest>,
) -> impl IntoResponse {
    let auth_state = get_auth_state(&state);
    let token = payload.token.trim();

    if token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse {
                success: false,
                message: "Verification token is required".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    let user = match auth_state.users.verify_email(token) {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(AuthResponse {
                    success: false,
                    message: "Invalid or expired verification token".to_string(),
                    user: None,
                    token: None,
                    requires_verification: true,
                }),
            )
        }
        Err(e) => {
            error!("Failed to verify email: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Email verification failed".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            );
        }
    };

    let jwt = match create_jwt_token(&user.email, &user.name, &auth_state.jwt_secret) {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to create JWT token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Verification succeeded but login failed".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            );
        }
    };

    info!("Email verified for user: {}", user.email);
    (
        StatusCode::OK,
        Json(AuthResponse {
            success: true,
            message: "Email verified successfully!".to_string(),
            user: Some(UserResponse {
                id: user.id,
                name: user.name,
                email: user.email,
                email_verified: true,
                avatar_url: user.avatar_url,
            }),
            token: Some(jwt),
            requires_verification: false,
        }),
    )
}

pub async fn get_current_user(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<MeRequest>,
) -> impl IntoResponse {
    let auth_state = get_auth_state(&state);
    let token = payload.token;

    if token.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse {
                success: false,
                message: "No token provided".to_string(),
                user: None,
                token: None,
                requires_verification: false,
            }),
        );
    }

    let token_data = match decode_jwt_token(&token, &auth_state.jwt_secret) {
        Ok(data) => data,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthResponse {
                    success: false,
                    message: format!("Invalid token: {}", e),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            )
        }
    };

    let user = match auth_state
        .users
        .get_user_by_email(&token_data.claims.email)
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(AuthResponse {
                    success: false,
                    message: "User not found".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            )
        }
        Err(e) => {
            error!("Failed to get user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthResponse {
                    success: false,
                    message: "Failed to get user info".to_string(),
                    user: None,
                    token: None,
                    requires_verification: false,
                }),
            );
        }
    };

    (
        StatusCode::OK,
        Json(AuthResponse {
            success: true,
            message: "User found".to_string(),
            user: Some(UserResponse {
                id: user.id,
                name: user.name,
                email: user.email,
                email_verified: user.email_verified,
                avatar_url: user.avatar_url,
            }),
            token: Some(token),
            requires_verification: false,
        }),
    )
}

pub async fn google_init(
    State(state): State<UnifiedAppState>,
    Json(payload): Json<GoogleInitRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let auth_state = get_auth_state(&state);

    let google = match &auth_state.google {
        Some(g) => g,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "success": false,
                    "message": "Google OAuth not configured. Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables."
                })),
            )
        }
    };

    let state_param: String = hex::encode(rand::thread_rng().gen::<[u8; 16]>());
    let redirect_uri = format!(
        "http://127.0.0.1:{}/auth/google/callback",
        payload.port
    );

    {
        let mut states = google.states.lock().unwrap();
        states.insert(state_param.clone(), (redirect_uri.clone(), None));
    }

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}&access_type=online&prompt=select_account",
        url_encode(&google.client_id),
        url_encode(&redirect_uri),
        state_param,
    );

    info!(
        "Google OAuth init: state={}... port={}",
        &state_param[..8],
        payload.port
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "auth_url": auth_url,
            "state": state_param
        })),
    )
}

pub async fn google_callback(
    State(state): State<UnifiedAppState>,
    Query(params): Query<GoogleCallbackQuery>,
) -> Response {
    let auth_state = get_auth_state(&state);

    let google = match &auth_state.google {
        Some(g) => g,
        None => return Html(error_html("OAuth not configured on this server.")).into_response(),
    };

    if let Some(err) = params.error {
        if let Some(ref s) = params.state {
            google.states.lock().unwrap().remove(s);
        }
        return Html(error_html(&format!("Google sign-in was cancelled: {}", err))).into_response();
    }

    let code = match params.code {
        Some(c) if !c.is_empty() => c,
        _ => {
            return Html(error_html(
                "Missing authorization code. Please try signing in again.",
            ))
            .into_response()
        }
    };

    let state_param = match params.state {
        Some(s) if !s.is_empty() => s,
        _ => return Html(error_html("Missing state parameter.")).into_response(),
    };

    let redirect_uri = {
        let states = google.states.lock().unwrap();
        match states.get(&state_param) {
            Some((uri, _)) => uri.clone(),
            None => {
                return Html(error_html(
                    "Invalid or expired sign-in session. Please try again.",
                ))
                .into_response()
            }
        }
    };

    let http = reqwest::Client::new();
    let token_resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code.as_str()),
            ("client_id", &google.client_id),
            ("client_secret", &google.client_secret),
            ("redirect_uri", &redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await;

    let access_token = match token_resp {
        Ok(resp) => match resp.json::<GoogleTokenResponse>().await {
            Ok(t) => t.access_token,
            Err(e) => {
                error!("Failed to parse Google token response: {}", e);
                return Html(error_html(
                    "Failed to exchange authorization code with Google.",
                ))
                .into_response();
            }
        },
        Err(e) => {
            error!("Google token exchange failed: {}", e);
            return Html(error_html(
                "Could not contact Google servers. Please check your internet connection.",
            ))
            .into_response();
        }
    };

    let user_info_resp = http
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(&access_token)
        .send()
        .await;

    let google_user = match user_info_resp {
        Ok(resp) => match resp.json::<GoogleUserInfo>().await {
            Ok(u) => u,
            Err(e) => {
                error!("Failed to parse Google user info: {}", e);
                return Html(error_html("Failed to retrieve your Google account info."))
                    .into_response();
            }
        },
        Err(e) => {
            error!("Google userinfo request failed: {}", e);
            return Html(error_html("Could not retrieve Google profile info.")).into_response();
        }
    };

    let (db_user, is_new_user) = match auth_state.users.upsert_google_user(
        &google_user.email,
        &google_user.name,
        &google_user.id,
        google_user.picture.as_deref(),
    ) {
        Ok(pair) => pair,
        Err(e) => {
            error!("Failed to upsert Google user: {}", e);
            return Html(error_html(
                "Failed to create or update your account. Please try again.",
            ))
            .into_response();
        }
    };

    if is_new_user {
        
        let name_notif = google_user.name.clone();
        let email_notif = google_user.email.clone();
        tokio::spawn(async move {
            if let Err(e) = send_new_user_notification(&name_notif, &email_notif).await {
                warn!("Failed to send new-user notification: {}", e);
            }
        });

        let name_welcome = google_user.name.clone();
        let email_welcome = google_user.email.clone();
        tokio::spawn(async move {
            if let Err(e) = send_welcome_email(&name_welcome, &email_welcome).await {
                warn!("Failed to send welcome email to new Google user: {}", e);
            }
        });
    }

    let jwt = match create_jwt_token(&db_user.email, &db_user.name, &auth_state.jwt_secret) {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to create JWT for Google user: {}", e);
            return Html(error_html("Failed to generate authentication token.")).into_response();
        }
    };

    let user_response = UserResponse {
        id: db_user.id,
        name: db_user.name.clone(),
        email: db_user.email.clone(),
        email_verified: true,
        avatar_url: db_user.avatar_url.clone(),
    };

    {
        let mut states = google.states.lock().unwrap();
        if let Some(entry) = states.get_mut(&state_param) {
            entry.1 = Some(GoogleOAuthResult {
                token: jwt,
                user: user_response,
            });
        }
    }

    {
        let name  = google_user.name.clone();
        let email = google_user.email.clone();
        tokio::spawn(async move {
            if let Err(e) = send_login_confirmation_email(&name, &email).await {
                warn!("Failed to send login confirmation email: {}", e);
            }
        });
    }

    info!("Google OAuth complete for: {}", db_user.email);
    Html(success_html()).into_response()
}

pub async fn google_status(
    State(state): State<UnifiedAppState>,
    Query(params): Query<GoogleStatusQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    let auth_state = get_auth_state(&state);

    let google = match &auth_state.google {
        Some(g) => g,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "pending": false,
                    "success": false,
                    "message": "Google OAuth not configured"
                })),
            )
        }
    };

    let mut states = google.states.lock().unwrap();

    match states.get(&params.state) {
        
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "pending": false,
                "success": false,
                "message": "Unknown or expired auth session"
            })),
        ),

        Some((_, None)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "pending": true })),
        ),

        Some((_, Some(_))) => {
            let result = states.remove(&params.state).unwrap().1.unwrap();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "pending": false,
                    "success": true,
                    "message": "Authentication successful",
                    "token": result.token,
                    "user": {
                        "id": result.user.id,
                        "name": result.user.name,
                        "email": result.user.email,
                        "email_verified": result.user.email_verified,
                        "avatar_url": result.user.avatar_url,
                    }
                })),
            )
        }
    }
}

async fn send_new_user_notification(name: &str, email: &str) -> Result<(), String> {
    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials, Message,
        SmtpTransport, Transport,
    };

    let smtp_user = std::env::var("SMTP_USER").unwrap_or_default();
    let smtp_pass = std::env::var("SMTP_PASS").unwrap_or_default();

    if smtp_user.is_empty() || smtp_pass.is_empty() {
        return Err("SMTP credentials not configured — skipping new-user notification".into());
    }

    let smtp_host =
        std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.hostinger.com".to_string());
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".to_string())
        .parse()
        .unwrap_or(587);

    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let msg = Message::builder()
        .from(
            format!("Aud.io <{}>", smtp_user)
                .parse()
                .map_err(|e: lettre::address::AddressError| e.to_string())?,
        )
        .to("Product Team <product@offlineintelligence.io>"
            .parse()
            .map_err(|e: lettre::address::AddressError| e.to_string())?)
        .subject(format!("New User - {}", name))
        .header(ContentType::TEXT_HTML)
        .body(format!(
            r#"<!DOCTYPE html>
<html>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); padding: 24px 30px; border-radius: 10px 10px 0 0;">
        <h1 style="color: white; margin: 0; font-size: 22px;">New User Registered</h1>
    </div>
    <div style="background: #f9f9f9; padding: 30px; border-radius: 0 0 10px 10px; border: 1px solid #e0e0e0;">
        <table style="width: 100%; border-collapse: collapse;">
            <tr><td style="padding: 8px 0; color: #666; width: 100px;">Name</td><td style="padding: 8px 0; font-weight: 600;">{}</td></tr>
            <tr><td style="padding: 8px 0; color: #666;">Email</td><td style="padding: 8px 0; font-weight: 600;">{}</td></tr>
            <tr><td style="padding: 8px 0; color: #666;">Sign-in</td><td style="padding: 8px 0;">Google OAuth</td></tr>
            <tr><td style="padding: 8px 0; color: #666;">Time</td><td style="padding: 8px 0;">{}</td></tr>
        </table>
    </div>
</body>
</html>"#,
            name, email, now
        ))
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(smtp_user, smtp_pass);
    let mailer = SmtpTransport::starttls_relay(&smtp_host)
        .map_err(|e| format!("SMTP relay error: {}", e))?
        .port(smtp_port)
        .credentials(creds)
        .build();

    mailer.send(&msg).map_err(|e| format!("Failed to send: {}", e))?;
    info!("New-user notification sent for: {}", email);
    Ok(())
}

async fn send_login_confirmation_email(name: &str, user_email: &str) -> Result<(), String> {
    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials, Message,
        SmtpTransport, Transport,
    };

    let smtp_user = std::env::var("SMTP_USER").unwrap_or_default();
    let smtp_pass = std::env::var("SMTP_PASS").unwrap_or_default();

    if smtp_user.is_empty() || smtp_pass.is_empty() {
        return Err("SMTP credentials not configured — skipping login confirmation".into());
    }

    let smtp_host =
        std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.hostinger.com".to_string());
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".to_string())
        .parse()
        .unwrap_or(587);

    let first_name = name.split_whitespace().next().unwrap_or(name);
    let now = chrono::Utc::now().format("%d %B %Y at %H:%M UTC").to_string();

    let html_body = format!(
        r#"<!DOCTYPE html>
<html>
<body style="margin:0;padding:0;background:#0d0d0d;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
<div style="max-width:480px;margin:40px auto;background:#1a1a1a;border-radius:16px;overflow:hidden;border:1px solid #2a2a2a;">

  <!-- Header -->
  <div style="background:linear-gradient(135deg,#00c9a7 0%,#0099ff 100%);padding:28px 32px 22px;text-align:center;">
    <div style="font-size:26px;font-weight:800;color:#fff;letter-spacing:-0.5px;">Aud.io</div>
    <div style="font-size:12px;color:rgba(255,255,255,0.75);margin-top:4px;">Offline-first AI Assistant</div>
  </div>

  <!-- Body -->
  <div style="padding:32px;">
    <h2 style="margin:0 0 8px;font-size:20px;font-weight:700;color:#f0f0f0;">You're signed in, {}! 👋</h2>
    <p style="color:#a0a0a0;font-size:14px;line-height:1.6;margin:0 0 24px;">
      Your Google account was used to sign in to Aud.io. If this was you, no action is needed.
    </p>

    <!-- Details card -->
    <div style="background:#242424;border-radius:10px;padding:16px 20px;border-left:3px solid #00c9a7;margin-bottom:24px;">
      <table style="width:100%;border-collapse:collapse;">
        <tr>
          <td style="padding:5px 0;color:#707070;font-size:12px;width:80px;">Account</td>
          <td style="padding:5px 0;color:#e0e0e0;font-size:13px;font-weight:500;">{}</td>
        </tr>
        <tr>
          <td style="padding:5px 0;color:#707070;font-size:12px;">Method</td>
          <td style="padding:5px 0;color:#e0e0e0;font-size:13px;">Google Sign-In</td>
        </tr>
        <tr>
          <td style="padding:5px 0;color:#707070;font-size:12px;">Time</td>
          <td style="padding:5px 0;color:#e0e0e0;font-size:13px;">{}</td>
        </tr>
      </table>
    </div>

    <p style="color:#606060;font-size:12px;line-height:1.6;margin:0;">
      If you did not sign in to Aud.io, you can safely ignore this email.
      No one can access your account without your Google credentials.
    </p>
  </div>

  <!-- Footer -->
  <div style="padding:14px 32px;border-top:1px solid #242424;text-align:center;">
    <span style="font-size:11px;color:#404040;">© 2025 Aud.io · Offline Intelligence</span>
  </div>

</div>
</body>
</html>"#,
        first_name, user_email, now
    );

    let msg = Message::builder()
        .from(
            format!("Aud.io <{}>", smtp_user)
                .parse()
                .map_err(|e: lettre::address::AddressError| e.to_string())?,
        )
        .to(format!("{} <{}>", name, user_email)
            .parse()
            .map_err(|e: lettre::address::AddressError| e.to_string())?)
        .subject("You're signed in to Aud.io")
        .header(ContentType::TEXT_HTML)
        .body(html_body)
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(smtp_user, smtp_pass);
    let mailer = SmtpTransport::starttls_relay(&smtp_host)
        .map_err(|e| format!("SMTP relay error: {}", e))?
        .port(smtp_port)
        .credentials(creds)
        .build();

    mailer.send(&msg).map_err(|e| format!("Failed to send login confirmation: {}", e))?;
    info!("Login confirmation email sent to: {}", user_email);
    Ok(())
}

async fn send_welcome_email(name: &str, user_email: &str) -> Result<(), String> {
    use lettre::{
        message::header::ContentType, transport::smtp::authentication::Credentials, Message,
        SmtpTransport, Transport,
    };

    let smtp_user = std::env::var("SMTP_USER").unwrap_or_default();
    let smtp_pass = std::env::var("SMTP_PASS").unwrap_or_default();

    if smtp_user.is_empty() || smtp_pass.is_empty() {
        return Err("SMTP credentials not configured — skipping welcome email".into());
    }

    let smtp_host =
        std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.hostinger.com".to_string());
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".to_string())
        .parse()
        .unwrap_or(587);

    let first_name = name.split_whitespace().next().unwrap_or(name);

    let html_body = format!(
        r#"<!DOCTYPE html>
<html>
<body style="margin:0;padding:0;background:#0d0d0d;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;">
<div style="max-width:520px;margin:40px auto;background:#1a1a1a;border-radius:16px;overflow:hidden;border:1px solid #2a2a2a;">

  <!-- Header -->
  <div style="background:linear-gradient(135deg,#00c9a7 0%,#0099ff 100%);padding:32px 36px 26px;text-align:center;">
    <div style="font-size:28px;font-weight:800;color:#fff;letter-spacing:-0.5px;">_Aud.io</div>
    <div style="font-size:13px;color:rgba(255,255,255,0.8);margin-top:5px;">Chat Interface</div>
  </div>

  <!-- Body -->
  <div style="padding:36px;">
    <h2 style="margin:0 0 10px;font-size:22px;font-weight:700;color:#f0f0f0;">Welcome, {}! 🎉</h2>
    <p style="color:#a0a0a0;font-size:15px;line-height:1.7;margin:0 0 20px;">
      Thank you for downloading <strong style="color:#e0e0e0;">_Aud.io Chat Interface</strong> —
      your privacy-first, offline AI assistant. We're thrilled to have you on board.
    </p>

    <p style="color:#a0a0a0;font-size:15px;line-height:1.7;margin:0 0 28px;">
      Everything runs locally on your device, so your conversations stay completely private.
      No data leaves your machine unless you explicitly use an online model.
    </p>

    <!-- Feedback callout -->
    <div style="background:#242424;border-radius:12px;padding:20px 24px;border-left:3px solid #00c9a7;margin-bottom:28px;">
      <p style="margin:0 0 8px;font-size:14px;font-weight:600;color:#e0e0e0;">Share your feedback 💬</p>
      <p style="margin:0;font-size:13px;color:#808080;line-height:1.6;">
        We read every message. If you have a suggestion, a feature request, or ran into
        something unexpected, simply reply to this email — we'd love to hear from you.
      </p>
    </div>

    <!-- CTA button -->
    <div style="text-align:center;margin-bottom:28px;">
      <a href="https://www.offlineintelligence.io"
         style="display:inline-block;padding:13px 32px;background:linear-gradient(135deg,#00c9a7,#0099ff);color:#fff;
                text-decoration:none;border-radius:50px;font-weight:600;font-size:14px;letter-spacing:0.3px;">
        Explore Our Products
      </a>
    </div>

    <p style="color:#606060;font-size:13px;line-height:1.6;margin:0;">
      You're receiving this because you just created an account on _Aud.io Chat Interface.
    </p>
  </div>

  <!-- Footer -->
  <div style="padding:16px 36px;border-top:1px solid #242424;text-align:center;">
    <span style="font-size:11px;color:#404040;">
      © 2025 Offline Intelligence ·
      <a href="https://www.offlineintelligence.io" style="color:#505050;text-decoration:none;">offlineintelligence.io</a>
    </span>
  </div>

</div>
</body>
</html>"#,
        first_name
    );

    let msg = Message::builder()
        .from(
            format!("_Aud.io <{}>", smtp_user)
                .parse()
                .map_err(|e: lettre::address::AddressError| e.to_string())?,
        )
        .to(format!("{} <{}>", name, user_email)
            .parse()
            .map_err(|e: lettre::address::AddressError| e.to_string())?)
        .subject("Welcome to _Aud.io Chat Interface!")
        .header(ContentType::TEXT_HTML)
        .body(html_body)
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(smtp_user, smtp_pass);
    let mailer = SmtpTransport::starttls_relay(&smtp_host)
        .map_err(|e| format!("SMTP relay error: {}", e))?
        .port(smtp_port)
        .credentials(creds)
        .build();

    mailer.send(&msg).map_err(|e| format!("Failed to send welcome email: {}", e))?;
    info!("Welcome email sent to: {}", user_email);
    Ok(())
}
