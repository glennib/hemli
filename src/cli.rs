use clap::Parser;
use clap::Subcommand;

/// Secret management CLI for local development
///
/// hemli caches secrets in the OS-native keyring and fetches them on-demand
/// from external providers via shell commands. Secrets are organized by
/// namespace and automatically re-fetched when their TTL expires.
#[derive(Debug, Parser)]
#[command(name = "hemli")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Get a secret, fetching from source if needed
    ///
    /// Checks the keyring cache first. If the secret is missing or expired,
    /// fetches it from the source command, stores the result in the keyring,
    /// and prints the value to stdout.
    ///
    /// When a secret is stored with a source command, subsequent calls will
    /// automatically re-fetch using that stored source when the TTL expires --
    /// no need to pass --source-sh/--source-cmd again.
    Get {
        /// Namespace for the secret
        ///
        /// Groups secrets by project or environment. The keyring service name
        /// is "hemli:<namespace>", so secrets in different namespaces are fully
        /// isolated.
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        ///
        /// The identifier for this secret within its namespace. Used as the
        /// keyring account name.
        secret: String,

        /// Force refresh from source even if cached
        ///
        /// Re-fetches the secret from the source command regardless of whether
        /// a valid cached value exists. Mutually exclusive with --no-refresh.
        #[arg(long, conflicts_with = "no_refresh")]
        force_refresh: bool,

        /// Only return cached value, never refresh
        ///
        /// Returns the cached secret even if expired. Errors if no cached
        /// value exists. Mutually exclusive with --force-refresh.
        #[arg(long, conflicts_with = "force_refresh")]
        no_refresh: bool,

        /// Don't store the fetched secret in keyring
        ///
        /// Fetches and prints the secret but does not persist it in the
        /// keyring or update the index. Useful for one-off lookups.
        #[arg(long)]
        no_store: bool,

        /// TTL in seconds for the cached secret
        ///
        /// Sets how long the cached secret is considered valid. After this
        /// duration, the next get call will re-fetch from the source. If
        /// omitted, falls back to the TTL stored with the existing cached
        /// secret, or no expiration if none was ever set.
        #[arg(long)]
        ttl: Option<i64>,

        /// Source command to run via sh -c
        ///
        /// The command string is passed to the system shell as "sh -c <CMD>".
        /// Supports pipes, redirects, and shell syntax. Mutually exclusive
        /// with --source-cmd.
        #[arg(long, conflicts_with = "source_cmd")]
        source_sh: Option<String>,

        /// Source command to run directly
        ///
        /// The command string is split on whitespace and executed directly
        /// without a shell. Use this when you don't need shell features.
        /// Mutually exclusive with --source-sh.
        #[arg(long, conflicts_with = "source_sh")]
        source_cmd: Option<String>,
    },

    /// Delete a secret from the keyring
    ///
    /// Removes the secret from both the keyring and the index. Deleting a
    /// non-existent secret is a no-op.
    Delete {
        /// Namespace for the secret
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        secret: String,
    },

    /// List stored secrets
    ///
    /// Prints all cached secrets from the index as tab-separated lines:
    /// namespace, secret name, and creation timestamp. Use -n to filter
    /// by namespace.
    List {
        /// Filter by namespace
        ///
        /// Only show secrets belonging to this namespace. If omitted, all
        /// namespaces are shown.
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Inspect a cached secret, showing full metadata as JSON
    ///
    /// Prints the complete stored secret including value, creation time,
    /// source command, source type, TTL, and expiration time. Errors if the
    /// secret is not cached.
    Inspect {
        /// Namespace for the secret
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        secret: String,
    },

    /// Edit metadata of a cached secret (TTL, source command)
    ///
    /// Modifies metadata of an existing cached secret without re-fetching
    /// from the source. The secret value and creation timestamp are preserved.
    /// At least one modification flag must be provided.
    Edit {
        /// Namespace for the secret
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        secret: String,

        /// New TTL in seconds
        ///
        /// Replaces the existing TTL and recalculates the expiration time
        /// from the original creation timestamp. Mutually exclusive with
        /// --clear-ttl.
        #[arg(long, conflicts_with = "clear_ttl")]
        ttl: Option<i64>,

        /// Remove TTL (secret will never expire)
        ///
        /// Clears both the TTL and the expiration time so the secret never
        /// expires. Mutually exclusive with --ttl.
        #[arg(long, conflicts_with = "ttl")]
        clear_ttl: bool,

        /// New source command (sh -c)
        ///
        /// Replaces the stored source command and sets the source type to
        /// "sh". Mutually exclusive with --source-cmd.
        #[arg(long, conflicts_with = "source_cmd")]
        source_sh: Option<String>,

        /// New source command (direct)
        ///
        /// Replaces the stored source command and sets the source type to
        /// "cmd". Mutually exclusive with --source-sh.
        #[arg(long, conflicts_with = "source_sh")]
        source_cmd: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_get_basic() {
        let cli = Cli::try_parse_from(["hemli", "get", "-n", "myns", "mysecret"]).unwrap();
        match cli.command {
            Command::Get {
                namespace, secret, ..
            } => {
                assert_eq!(namespace, "myns");
                assert_eq!(secret, "mysecret");
            }
            _ => panic!("expected Get"),
        }
    }

    #[test]
    fn parse_get_with_source_sh() {
        let cli =
            Cli::try_parse_from(["hemli", "get", "-n", "ns", "sec", "--source-sh", "echo hi"])
                .unwrap();
        match cli.command {
            Command::Get { source_sh, .. } => {
                assert_eq!(source_sh.as_deref(), Some("echo hi"));
            }
            _ => panic!("expected Get"),
        }
    }

    #[test]
    fn parse_get_with_source_cmd() {
        let cli = Cli::try_parse_from([
            "hemli",
            "get",
            "-n",
            "ns",
            "sec",
            "--source-cmd",
            "my-cmd arg1",
        ])
        .unwrap();
        match cli.command {
            Command::Get { source_cmd, .. } => {
                assert_eq!(source_cmd.as_deref(), Some("my-cmd arg1"));
            }
            _ => panic!("expected Get"),
        }
    }

    #[test]
    fn source_sh_and_source_cmd_conflict() {
        let result = Cli::try_parse_from([
            "hemli",
            "get",
            "-n",
            "ns",
            "sec",
            "--source-sh",
            "echo hi",
            "--source-cmd",
            "my-cmd",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn force_refresh_and_no_refresh_conflict() {
        let result = Cli::try_parse_from([
            "hemli",
            "get",
            "-n",
            "ns",
            "sec",
            "--force-refresh",
            "--no-refresh",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_get_with_ttl() {
        let cli =
            Cli::try_parse_from(["hemli", "get", "-n", "ns", "sec", "--ttl", "3600"]).unwrap();
        match cli.command {
            Command::Get { ttl, .. } => {
                assert_eq!(ttl, Some(3600));
            }
            _ => panic!("expected Get"),
        }
    }

    #[test]
    fn parse_delete() {
        let cli = Cli::try_parse_from(["hemli", "delete", "-n", "myns", "mysecret"]).unwrap();
        match cli.command {
            Command::Delete {
                namespace, secret, ..
            } => {
                assert_eq!(namespace, "myns");
                assert_eq!(secret, "mysecret");
            }
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn parse_list_no_namespace() {
        let cli = Cli::try_parse_from(["hemli", "list"]).unwrap();
        match cli.command {
            Command::List { namespace } => {
                assert!(namespace.is_none());
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_list_with_namespace() {
        let cli = Cli::try_parse_from(["hemli", "list", "-n", "myns"]).unwrap();
        match cli.command {
            Command::List { namespace } => {
                assert_eq!(namespace.as_deref(), Some("myns"));
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_inspect() {
        let cli = Cli::try_parse_from(["hemli", "inspect", "-n", "myns", "mysecret"]).unwrap();
        match cli.command {
            Command::Inspect {
                namespace, secret, ..
            } => {
                assert_eq!(namespace, "myns");
                assert_eq!(secret, "mysecret");
            }
            _ => panic!("expected Inspect"),
        }
    }

    #[test]
    fn inspect_missing_namespace_errors() {
        let result = Cli::try_parse_from(["hemli", "inspect", "mysecret"]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_edit_with_ttl() {
        let cli = Cli::try_parse_from(["hemli", "edit", "-n", "myns", "mysecret", "--ttl", "7200"])
            .unwrap();
        match cli.command {
            Command::Edit {
                namespace,
                secret,
                ttl,
                clear_ttl,
                source_sh,
                source_cmd,
            } => {
                assert_eq!(namespace, "myns");
                assert_eq!(secret, "mysecret");
                assert_eq!(ttl, Some(7200));
                assert!(!clear_ttl);
                assert!(source_sh.is_none());
                assert!(source_cmd.is_none());
            }
            _ => panic!("expected Edit"),
        }
    }

    #[test]
    fn parse_edit_with_clear_ttl() {
        let cli = Cli::try_parse_from(["hemli", "edit", "-n", "ns", "sec", "--clear-ttl"]).unwrap();
        match cli.command {
            Command::Edit { clear_ttl, ttl, .. } => {
                assert!(clear_ttl);
                assert!(ttl.is_none());
            }
            _ => panic!("expected Edit"),
        }
    }

    #[test]
    fn edit_ttl_and_clear_ttl_conflict() {
        let result = Cli::try_parse_from([
            "hemli",
            "edit",
            "-n",
            "ns",
            "sec",
            "--ttl",
            "60",
            "--clear-ttl",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_edit_with_source_sh() {
        let cli =
            Cli::try_parse_from(["hemli", "edit", "-n", "ns", "sec", "--source-sh", "echo hi"])
                .unwrap();
        match cli.command {
            Command::Edit {
                source_sh,
                source_cmd,
                ..
            } => {
                assert_eq!(source_sh.as_deref(), Some("echo hi"));
                assert!(source_cmd.is_none());
            }
            _ => panic!("expected Edit"),
        }
    }

    #[test]
    fn parse_edit_with_source_cmd() {
        let cli = Cli::try_parse_from([
            "hemli",
            "edit",
            "-n",
            "ns",
            "sec",
            "--source-cmd",
            "my-cmd arg1",
        ])
        .unwrap();
        match cli.command {
            Command::Edit {
                source_sh,
                source_cmd,
                ..
            } => {
                assert!(source_sh.is_none());
                assert_eq!(source_cmd.as_deref(), Some("my-cmd arg1"));
            }
            _ => panic!("expected Edit"),
        }
    }

    #[test]
    fn edit_source_sh_and_source_cmd_conflict() {
        let result = Cli::try_parse_from([
            "hemli",
            "edit",
            "-n",
            "ns",
            "sec",
            "--source-sh",
            "echo hi",
            "--source-cmd",
            "my-cmd",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn edit_missing_namespace_errors() {
        let result = Cli::try_parse_from(["hemli", "edit", "mysecret", "--ttl", "60"]);
        assert!(result.is_err());
    }

    #[test]
    fn missing_namespace_errors() {
        let result = Cli::try_parse_from(["hemli", "get", "mysecret"]);
        assert!(result.is_err());
    }

    #[test]
    fn missing_secret_name_errors() {
        let result = Cli::try_parse_from(["hemli", "get", "-n", "ns"]);
        assert!(result.is_err());
    }
}
