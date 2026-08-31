use std::time::Duration;

use log::info;
use url::Url;
use uuid::Uuid;

use crate::{
    client::Client,
    error::{AuthError, Result},
    ipc, scheme,
};

const CALLBACK_TIMEOUT: Duration = Duration::from_mins(5);

#[derive(Debug, Clone)]
struct AuthOptions {
    state: String,
    nonce: String,
}

impl AuthOptions {
    fn new() -> Self {
        Self {
            state: Uuid::new_v4().to_string(),
            nonce: Uuid::new_v4().to_string(),
        }
    }
}

fn create_auth_url() -> Result<(String, AuthOptions)> {
    let auth_options = AuthOptions::new();
    let mut url = Url::parse(crate::env::ORIGIN)?.join("/oauth2/auth")?;
    let mut query = url.query_pairs_mut();
    query.append_pair("response_type", "id_token");
    query.append_pair("client_id", crate::env::CLIENT_ID);
    query.append_pair("scope", "openid");
    query.append_pair("redirect_uri", crate::env::REDIRECT);
    query.append_pair("state", &auth_options.state);
    query.append_pair("nonce", &auth_options.nonce);
    drop(query);

    Ok((url.as_str().to_owned(), auth_options))
}

fn parse_callback(url: &str) -> Option<(String, String)> {
    let url_with_query = url.replace('#', "?");
    let parsed_url = Url::parse(&url_with_query).ok()?;

    if parsed_url.scheme() != crate::env::CALLBACK_SCHEME {
        return None;
    }

    let id_token = parsed_url.query_pairs().find(|q| q.0 == "id_token")?.1;
    let state = parsed_url.query_pairs().find(|q| q.0 == "state")?.1;

    Some((id_token.into_owned(), state.into_owned()))
}

fn open_in_browser(url: &str) -> Result<()> {
    opener::open_browser(url)
        .map_err(|e| AuthError::BrowserError(format!("Failed to open browser: {e}")))
}

pub fn authorize(session_name: Option<String>) -> Result<()> {
    scheme::ensure_registered()?;

    let client = Client::new(session_name);

    let (auth_url, options) = create_auth_url()?;
    open_in_browser(&auth_url)?;

    info!("Waiting for authorization to complete in the browser...");
    let callback = ipc::wait_for_callback(CALLBACK_TIMEOUT)?;
    let (id_token, state) = parse_callback(&callback)
        .ok_or_else(|| AuthError::InvalidResponse("Unrecognized callback URL".to_string()))?;

    if state != options.state {
        return Err(AuthError::InvalidResponse(
            "State parameter mismatch - possible CSRF attack".to_string(),
        ));
    }

    client.create_session(&id_token)?;

    Ok(())
}
