use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Failed to open browser")]
    BrowserError(String),

    #[error("Timed out waiting for the browser callback")]
    CallbackTimeout,

    #[error("Failed to communicate with the running 'authorize' process")]
    IpcError(String),

    #[error("Failed to register the OAuth callback URI scheme handler")]
    SchemeRegistrationError(String),

    #[error("Unable to connect to Jagex servers")]
    NetworkError(#[from] ureq::Error),

    #[error("Invalid response from server")]
    JsonError(#[from] serde_json::Error),

    #[error("System error")]
    FileSystemError(#[from] std::io::Error),

    #[error("Invalid URL format")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Unexpected response from authentication server")]
    InvalidResponse(String),

    #[error("Not authenticated")]
    SessionNotFound,

    #[error("Character '{character_id}' not found")]
    CharacterNotFound {
        character_id: String,
        available_chars: String,
    },

    #[error("Failed to launch program '{program}'")]
    ExecError { program: String, details: String },

    #[error("Unable to access system credential store")]
    KeyringError(String),

    #[error("Credential store unavailable")]
    CredentialStoreError(String),

    #[error("No cache directory unavailable")]
    NoCacheDir,
}

impl AuthError {
    pub fn help(&self) -> Option<String> {
        match self {
            Self::BrowserError(_)
            | Self::KeyringError(_)
            | Self::CredentialStoreError(_)
            | Self::NoCacheDir => {
                Some("Please try again or report this bug if it persists".to_owned())
            }
            Self::CallbackTimeout => Some(
                "Complete the login/consent step in your browser within a few minutes, then try again"
                    .to_owned(),
            ),
            Self::IpcError(_) => Some("Make sure 'auth-rs authorize' is running and try again".to_owned()),
            Self::SchemeRegistrationError(_) => {
                Some("Make sure 'xdg-mime' and 'update-desktop-database' are installed".to_owned())
            }
            Self::NetworkError(_) => {
                Some("• Check your internet connection\n• Try again in a few moments".to_owned())
            }
            Self::JsonError(_) | Self::InvalidResponse(_) => Some(
                "This may indicate a temporary server issue. Please try authenticating again."
                    .to_owned(),
            ),
            Self::FileSystemError(_) => Some("Check file permissions and available disk space".to_owned()),
            Self::InvalidUrl(_) => None,
            Self::SessionNotFound => {
                Some("Run 'auth-rs authorize' to log in with your Jagex account".to_owned())
            }
            Self::CharacterNotFound { available_chars, .. } => Some(format!(
                "Available characters:\n{available_chars}\n\nUse one of the account IDs listed above with the --character-id option"
            )),
            Self::ExecError { program, .. } => Some(format!(
                "• Make sure '{program}' is installed and in your $PATH\n• Check the program name is spelled correctly\n• Try using the full path to the executable"
            )),
        }
    }
}

impl From<keyring_core::Error> for AuthError {
    fn from(error: keyring_core::Error) -> Self {
        match error {
            keyring_core::Error::NoEntry => AuthError::SessionNotFound,
            keyring_core::Error::PlatformFailure(e) => {
                AuthError::CredentialStoreError(e.to_string())
            }
            _ => AuthError::KeyringError(error.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, AuthError>;
