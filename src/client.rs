use std::path::PathBuf;

use crate::error::{AuthError, Result};
use keyring_core::Entry;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SessionRequest {
    #[serde(rename = "idToken")]
    id_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::struct_field_names)] // `account_id` is clearer than `id` next to `display_name`/`user_hash`
pub struct Account {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "userHash")]
    pub user_hash: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Session {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

struct SessionStore;

impl SessionStore {
    const SERVICE: &'static str = "auth-rs";

    fn use_file_storage() -> bool {
        !std::env::var("AUTH_RS_USE_KEYRING")
            .is_ok_and(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
    }

    fn key(session_name: Option<&String>) -> String {
        match session_name {
            Some(session_name) => format!("named-session-{session_name}"),
            None => "session".to_owned(),
        }
    }

    fn get_entry(session_name: Option<&String>) -> Result<Entry> {
        crate::keystore::ensure_initialized()?;
        Entry::new(Self::SERVICE, &Self::key(session_name)).map_err(AuthError::from)
    }

    fn file_path(session_name: Option<&String>) -> Result<PathBuf> {
        let mut path = dirs::data_local_dir().ok_or(AuthError::NoCacheDir)?;
        path.push("auth-rs");
        path.push("sessions");
        std::fs::create_dir_all(&path)?;
        path.push(format!("{}.json", Self::key(session_name)));
        Ok(path)
    }

    fn write_file_restricted(path: &std::path::Path, contents: &str) -> Result<()> {
        std::fs::write(path, contents)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    fn store(session_name: Option<&String>, session: &Session) -> Result<()> {
        let session_json = serde_json::to_string(session)?;

        if Self::use_file_storage() {
            let path = Self::file_path(session_name)?;
            Self::write_file_restricted(&path, &session_json)
        } else {
            let entry = Self::get_entry(session_name)?;
            entry.set_password(&session_json).map_err(AuthError::from)
        }
    }

    fn load(session_name: Option<&String>) -> Result<Option<Session>> {
        if Self::use_file_storage() {
            let path = Self::file_path(session_name)?;
            match std::fs::read_to_string(&path) {
                Ok(session_json) => Ok(Some(serde_json::from_str(&session_json)?)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(AuthError::from(e)),
            }
        } else {
            let entry = Self::get_entry(session_name)?;
            match entry.get_password() {
                Ok(session_json) => {
                    let session: Session = serde_json::from_str(&session_json)?;
                    Ok(Some(session))
                }
                Err(keyring_core::Error::NoEntry) => Ok(None),
                Err(e) => Err(AuthError::from(e)),
            }
        }
    }

    fn clear(session_name: Option<&String>) -> Result<()> {
        if Self::use_file_storage() {
            let path = Self::file_path(session_name)?;
            match std::fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(AuthError::from(e)),
            }
        } else {
            let entry = Self::get_entry(session_name)?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
                Err(e) => Err(AuthError::from(e)),
            }
        }
    }
}

pub struct Client {
    session_name: Option<String>,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(session_name: Option<String>) -> Self {
        let tls_config = ureq::tls::TlsConfig::builder()
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .tls_config(tls_config)
            .build();
        Self {
            session_name,
            agent: ureq::Agent::new_with_config(config),
        }
    }

    pub fn create_session(&self, token: &str) -> Result<Session> {
        let url = "https://auth.jagex.com/game-session/v1/sessions";
        let body = SessionRequest {
            id_token: token.to_owned(),
        };
        let response = self
            .agent
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send_json(&body)?;
        let session: Session = Self::parse_json_response(response)?;
        SessionStore::store(self.session_name.as_ref(), &session)?;
        self.clear_accounts_cache()?;
        Ok(session)
    }

    fn parse_json_response<T: serde::de::DeserializeOwned>(
        mut response: ureq::http::Response<ureq::Body>,
    ) -> Result<T> {
        let status = response.status();
        let body = response.body_mut().read_to_string()?;

        if !status.is_success() {
            return Err(AuthError::InvalidResponse(format!(
                "Server returned {status}: {body}"
            )));
        }

        serde_json::from_str(&body).map_err(|e| {
            AuthError::InvalidResponse(format!("Unexpected response shape ({e}): {body}"))
        })
    }

    pub fn session(&self) -> Result<Session> {
        SessionStore::load(self.session_name.as_ref())?.ok_or(AuthError::SessionNotFound)
    }

    fn clear_accounts_cache(&self) -> Result<()> {
        let path = match self.accounts_cache_dir() {
            Ok(path) => path,
            Err(AuthError::NoCacheDir) => return Ok(()),
            Err(e) => return Err(e),
        };

        if path.exists() {
            return Ok(std::fs::remove_dir_all(path)?);
        }

        Ok(())
    }

    fn accounts_cache_dir(&self) -> Result<PathBuf> {
        let mut path = dirs::cache_dir().ok_or(AuthError::NoCacheDir)?;
        let key = match &self.session_name {
            Some(session_name) => format!("named-session-{session_name}"),
            None => "session".to_owned(),
        };
        path = path.join("auth-rs");
        path = path.join(key);
        Ok(path)
    }

    fn accounts_cache(&self) -> Result<Vec<Account>> {
        let path = self.accounts_cache_dir()?;
        let path = path.join("accounts.json");

        if !path.exists() {
            return Ok(vec![]);
        }

        let file = std::fs::File::open(path)?;
        let accounts: Vec<Account> = serde_json::from_reader(file)?;
        Ok(accounts)
    }

    fn store_accounts(&self, accounts: &Vec<Account>) -> Result<()> {
        let path = self.accounts_cache_dir()?;

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let path = path.join("accounts.json");
        let file = std::fs::File::create(path)?;

        serde_json::to_writer(file, accounts)?;

        Ok(())
    }

    pub fn accounts(&self, offline: bool, store_offline: bool) -> Result<Vec<Account>> {
        let session = self.session()?;

        if offline {
            return self.accounts_cache();
        }

        let url = "https://auth.jagex.com/game-session/v1/accounts";
        let response = self
            .agent
            .get(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", session.session_id))
            .call()?;
        let accounts: Vec<Account> = Self::parse_json_response(response)?;

        if store_offline {
            self.store_accounts(&accounts)?;
        }

        Ok(accounts)
    }

    pub fn logout(&self) -> Result<()> {
        SessionStore::clear(self.session_name.as_ref())?;
        self.clear_accounts_cache()?;

        Ok(())
    }
}
