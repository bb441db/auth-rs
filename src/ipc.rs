use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::error::{AuthError, Result};

fn socket_path() -> PathBuf {
    let dir = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
    dir.join("auth-rs-callback.sock")
}

pub fn wait_for_callback(timeout: Duration) -> Result<String> {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);

    let listener = UnixListener::bind(&path).map_err(|e| {
        AuthError::IpcError(format!(
            "Failed to bind callback socket at {}: {e}",
            path.display()
        ))
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|e| AuthError::IpcError(format!("Failed to configure callback socket: {e}")))?;

    let start = Instant::now();
    let result = loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = String::new();
                match stream.read_to_string(&mut buf) {
                    Ok(_) => break Ok(buf.trim().to_string()),
                    Err(e) => {
                        break Err(AuthError::IpcError(format!("Failed to read callback: {e}")))
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if start.elapsed() > timeout {
                    break Err(AuthError::CallbackTimeout);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                break Err(AuthError::IpcError(format!(
                    "Failed to accept callback connection: {e}"
                )))
            }
        }
    };

    let _ = std::fs::remove_file(&path);
    result
}

pub fn forward_callback(url: &str) -> Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        AuthError::IpcError(format!(
            "Failed to connect to a running 'auth-rs authorize' process at {}: {e}",
            path.display()
        ))
    })?;
    stream
        .write_all(url.as_bytes())
        .map_err(|e| AuthError::IpcError(format!("Failed to forward callback: {e}")))?;
    Ok(())
}
