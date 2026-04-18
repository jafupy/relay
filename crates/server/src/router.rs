use crate::{
   auth,
   events::events_socket,
   rpc,
   state::RelayState,
   static_files::{file_asset, serve_app, serve_dev_websocket},
   webauthn,
};
use axum::{
   Router,
   middleware::from_fn_with_state,
   routing::{delete, get, post},
};

pub fn build_router(state: RelayState) -> Router {
   let protected = Router::new()
      .route("/api/rpc/{command}", post(rpc::handle_rpc))
      .route("/api/auth/password", post(auth::change_password))
      .route(
         "/api/auth/users",
         get(auth::list_users).post(auth::create_user),
      )
      .route(
         "/api/auth/passkeys/register/start",
         post(webauthn::register_start),
      )
      .route(
         "/api/auth/passkeys/register/finish",
         post(webauthn::register_finish),
      )
      .route(
         "/api/auth/passkeys/{credential_id}",
         delete(webauthn::delete_passkey),
      )
      .route("/api/events", get(events_socket))
      .route("/assets/file", get(file_asset))
      .route("/assets/extension", get(file_asset))
      .layer(from_fn_with_state(state.clone(), auth::require_auth));

   Router::new()
      .route("/api/health", get(rpc::health))
      .route("/api/version", get(rpc::version))
      .route("/api/platform", get(rpc::platform))
      .route("/api/auth/login", post(auth::login))
      .route("/api/auth/logout", post(auth::logout))
      .route("/api/auth/me", get(auth::me))
      .route(
         "/api/auth/passkeys/login/start",
         post(webauthn::login_start),
      )
      .route(
         "/api/auth/passkeys/login/finish",
         post(webauthn::login_finish),
      )
      .route("/@vite-hmr", get(serve_dev_websocket))
      .merge(protected)
      .fallback(serve_app)
      .with_state(state)
}
