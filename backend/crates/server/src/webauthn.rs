use crate::{
   auth::{self, PublicUser},
   state::RelayState,
};
use anyhow::{Result, anyhow};
use axum::{
   Json,
   extract::{Path, State},
   http::{HeaderMap, StatusCode, header},
   response::IntoResponse,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use std::{
   collections::HashMap,
   path::PathBuf,
   sync::{Arc, Mutex},
   time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use webauthn_rs::prelude::{
   CreationChallengeResponse, CredentialID, Passkey, PasskeyAuthentication, PasskeyRegistration,
   PublicKeyCredential, RegisterPublicKeyCredential, RequestChallengeResponse, Url, Webauthn,
   WebauthnBuilder,
};

#[derive(Clone)]
pub struct PasskeyStore {
   file: PathBuf,
   webauthn: Arc<Webauthn>,
   inner: Arc<Mutex<PasskeyData>>,
   pending_registrations: Arc<Mutex<HashMap<String, PendingRegistration>>>,
   pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PasskeyData {
   passkeys: Vec<StoredPasskey>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredPasskey {
   credential_id: String,
   user_id: String,
   name: String,
   passkey: Passkey,
   created_at: u64,
   last_used_at: Option<u64>,
}

struct PendingRegistration {
   user_id: String,
   name: String,
   state: PasskeyRegistration,
}

struct PendingLogin {
   user_id: String,
   state: PasskeyAuthentication,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterStartRequest {
   name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterFinishRequest {
   challenge_id: String,
   credential: RegisterPublicKeyCredential,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStartRequest {
   username: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginFinishRequest {
   challenge_id: String,
   credential: PublicKeyCredential,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterStartResponse {
   challenge_id: String,
   #[serde(flatten)]
   challenge: CreationChallengeResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginStartResponse {
   challenge_id: String,
   #[serde(flatten)]
   challenge: RequestChallengeResponse,
}

impl PasskeyStore {
   pub async fn initialize(file: PathBuf) -> Result<Self> {
      let webauthn = Arc::new(build_webauthn()?);
      let data = read_json(&file).await.unwrap_or_default();
      Ok(Self {
         file,
         webauthn,
         inner: Arc::new(Mutex::new(data)),
         pending_registrations: Arc::new(Mutex::new(HashMap::new())),
         pending_logins: Arc::new(Mutex::new(HashMap::new())),
      })
   }

   async fn start_registration(
      &self,
      user: &PublicUser,
      name: Option<String>,
   ) -> Result<RegisterStartResponse> {
      let user_uuid = Uuid::parse_str(&user.id)?;
      let credentials = self.user_credential_ids(&user.id);
      let display_name = user
         .display_name
         .as_deref()
         .filter(|value| !value.is_empty())
         .unwrap_or(&user.username);
      let (challenge, state) = self.webauthn.start_passkey_registration(
         user_uuid,
         &user.username,
         display_name,
         Some(credentials),
      )?;
      let challenge_id = Uuid::new_v4().to_string();
      self.pending_registrations.lock().unwrap().insert(
         challenge_id.clone(),
         PendingRegistration {
            user_id: user.id.clone(),
            name: name
               .filter(|value| !value.trim().is_empty())
               .unwrap_or_else(|| "Passkey".to_string()),
            state,
         },
      );
      Ok(RegisterStartResponse {
         challenge_id,
         challenge,
      })
   }

   async fn finish_registration(
      &self,
      user: &PublicUser,
      challenge_id: &str,
      credential: &RegisterPublicKeyCredential,
   ) -> Result<StoredPasskey> {
      let pending = self
         .pending_registrations
         .lock()
         .unwrap()
         .remove(challenge_id)
         .ok_or_else(|| anyhow!("registration challenge was not found"))?;
      if pending.user_id != user.id {
         return Err(anyhow!(
            "registration challenge does not belong to this user"
         ));
      }

      let passkey = self
         .webauthn
         .finish_passkey_registration(credential, &pending.state)?;
      let credential_id = credential_id(passkey.cred_id());
      let stored = StoredPasskey {
         credential_id: credential_id.clone(),
         user_id: user.id.clone(),
         name: pending.name,
         passkey,
         created_at: now(),
         last_used_at: None,
      };

      {
         let mut data = self.inner.lock().unwrap();
         if data
            .passkeys
            .iter()
            .any(|passkey| passkey.credential_id == credential_id)
         {
            return Err(anyhow!("passkey is already registered"));
         }
         data.passkeys.push(stored.clone());
      }
      self.save().await?;
      Ok(stored)
   }

   async fn start_login(&self, user: &PublicUser) -> Result<LoginStartResponse> {
      let credentials = self.user_passkeys(&user.id);
      if credentials.is_empty() {
         return Err(anyhow!("no passkeys are registered for this user"));
      }
      let (challenge, state) = self.webauthn.start_passkey_authentication(&credentials)?;
      let challenge_id = Uuid::new_v4().to_string();
      self.pending_logins.lock().unwrap().insert(
         challenge_id.clone(),
         PendingLogin {
            user_id: user.id.clone(),
            state,
         },
      );
      Ok(LoginStartResponse {
         challenge_id,
         challenge,
      })
   }

   pub async fn finish_login(
      &self,
      challenge_id: &str,
      credential: &PublicKeyCredential,
   ) -> Result<String> {
      let pending = self
         .pending_logins
         .lock()
         .unwrap()
         .remove(challenge_id)
         .ok_or_else(|| anyhow!("login challenge was not found"))?;
      let result = self
         .webauthn
         .finish_passkey_authentication(credential, &pending.state)?;

      {
         let mut data = self.inner.lock().unwrap();
         let stored = data
            .passkeys
            .iter_mut()
            .find(|passkey| {
               passkey.user_id == pending.user_id
                  && passkey.credential_id == credential_id(result.cred_id())
            })
            .ok_or_else(|| anyhow!("passkey was not found"))?;
         if result.needs_update() {
            stored.passkey.update_credential(&result);
         }
         stored.last_used_at = Some(now());
      }
      self.save().await?;
      Ok(pending.user_id)
   }

   pub async fn delete_passkey(&self, user: &PublicUser, credential_id: &str) -> Result<()> {
      {
         let mut data = self.inner.lock().unwrap();
         let before = data.passkeys.len();
         data.passkeys.retain(|passkey| {
            !(passkey.user_id == user.id && passkey.credential_id == credential_id)
         });
         if data.passkeys.len() == before {
            return Err(anyhow!("passkey was not found"));
         }
      }
      self.save().await
   }

   fn user_credential_ids(&self, user_id: &str) -> Vec<CredentialID> {
      self
         .inner
         .lock()
         .unwrap()
         .passkeys
         .iter()
         .filter(|passkey| passkey.user_id == user_id)
         .map(|passkey| passkey.passkey.cred_id().clone())
         .collect()
   }

   fn user_passkeys(&self, user_id: &str) -> Vec<Passkey> {
      self
         .inner
         .lock()
         .unwrap()
         .passkeys
         .iter()
         .filter(|passkey| passkey.user_id == user_id)
         .map(|passkey| passkey.passkey.clone())
         .collect()
   }

   async fn save(&self) -> Result<()> {
      let data = {
         let data = self.inner.lock().unwrap();
         PasskeyData {
            passkeys: data.passkeys.clone(),
         }
      };
      write_json(&self.file, &data).await
   }
}

pub async fn register_start(
   State(state): State<RelayState>,
   headers: HeaderMap,
   Json(payload): Json<RegisterStartRequest>,
) -> impl IntoResponse {
   let user = match auth::user_from_headers(&state, &headers).await {
      Some(user) => user,
      None => return StatusCode::UNAUTHORIZED.into_response(),
   };
   match state.passkeys.start_registration(&user, payload.name).await {
      Ok(response) => Json(response).into_response(),
      Err(error) => error_response(StatusCode::BAD_REQUEST, error),
   }
}

pub async fn register_finish(
   State(state): State<RelayState>,
   headers: HeaderMap,
   Json(payload): Json<RegisterFinishRequest>,
) -> impl IntoResponse {
   let user = match auth::user_from_headers(&state, &headers).await {
      Some(user) => user,
      None => return StatusCode::UNAUTHORIZED.into_response(),
   };
   match state
      .passkeys
      .finish_registration(&user, &payload.challenge_id, &payload.credential)
      .await
   {
      Ok(passkey) => {
         Json(serde_json::json!({ "credentialId": passkey.credential_id })).into_response()
      }
      Err(error) => error_response(StatusCode::BAD_REQUEST, error),
   }
}

pub async fn login_start(
   State(state): State<RelayState>,
   Json(payload): Json<LoginStartRequest>,
) -> impl IntoResponse {
   let user = state
      .auth
      .list_users()
      .await
      .into_iter()
      .find(|user| user.username == payload.username);
   let Some(user) = user else {
      return error_response(StatusCode::UNAUTHORIZED, anyhow!("invalid login"));
   };

   match state.passkeys.start_login(&user).await {
      Ok(response) => Json(response).into_response(),
      Err(error) => error_response(StatusCode::BAD_REQUEST, error),
   }
}

pub async fn login_finish(
   State(state): State<RelayState>,
   Json(payload): Json<LoginFinishRequest>,
) -> impl IntoResponse {
   let user_id = match state
      .passkeys
      .finish_login(&payload.challenge_id, &payload.credential)
      .await
   {
      Ok(user_id) => user_id,
      Err(error) => return error_response(StatusCode::UNAUTHORIZED, error),
   };

   match state.auth.create_session_for_user_id(&user_id).await {
      Ok((user, session_token)) => (
         StatusCode::OK,
         [(header::SET_COOKIE, auth::session_cookie(&session_token))],
         Json(serde_json::json!({ "user": user })),
      )
         .into_response(),
      Err(error) => error_response(StatusCode::UNAUTHORIZED, error),
   }
}

pub async fn delete_passkey(
   State(state): State<RelayState>,
   Path(credential_id): Path<String>,
   headers: HeaderMap,
) -> impl IntoResponse {
   let user = match auth::user_from_headers(&state, &headers).await {
      Some(user) => user,
      None => return StatusCode::UNAUTHORIZED.into_response(),
   };
   match state.passkeys.delete_passkey(&user, &credential_id).await {
      Ok(()) => StatusCode::NO_CONTENT.into_response(),
      Err(error) => error_response(StatusCode::BAD_REQUEST, error),
   }
}

fn build_webauthn() -> Result<Webauthn> {
   let public_origin =
      std::env::var("RELAY_PUBLIC_ORIGIN").unwrap_or_else(|_| "http://localhost:1420".to_string());
   let rp_origin = Url::parse(&public_origin)?;
   let rp_id = std::env::var("RELAY_WEBAUTHN_RP_ID")
      .ok()
      .or_else(|| rp_origin.domain().map(ToString::to_string))
      .unwrap_or_else(|| "localhost".to_string());
   let rp_name = std::env::var("RELAY_WEBAUTHN_RP_NAME").unwrap_or_else(|_| "Relay".to_string());
   Ok(WebauthnBuilder::new(&rp_id, &rp_origin)?
      .rp_name(Box::leak(rp_name.into_boxed_str()))
      .allow_any_port(true)
      .build()?)
}

fn credential_id(credential_id: &CredentialID) -> String {
   URL_SAFE_NO_PAD.encode(credential_id.as_slice())
}

fn error_response(status: StatusCode, error: anyhow::Error) -> axum::response::Response {
   (
      status,
      Json(serde_json::json!({
         "error": error.to_string(),
         "requiresSecureOrigin": true
      })),
   )
      .into_response()
}

fn now() -> u64 {
   SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or(Duration::ZERO)
      .as_secs()
}

async fn read_json<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Result<T> {
   let bytes = tokio::fs::read(path).await?;
   Ok(serde_json::from_slice(&bytes)?)
}

async fn write_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<()> {
   if let Some(parent) = path.parent() {
      tokio::fs::create_dir_all(parent).await?;
   }
   let bytes = serde_json::to_vec_pretty(value)?;
   tokio::fs::write(path, bytes).await?;
   Ok(())
}
