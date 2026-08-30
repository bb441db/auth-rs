use crate::error::{AuthError, Result};

const DESKTOP_FILE_NAME: &str = "auth-rs-handler.desktop";

#[cfg(target_os = "linux")]
pub fn ensure_registered() -> Result<()> {
    let apps_dir = dirs::data_local_dir()
        .ok_or(AuthError::NoCacheDir)?
        .join("applications");
    std::fs::create_dir_all(&apps_dir)?;

    let exe = std::env::current_exe()?;
    let desktop_path = apps_dir.join(DESKTOP_FILE_NAME);
    let mime_type = format!("x-scheme-handler/{}", crate::env::CALLBACK_SCHEME);

    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=auth-rs Callback Handler\n\
         Exec={} handle-callback %u\n\
         NoDisplay=true\n\
         MimeType={};\n",
        exe.display(),
        mime_type,
    );

    let up_to_date = std::fs::read_to_string(&desktop_path)
        .map(|existing| existing == contents)
        .unwrap_or(false);

    if !up_to_date {
        std::fs::write(&desktop_path, contents)?;

        let _ = std::process::Command::new("update-desktop-database")
            .arg(&apps_dir)
            .status();
    }

    let status = std::process::Command::new("xdg-mime")
        .args(["default", DESKTOP_FILE_NAME, &mime_type])
        .status()
        .map_err(|e| AuthError::SchemeRegistrationError(format!("Failed to run xdg-mime: {e}")))?;

    if !status.success() {
        return Err(AuthError::SchemeRegistrationError(
            "xdg-mime exited with a non-zero status".to_string(),
        ));
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn ensure_registered() -> Result<()> {
    Ok(())
}
