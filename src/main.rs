use std::process::ExitCode;

use clap::{Parser, Subcommand};
use client::Client;
use console::style;
use error::{AuthError, Result};

mod auth;
mod client;
mod env;
mod error;
mod ipc;
mod keystore;
mod scheme;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CommandLineArgs {
    #[command(subcommand)]
    command: AppCommand,
}

#[derive(Subcommand, Debug)]
enum AppCommand {
    /// Start the authentication flow to authorize with your Jagex account
    Authorize {
        #[arg(short, long)]
        session_name: Option<String>,
    },

    /// List all characters associated with the authorized Jagex account
    #[command(name = "ls")]
    ListCharacters {
        #[arg(short, long)]
        session_name: Option<String>,
        /// Use offline cache to fetch characters
        #[arg(short, long)]
        offline: bool,
        /// Stores list of characters for offline use
        #[arg(short, long)]
        write_cache: bool,
    },

    /// Execute a program with Jagex session credentials (e.g., `RuneLite`, OSRS client)
    Exec {
        #[arg(short, long)]
        session_name: Option<String>,
        /// Use offline cache to fetch characters
        #[arg(short, long)]
        offline: bool,
        /// Character ID to use for authentication
        #[arg(short, long, help = "Character ID from 'ls' command")]
        character_id: String,
        /// Name or path of the executable to run
        exec: String,
        /// Arguments to pass to the program
        #[arg(help = "Additional arguments for the program")]
        args: Vec<String>,
    },

    /// Clear all stored authentication tokens and sessions
    Logout {
        #[arg(short, long)]
        session_name: Option<String>,
    },

    #[command(hide = true, name = "handle-callback")]
    HandleCallback { url: String },

    #[command(hide = true, name = "dump-callback")]
    DumpCallback { url: String },
}

fn main() -> ExitCode {
    env_logger::init();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("a default crypto provider was already installed");

    if let Err(error) = run() {
        eprintln!("{} {error}", style("Error:").red().bold());
        if let Some(help) = error.help() {
            eprintln!("{} {help}", style("help:").cyan().bold());
        }
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn run() -> Result<()> {
    let cli = CommandLineArgs::parse();

    match cli.command {
        AppCommand::Authorize { session_name } => auth::authorize(session_name),
        AppCommand::ListCharacters {
            session_name,
            offline,
            write_cache,
        } => {
            let client = Client::new(session_name);
            let accounts = client.accounts(offline, write_cache)?;
            for account in accounts {
                println!(
                    "  {} {} (ID: {})",
                    style("•").cyan(),
                    style(&account.display_name).green().bold(),
                    style(account.account_id.clone()).bold()
                );
            }
            Ok(())
        }
        AppCommand::Exec {
            session_name,
            offline,
            character_id,
            exec,
            args,
        } => {
            let client = Client::new(session_name);
            let session = client.session()?;
            let accounts = client.accounts(offline, false)?;

            if let Some(account) = accounts.iter().find(|a| a.account_id == character_id) {
                std::env::set_var("JX_SESSION_ID", session.session_id);
                std::env::set_var("JX_CHARACTER_ID", &account.account_id);
                std::env::set_var("JX_DISPLAY_NAME", &account.display_name);

                let mut args_with_program = args.clone();
                args_with_program.insert(0, exec.clone());
                let error = exec::execvp(&exec, args_with_program);
                Err(AuthError::ExecError {
                    program: exec.clone(),
                    details: format!("System error (errno: {error})"),
                })
            } else {
                let available_chars = accounts
                    .iter()
                    .map(|a| format!("  • {} (ID: {})", a.display_name, a.account_id))
                    .collect::<Vec<_>>()
                    .join("\n");

                Err(AuthError::CharacterNotFound {
                    character_id: character_id.clone(),
                    available_chars,
                })
            }
        }
        AppCommand::Logout { session_name } => {
            let client = Client::new(session_name);
            client.logout()
        }
        AppCommand::HandleCallback { url } => ipc::forward_callback(&url),
        AppCommand::DumpCallback { url } => {
            use std::io::Write;
            (|| -> std::io::Result<()> {
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("/tmp/auth-rs-callback-dump.log")?;
                writeln!(file, "{url}")
            })()
            .map_err(AuthError::from)
        }
    }
}
