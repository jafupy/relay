use crate::state::RelayState;
use anyhow::{Result, anyhow};
use argon2::{
   Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
   password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
   Json,
   extract::{Request, State},
   http::{HeaderMap, StatusCode, header},
   middleware::Next,
   response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
   path::PathBuf,
   sync::{Arc, Mutex},
   time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const SESSION_COOKIE: &str = "relay_session";
const SESSION_TTL_SECONDS: u64 = 60 * 60 * 24 * 30;

#[derive(Clone)]
pub struct AuthStore {
   dir: PathBuf,
   inner: Arc<Mutex<AuthData>>,
}

#[derive(Default, Serialize, Deserialize)]
struct AuthData {
   users: Vec<StoredUser>,
   sessions: Vec<StoredSession>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredUser {
   pub id: String,
   pub username: String,
   pub display_name: Option<String>,
   pub role: UserRole,
   pub enabled: bool,
   pub password_hash: Option<String>,
   pub password_updated_at: Option<u64>,
   pub force_password_change: bool,
   pub created_at: u64,
   pub updated_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
   Admin,
   User,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSession {
   id_hash: String,
   user_id: String,
   created_at: u64,
   last_seen_at: u64,
   expires_at: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
   username: String,
   password: Option<String>,
   display_name: Option<String>,
   role: Option<UserRole>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
   username: String,
   password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
   current_password: Option<String>,
   new_password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
   user: PublicUser,
   force_password_change: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
   pub id: String,
   pub username: String,
   pub display_name: Option<String>,
   pub role: UserRole,
   pub force_password_change: bool,
}

impl From<&StoredUser> for PublicUser {
   fn from(user: &StoredUser) -> Self {
      Self {
         id: user.id.clone(),
         username: user.username.clone(),
         display_name: user.display_name.clone(),
         role: user.role.clone(),
         force_password_change: user.force_password_change,
      }
   }
}

impl AuthStore {
   pub async fn initialize(dir: PathBuf) -> Result<Self> {
      tokio::fs::create_dir_all(&dir).await?;
      let data = AuthData {
         users: read_json(dir.join("users.json")).await.unwrap_or_default(),
         sessions: read_json(dir.join("sessions.json"))
            .await
            .unwrap_or_default(),
      };
      let store = Self {
         dir,
         inner: Arc::new(Mutex::new(data)),
      };

      store.ensure_initial_admin().await?;
      Ok(store)
   }

   async fn ensure_initial_admin(&self) -> Result<()> {
      let should_create = self.inner.lock().unwrap().users.is_empty();
      if !should_create {
         return Ok(());
      }

      let password = generate_initial_password();
      let password_hash = hash_password(&password)?;
      let now = now();
      let user = StoredUser {
         id: Uuid::new_v4().to_string(),
         username: "admin".to_string(),
         display_name: Some("Admin".to_string()),
         role: UserRole::Admin,
         enabled: true,
         password_hash: Some(password_hash),
         password_updated_at: Some(now),
         force_password_change: true,
         created_at: now,
         updated_at: now,
      };

      {
         let mut data = self.inner.lock().unwrap();
         data.users.push(user);
      }
      self.save_users().await?;

      println!();
      println!("Relay created the initial admin user.");
      println!();
      println!("Username: admin");
      println!("Password: {}", password);
      println!();
      println!("Store this now. It will not be shown again.");
      println!();

      Ok(())
   }

   pub async fn login(&self, username: &str, password: &str) -> Result<(PublicUser, String)> {
      let user = {
         let data = self.inner.lock().unwrap();
         data
            .users
            .iter()
            .find(|user| user.username == username && user.enabled)
            .cloned()
      }
      .ok_or_else(|| anyhow!("invalid username or password"))?;

      let password_hash = user
         .password_hash
         .as_deref()
         .ok_or_else(|| anyhow!("password login is not enabled for this user"))?;
      verify_password(password_hash, password)?;

      let session_token = generate_session_token();
      self
         .store_session(user.id.clone(), session_token.clone())
         .await?;

      Ok((PublicUser::from(&user), session_token))
   }

   pub async fn create_session_for_user_id(&self, user_id: &str) -> Result<(PublicUser, String)> {
      let user = self
         .stored_user_by_id(user_id)
         .await
         .ok_or_else(|| anyhow!("user not found"))?;
      let session_token = generate_session_token();
      self
         .store_session(user.id.clone(), session_token.clone())
         .await?;
      Ok((PublicUser::from(&user), session_token))
   }

   async fn store_session(&self, user_id: String, session_token: String) -> Result<()> {
      let session = StoredSession {
         id_hash: hash_session_token(&session_token),
         user_id,
         created_at: now(),
         last_seen_at: now(),
         expires_at: now() + SESSION_TTL_SECONDS,
      };

      {
         let mut data = self.inner.lock().unwrap();
         data.sessions.push(session);
      }
      self.save_sessions().await
   }

   pub async fn user_for_session(&self, token: &str) -> Option<PublicUser> {
      let token_hash = hash_session_token(token);
      let user = {
         let mut data = self.inner.lock().unwrap();
         let session = data
            .sessions
            .iter_mut()
            .find(|session| session.id_hash == token_hash && session.expires_at > now())?;
         session.last_seen_at = now();
         let user_id = session.user_id.clone();
         data
            .users
            .iter()
            .find(|user| user.id == user_id && user.enabled)
            .cloned()
      };
      if user.is_some() {
         let _ = self.save_sessions().await;
      }
      user.as_ref().map(PublicUser::from)
   }

   pub async fn logout(&self, token: &str) -> Result<()> {
      let token_hash = hash_session_token(token);
      {
         let mut data = self.inner.lock().unwrap();
         data
            .sessions
            .retain(|session| session.id_hash != token_hash);
      }
      self.save_sessions().await
   }

   pub async fn stored_user_by_id(&self, user_id: &str) -> Option<StoredUser> {
      self
         .inner
         .lock()
         .unwrap()
         .users
         .iter()
         .find(|user| user.id == user_id && user.enabled)
         .cloned()
   }

   pub async fn list_users(&self) -> Vec<PublicUser> {
      self
         .inner
         .lock()
         .unwrap()
         .users
         .iter()
         .filter(|user| user.enabled)
         .map(PublicUser::from)
         .collect()
   }

   pub async fn create_user(
      &self,
      username: String,
      display_name: Option<String>,
      password: Option<String>,
      role: UserRole,
   ) -> Result<PublicUser> {
      let username = username.trim().to_string();
      if username.is_empty() {
         return Err(anyhow!("username is required"));
      }

      let public_user = {
         let mut data = self.inner.lock().unwrap();
         if data.users.iter().any(|user| user.username == username) {
            return Err(anyhow!("username already exists"));
         }

         let now = now();
         let password_hash = password.as_deref().map(hash_password).transpose()?;
         let user = StoredUser {
            id: Uuid::new_v4().to_string(),
            username,
            display_name,
            role,
            enabled: true,
            password_hash,
            password_updated_at: password.as_ref().map(|_| now),
            force_password_change: password.is_some(),
            created_at: now,
            updated_at: now,
         };
         let public_user = PublicUser::from(&user);
         data.users.push(user);
         public_user
      };
      self.save_users().await?;
      Ok(public_user)
   }

   pub async fn change_password(
      &self,
      user_id: &str,
      current_password: Option<&str>,
      new_password: &str,
   ) -> Result<PublicUser> {
      if new_password.len() < 12 {
         return Err(anyhow!("new password must be at least 12 characters"));
      }

      let public_user = {
         let mut data = self.inner.lock().unwrap();
         let user = data
            .users
            .iter_mut()
            .find(|user| user.id == user_id && user.enabled)
            .ok_or_else(|| anyhow!("user not found"))?;

         if !user.force_password_change {
            let password_hash = user
               .password_hash
               .as_deref()
               .ok_or_else(|| anyhow!("password login is not enabled for this user"))?;
            verify_password(password_hash, current_password.unwrap_or_default())?;
         }

         let now = now();
         user.password_hash = Some(hash_password(new_password)?);
         user.password_updated_at = Some(now);
         user.force_password_change = false;
         user.updated_at = now;
         PublicUser::from(&*user)
      };
      self.save_users().await?;
      Ok(public_user)
   }

   async fn save_users(&self) -> Result<()> {
      let users = self.inner.lock().unwrap().users.clone();
      write_json(self.dir.join("users.json"), &users).await
   }

   async fn save_sessions(&self) -> Result<()> {
      let sessions = self.inner.lock().unwrap().sessions.clone();
      write_json(self.dir.join("sessions.json"), &sessions).await
   }
}

pub async fn login(
   State(state): State<RelayState>,
   Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
   match state.auth.login(&payload.username, &payload.password).await {
      Ok((user, session_token)) => (
         StatusCode::OK,
         [(header::SET_COOKIE, session_cookie(&session_token))],
         Json(LoginResponse {
            force_password_change: user.force_password_change,
            user,
         }),
      )
         .into_response(),
      Err(_) => (
         StatusCode::UNAUTHORIZED,
         Json(serde_json::json!({ "error": "invalid login" })),
      )
         .into_response(),
   }
}

pub async fn me(State(state): State<RelayState>, request: Request) -> impl IntoResponse {
   match session_token_from_request(&request) {
      Some(token) => match state.auth.user_for_session(&token).await {
         Some(user) => (StatusCode::OK, Json(serde_json::json!({ "user": user }))).into_response(),
         None => StatusCode::UNAUTHORIZED.into_response(),
      },
      None => StatusCode::UNAUTHORIZED.into_response(),
   }
}

pub async fn logout(State(state): State<RelayState>, request: Request) -> impl IntoResponse {
   if let Some(token) = session_token_from_request(&request) {
      let _ = state.auth.logout(&token).await;
   }
   (
      StatusCode::NO_CONTENT,
      [(
         header::SET_COOKIE,
         format!(
            "{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0",
            SESSION_COOKIE
         ),
      )],
   )
}

pub async fn change_password(
   State(state): State<RelayState>,
   headers: HeaderMap,
   Json(payload): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
   let user = match user_from_headers(&state, &headers).await {
      Some(user) => user,
      None => return StatusCode::UNAUTHORIZED.into_response(),
   };

   match state
      .auth
      .change_password(
         &user.id,
         payload.current_password.as_deref(),
         &payload.new_password,
      )
      .await
   {
      Ok(user) => (StatusCode::OK, Json(serde_json::json!({ "user": user }))).into_response(),
      Err(error) => (
         StatusCode::BAD_REQUEST,
         Json(serde_json::json!({ "error": error.to_string() })),
      )
         .into_response(),
   }
}

pub async fn list_users(State(state): State<RelayState>, headers: HeaderMap) -> impl IntoResponse {
   let user = match user_from_headers(&state, &headers).await {
      Some(user) => user,
      None => return StatusCode::UNAUTHORIZED.into_response(),
   };
   if !matches!(user.role, UserRole::Admin) {
      return StatusCode::FORBIDDEN.into_response();
   }
   Json(serde_json::json!({ "users": state.auth.list_users().await })).into_response()
}

pub async fn create_user(
   State(state): State<RelayState>,
   headers: HeaderMap,
   Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
   let user = match user_from_headers(&state, &headers).await {
      Some(user) => user,
      None => return StatusCode::UNAUTHORIZED.into_response(),
   };
   if !matches!(user.role, UserRole::Admin) {
      return StatusCode::FORBIDDEN.into_response();
   }

   match state
      .auth
      .create_user(
         payload.username,
         payload.display_name,
         payload.password,
         payload.role.unwrap_or(UserRole::User),
      )
      .await
   {
      Ok(user) => (
         StatusCode::CREATED,
         Json(serde_json::json!({ "user": user })),
      )
         .into_response(),
      Err(error) => (
         StatusCode::BAD_REQUEST,
         Json(serde_json::json!({ "error": error.to_string() })),
      )
         .into_response(),
   }
}

pub async fn require_auth(
   State(state): State<RelayState>,
   request: Request,
   next: Next,
) -> Response {
   let user = match session_token_from_request(&request) {
      Some(token) => state.auth.user_for_session(&token).await,
      None => None,
   };

   let Some(user) = user else {
      return StatusCode::UNAUTHORIZED.into_response();
   };

   if user.force_password_change && !is_password_bootstrap_route(&request) {
      return (
         StatusCode::FORBIDDEN,
         Json(serde_json::json!({ "error": "password change required" })),
      )
         .into_response();
   }

   next.run(request).await
}

pub async fn user_from_headers(state: &RelayState, headers: &HeaderMap) -> Option<PublicUser> {
   let token = session_token_from_headers(headers)?;
   state.auth.user_for_session(&token).await
}

pub fn session_token_from_request(request: &Request) -> Option<String> {
   session_token_from_headers(request.headers())
}

pub fn session_token_from_headers(headers: &HeaderMap) -> Option<String> {
   headers
      .get(header::COOKIE)
      .and_then(|value| value.to_str().ok())
      .and_then(|cookies| {
         cookies.split(';').find_map(|part| {
            let (name, value) = part.trim().split_once('=')?;
            (name == SESSION_COOKIE).then(|| value.to_string())
         })
      })
}

pub fn session_cookie(session_token: &str) -> String {
   let secure = public_origin_is_https();
   let mut cookie = format!(
      "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
      SESSION_COOKIE, session_token, SESSION_TTL_SECONDS
   );
   if secure {
      cookie.push_str("; Secure");
   }
   cookie
}

fn is_password_bootstrap_route(request: &Request) -> bool {
   matches!(
      request.uri().path(),
      "/api/auth/password"
         | "/api/auth/passkeys/register/start"
         | "/api/auth/passkeys/register/finish"
   )
}

fn hash_password(password: &str) -> Result<String> {
   let salt = SaltString::generate(&mut OsRng);
   Ok(Argon2::default()
      .hash_password(password.as_bytes(), &salt)
      .map_err(|err| anyhow!("failed to hash password: {}", err))?
      .to_string())
}

fn verify_password(password_hash: &str, password: &str) -> Result<()> {
   let parsed_hash =
      PasswordHash::new(password_hash).map_err(|err| anyhow!("invalid password hash: {}", err))?;
   Argon2::default()
      .verify_password(password.as_bytes(), &parsed_hash)
      .map_err(|_| anyhow!("invalid username or password"))
}

fn generate_session_token() -> String {
   let mut bytes = [0u8; 32];
   rand::thread_rng().fill_bytes(&mut bytes);
   URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_session_token(token: &str) -> String {
   let digest = Sha256::digest(token.as_bytes());
   URL_SAFE_NO_PAD.encode(digest)
}

fn generate_initial_password() -> String {
   const WORDS: &[&str] = &[
      "Purple", "Octopus", "Elephant", "Copper", "Falcon", "River", "Maple", "Quartz", "Harbor",
      "Nimbus", "Pioneer", "Velvet", "Summit", "Anchor", "Lantern", "Cedar",
   ];
   let mut rng = rand::thread_rng();
   format!(
      "{}.{}.{}.{:04}",
      WORDS[rng.gen_range(0..WORDS.len())],
      WORDS[rng.gen_range(0..WORDS.len())],
      WORDS[rng.gen_range(0..WORDS.len())],
      rng.gen_range(0..10_000)
   )
}

fn now() -> u64 {
   SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or(Duration::ZERO)
      .as_secs()
}

fn public_origin_is_https() -> bool {
   std::env::var("RELAY_PUBLIC_ORIGIN")
      .map(|origin| origin.starts_with("https://"))
      .unwrap_or(false)
}

async fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
   let bytes = tokio::fs::read(path).await?;
   Ok(serde_json::from_slice(&bytes)?)
}

async fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<()> {
   if let Some(parent) = path.parent() {
      tokio::fs::create_dir_all(parent).await?;
   }
   let bytes = serde_json::to_vec_pretty(value)?;
   tokio::fs::write(path, bytes).await?;
   Ok(())
}
