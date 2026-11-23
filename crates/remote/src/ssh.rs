use tokio::process::Command;

use crate::RemoteError;

#[derive(Debug, Clone)]
pub struct SshTarget {
    pub host: String,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub remote_path: String,
}

impl SshTarget {
    pub fn destination(&self) -> String {
        match &self.user {
            Some(u) => format!("{u}@{}", self.host),
            None => self.host.clone(),
        }
    }
}

/// Executes a command on a remote SSH workspace using the system `ssh` binary
/// with BatchMode enabled (never prompts or stores plaintext passwords).
pub async fn run_ssh_command(target: &SshTarget, remote_cmd: &str) -> Result<String, RemoteError> {
    let mut cmd = Command::new("ssh");
    cmd.arg("-o").arg("BatchMode=yes");
    if let Some(port) = target.port {
        cmd.arg("-p").arg(port.to_string());
    }
    cmd.arg(target.destination())
        .arg(format!("cd {} && {}", target.remote_path, remote_cmd));

    let out = cmd
        .output()
        .await
        .map_err(|e| RemoteError::Transport(e.to_string()))?;
    if !out.status.success() {
        return Err(RemoteError::Transport(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
