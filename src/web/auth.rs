// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::session_key::{AUTH_CALLBACK_PATH, AUTH_CSRF_STATE, AUTH_CSRF_VERIFIER, DISCORD_USER};
use super::state::AppState;
use crate::config::ConfigData;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use miette::IntoDiagnostic;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
	AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl,
	Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tower_sessions::Session;
use twilight_http::client::Client;
use twilight_model::id::marker::UserMarker;
use twilight_model::id::Id;

pub const DISCORD_AUTH_URL: &str = "https://discord.com/oauth2/authorize";
pub const DISCORD_AUTH_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";

/// Gets the OAuth client object for interacting with Discord as an OAuth2 client
fn discord_oauth_client(config: &ConfigData) -> miette::Result<BasicClient> {
	let client_id = ClientId::new(config.discord.client_id.clone());
	let client_secret = ClientSecret::new(config.discord.client_secret.clone());

	let auth_url = AuthUrl::new(DISCORD_AUTH_URL.to_string()).into_diagnostic()?;
	let token_url = TokenUrl::new(DISCORD_AUTH_TOKEN_URL.to_string()).into_diagnostic()?;

	let redirect_url = RedirectUrl::new(format!("{}/discord_auth_callback", config.web.base_url)).into_diagnostic()?;

	let client =
		BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url)).set_redirect_uri(redirect_url);
	Ok(client)
}

/// Axum layer function for Discord authorizations. Redirects all callbacks for sessions not authenticated to Discord
/// and aren't hitting the OAuth callback path to Discord's OAuth login.
pub async fn discord_auth_layer(
	State(state): State<AppState>,
	session: Session,
	request: Request,
	next: Next,
) -> Response {
	let user_id: Option<Id<UserMarker>> = match session.get(DISCORD_USER).await {
		Ok(id) => id,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to retrieve user ID from session");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	// Don't try to handle the path where we're returning from OAuth to authenticate the user
	if user_id.is_none() {
		let request_uri = request.uri();
		if request_uri.path() != "/discord_auth_callback" {
			let request_path_with_query = request_uri.path_and_query();
			let request_path_with_query = request_path_with_query
				.map(|path_and_query| path_and_query.as_str().to_string())
				.unwrap_or_default();
			let insert_result = session.insert(AUTH_CALLBACK_PATH, request_path_with_query).await;
			if let Err(insert_error) = insert_result {
				tracing::error!(source = ?insert_error, "Failed to set callback path to session");
				return StatusCode::INTERNAL_SERVER_ERROR.into_response();
			}

			let oauth_client = match discord_oauth_client(&state.config) {
				Ok(client) => client,
				Err(error) => {
					tracing::error!(source = ?error, "Failed to set up oauth client");
					return StatusCode::INTERNAL_SERVER_ERROR.into_response();
				}
			};
			let (code_challenge, code_verifier) = PkceCodeChallenge::new_random_sha256();

			let (oauth_url, csrf_state) = oauth_client
				.authorize_url(CsrfToken::new_random)
				.add_scope(Scope::new(String::from("identify")))
				.set_pkce_challenge(code_challenge)
				.url();

			let insert_result = session.insert(AUTH_CSRF_STATE, csrf_state.secret().clone()).await;
			if let Err(error) = insert_result {
				tracing::error!(source = ?error, "Failed to set oauth validation info to session");
				return StatusCode::INTERNAL_SERVER_ERROR.into_response();
			}
			let insert_result = session.insert(AUTH_CSRF_VERIFIER, code_verifier.secret().clone()).await;
			if let Err(error) = insert_result {
				tracing::error!(source = ?error, "Failed to set oauth validation info to session");
				return StatusCode::INTERNAL_SERVER_ERROR.into_response();
			}

			return Redirect::to(oauth_url.as_str()).into_response();
		}
	}

	next.run(request).await
}

#[derive(Debug, Deserialize)]
pub struct CallbackArgs {
	code: String,
	state: String,
}

/// Route function for the OAuth login callback
#[axum::debug_handler]
pub async fn discord_auth_route(
	Query(query): Query<CallbackArgs>,
	session: Session,
	State(state): State<AppState>,
) -> Response {
	let csrf_state: Option<String> = match session.remove(AUTH_CSRF_STATE).await {
		Ok(state) => state,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to get CSRF state for login callback");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};
	let code_verifier: Option<String> = match session.remove(AUTH_CSRF_VERIFIER).await {
		Ok(verifier) => verifier,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to get CSRF code verifier for login callback");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};
	let redirect_path: Option<String> = match session.remove(AUTH_CALLBACK_PATH).await {
		Ok(path) => path,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to get callback redirect path for login callback");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let (Some(csrf_state), Some(code_verifier), Some(redirect_path)) = (csrf_state, code_verifier, redirect_path)
	else {
		return StatusCode::BAD_REQUEST.into_response();
	};

	if csrf_state != query.state {
		return StatusCode::BAD_REQUEST.into_response();
	}

	let oauth_client = match discord_oauth_client(&state.config) {
		Ok(client) => client,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to set up oauth client");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let auth_code = AuthorizationCode::new(query.code);
	let code_verifier = PkceCodeVerifier::new(code_verifier);

	let token_response = oauth_client
		.exchange_code(auth_code)
		.set_pkce_verifier(code_verifier)
		.request_async(async_http_client)
		.await;
	let token_response = match token_response {
		Ok(response) => response,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to get token response for oauth");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	let discord_user_client = Client::builder()
		.token(format!("Bearer {}", token_response.access_token().secret()))
		.build();
	let discord_user_response = discord_user_client.current_user().await;
	let discord_user_response = match discord_user_response {
		Ok(user) => user,
		Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
	};
	let discord_user = discord_user_response.model().await;
	let discord_user = match discord_user {
		Ok(user) => user,
		Err(error) => {
			tracing::error!(source = ?error, "Failed to extract Discord user info");
			return StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};
	let discord_user_id = discord_user.id;

	let insert_result = session.insert(DISCORD_USER, discord_user_id).await;
	if let Err(error) = insert_result {
		tracing::error!(source = ?error, "Failed to store Discord user ID");
		return StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	Redirect::to(&redirect_path).into_response()
}
