use crate::errors::{run_command_with_timeout, ServiceProbeError, ServiceProbeErrorKind};
use crate::ServiceProbe;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

const LAUNCHD_LABEL: &str = "com.fs-jira.mount";

pub fn probe_service() -> Result<ServiceProbe, ServiceProbeError> {
    let plist_path = resolve_plist_path();
    let installed = plist_path.exists();
    let (config_path, mountpoint) = if installed {
        let content = fs::read_to_string(&plist_path).map_err(|error| ServiceProbeError {
            kind: ServiceProbeErrorKind::ParseError,
            message: format!(
                "failed to read launchd plist at {}: {error}",
                plist_path.display()
            ),
        })?;
        parse_program_arguments(&content)
    } else {
        (None, None)
    };

    let uid = nix_like_uid();
    let mut command = Command::new("launchctl");
    command
        .args(["print", &format!("gui/{uid}/{LAUNCHD_LABEL}")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = run_command_with_timeout(command, Duration::from_secs(2))?;
    let running = output.status_ok;

    Ok(ServiceProbe {
        installed,
        running,
        config_path,
        mountpoint,
    })
}

pub fn start_service() -> Result<(), ServiceProbeError> {
    let uid = nix_like_uid();
    let domain = format!("gui/{uid}");
    let label_target = format!("{domain}/{LAUNCHD_LABEL}");

    let mut kickstart = Command::new("launchctl");
    kickstart
        .args(["kickstart", "-k", &label_target])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let kickstart_output = run_command_with_timeout(kickstart, Duration::from_secs(5))?;
    if kickstart_output.status_ok {
        return Ok(());
    }

    let plist_path = resolve_plist_path();
    if !plist_path.exists() {
        return Err(ServiceProbeError {
            kind: ServiceProbeErrorKind::NotInstalled,
            message: format!(
                "launchd plist not found at {}; install service first",
                plist_path.display()
            ),
        });
    }

    let plist_path_str = plist_path.to_string_lossy().to_string();
    let mut bootstrap = Command::new("launchctl");
    bootstrap
        .args(["bootstrap", &domain, &plist_path_str])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let bootstrap_output = run_command_with_timeout(bootstrap, Duration::from_secs(5))?;
    if !bootstrap_output.status_ok {
        let lowered = bootstrap_output.stderr.to_ascii_lowercase();
        let already_bootstrapped = lowered.contains("already loaded") || lowered.contains("in use");
        if already_bootstrapped {
            // Continue to kickstart when the service is already loaded.
        } else {
            return Err(ServiceProbeError {
                kind: ServiceProbeErrorKind::Unreachable,
                message: format!(
                    "failed to bootstrap {}: {}",
                    LAUNCHD_LABEL, bootstrap_output.stderr
                ),
            });
        }
    }

    let mut retry_kickstart = Command::new("launchctl");
    retry_kickstart
        .args(["kickstart", "-k", &label_target])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let retry_output = run_command_with_timeout(retry_kickstart, Duration::from_secs(5))?;
    if retry_output.status_ok {
        Ok(())
    } else {
        return Err(ServiceProbeError {
            kind: ServiceProbeErrorKind::Unreachable,
            message: format!("failed to start {}: {}", LAUNCHD_LABEL, retry_output.stderr),
        });
    }
}

fn resolve_plist_path() -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join("com.fs-jira.mount.plist")
}

fn nix_like_uid() -> String {
    unsafe { libc::geteuid() }.to_string()
}

pub fn parse_program_arguments(plist_content: &str) -> (Option<String>, Option<String>) {
    let mut in_program_arguments = false;
    let mut args = Vec::new();

    for line in plist_content.lines() {
        let trimmed = line.trim();
        if trimmed == "<key>ProgramArguments</key>" {
            in_program_arguments = true;
            continue;
        }
        if !in_program_arguments {
            continue;
        }
        if trimmed == "</array>" {
            break;
        }
        if trimmed.starts_with("<string>") && trimmed.ends_with("</string>") {
            let value = trimmed
                .trim_start_matches("<string>")
                .trim_end_matches("</string>")
                .to_string();
            args.push(value);
        }
    }

    let mut config_path = None;
    for (idx, token) in args.iter().enumerate() {
        if token == "--config" {
            config_path = args.get(idx + 1).cloned();
        }
    }

    let mountpoint = args
        .iter()
        .rev()
        .find(|token| !token.starts_with('-'))
        .cloned();

    (config_path, mountpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_launchd_program_arguments() {
        let content = r#"
<key>ProgramArguments</key>
<array>
  <string>/usr/local/bin/fs-jira</string>
  <string>--config</string>
  <string>/tmp/config.toml</string>
  <string>/tmp/fs-jira</string>
</array>
"#;

        let (config, mountpoint) = parse_program_arguments(content);
        assert_eq!(config.as_deref(), Some("/tmp/config.toml"));
        assert_eq!(mountpoint.as_deref(), Some("/tmp/fs-jira"));
    }
}
