use crate::error::{AuthError, Result};
use std::path::PathBuf;

fn get_applications_dir() -> Result<PathBuf> {
    // Equivalent of "${XDG_DATA_HOME:-$HOME/.local/share}"
    let data_dir = dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")))
        .ok_or(AuthError::NoCacheDir)?;
    
    let applications_dir = data_dir.join("applications");

    std::fs::create_dir_all(&applications_dir)?;
    
    Ok(applications_dir)
}

fn build_exec_command(
    session_name: &Option<String>,
    character_id: &str,
    exec: &str,
    args: &[String],
) -> String {
    let mut exec_cmd = vec!["auth-rs".to_string(), "exec".to_string()];

    if let Some(session) = session_name {
        exec_cmd.push("--session-name".to_string());
        exec_cmd.push(session.clone());
    }
    
    exec_cmd.push("--character-id".to_string());
    exec_cmd.push(character_id.to_string());
    exec_cmd.push(exec.to_string());
    
    if !args.is_empty() {
        exec_cmd.push("--".to_string());
        exec_cmd.extend(args.iter().cloned());
    }
    
    exec_cmd.join(" ")
}

pub fn create_entry(
    session_name: Option<String>,
    name: String,
    character_id: String,
    exec: String,
    args: Vec<String>,
) -> Result<PathBuf> {
    let applications_dir = get_applications_dir()?;
    let exec_command = build_exec_command(&session_name, &character_id, &exec, &args);
    // TODO: What to do about the RuneLite references below?
    let contents = format!(
        r#"[Desktop Entry]
Name={}
Comment=Launch RuneLite
Exec={}
Icon=runelite
Terminal=false
Type=Application
Categories=Game;
"#,
        name, exec_command
    );

    let filename = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase();
    
    let desktop_entry = applications_dir.join(format!("{}.desktop", filename));

    std::fs::write(&desktop_entry, contents)?;
    
    Ok(desktop_entry)
}