use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum AuthError {
    #[error("Failed to open browser")]
    #[diagnostic(
        code(auth_rs::open_browser),
        help("Please try again or report this bug if it persists")
    )]
    BrowserError(String),

    #[error("Timed out waiting for the browser callback")]
    #[diagnostic(
        code(auth_rs::callback_timeout),
        help(
            "Complete the login/consent step in your browser within a few minutes, then try again"
        )
    )]
    CallbackTimeout,

    #[error("Failed to communicate with the running 'authorize' process")]
    #[diagnostic(
        code(auth_rs::ipc_error),
        help("Make sure 'auth-rs authorize' is running and try again")
    )]
    IpcError(String),

    #[error("Failed to register the OAuth callback URI scheme handler")]
    #[diagnostic(
        code(auth_rs::scheme_registration_error),
        help("Make sure 'xdg-mime' and 'update-desktop-database' are installed")
    )]
    SchemeRegistrationError(String),

    #[error("Unable to connect to Jagex servers")]
    #[diagnostic(
        code(auth_rs::network_error),
        help("• Check your internet connection\n• Try again in a few moments")
    )]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid response from server")]
    #[diagnostic(
        code(auth_rs::json_error),
        help("This appears to be a server-side issue, please try again or report this bug if it persists")
    )]
    JsonError(#[from] serde_json::Error),

    #[error("System error")]
    #[diagnostic(
        code(auth_rs::filesystem_error),
        help("Check file permissions and available disk space")
    )]
    FileSystemError(#[from] std::io::Error),

    #[error("Invalid URL format")]
    #[diagnostic(code(auth_rs::invalid_url))]
    InvalidUrl(#[from] url::ParseError),

    #[error("Unexpected response from authentication server")]
    #[diagnostic(
        code(auth_rs::invalid_response),
        help("This may indicate a temporary server issue. Please try authenticating again.")
    )]
    InvalidResponse(String),

    #[error("Not authenticated")]
    #[diagnostic(
        code(auth_rs::not_authenticated),
        help("Run 'auth-rs authorize' to log in with your Jagex account")
    )]
    SessionNotFound,

    #[error("Character '{character_id}' not found")]
    #[diagnostic(
        code(auth_rs::character_not_found),
        help("Available characters:\n{available_chars}\n\nUse one of the account IDs listed above with the --character-id option")
    )]
    CharacterNotFound {
        character_id: String,
        available_chars: String,
    },

    #[error("Failed to launch program '{program}'")]
    #[diagnostic(
        code(auth_rs::exec_error),
        help("• Make sure '{program}' is installed and in your $PATH\n• Check the program name is spelled correctly\n• Try using the full path to the executable")
    )]
    ExecError { program: String, details: String },

    #[error("Unable to access system credential store")]
    #[diagnostic(
        code(auth_rs::keyring_error),
        help("Please try again or report this bug if it persists")
    )]
    KeyringError(String),

    #[error("Credential store unavailable")]
    #[diagnostic(
        code(auth_rs::credential_store_error),
        help("Please try again or report this bug if it persists")
    )]
    CredentialStoreError(String),

    #[error("No cache directory unavailable")]
    #[diagnostic(
        code(auth_rs::no_cache_dir),
        help("Please try again or report this bug if it persists")
    )]
    NoCacheDir,
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

pub type Result<T> = miette::Result<T, AuthError>;
