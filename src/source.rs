use std::process::Command;

use crate::error::HemliError;
use crate::model::SourceType;

pub fn fetch_secret(command: &str, source_type: &SourceType) -> Result<String, HemliError> {
    let output = match source_type {
        SourceType::Sh => Command::new("sh").arg("-c").arg(command).output()?,
        SourceType::Cmd => {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.is_empty() {
                return Err(HemliError::SourceFailed("empty command".into()));
            }
            Command::new(parts[0]).args(&parts[1..]).output()?
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(HemliError::SourceFailed(format!(
            "command exited with {}: {}",
            output.status,
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_echo() {
        let result = fetch_secret("echo hello", &SourceType::Sh).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn cmd_echo() {
        let result = fetch_secret("echo hello", &SourceType::Cmd).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn sh_failure() {
        let result = fetch_secret("exit 1", &SourceType::Sh);
        assert!(result.is_err());
        match result.unwrap_err() {
            HemliError::SourceFailed(_) => {}
            other => panic!("expected SourceFailed, got {other:?}"),
        }
    }

    #[test]
    fn cmd_failure() {
        let result = fetch_secret("false", &SourceType::Cmd);
        assert!(result.is_err());
    }

    #[test]
    fn whitespace_trimming() {
        let result = fetch_secret("echo '  hello  '", &SourceType::Sh).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn sh_multiword_output() {
        let result = fetch_secret("echo 'hello world'", &SourceType::Sh).unwrap();
        assert_eq!(result, "hello world");
    }
}
