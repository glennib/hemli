use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Parser)]
#[command(name = "hemli", about = "Secret management CLI for local development")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Get a secret, fetching from source if needed
    Get {
        /// Namespace for the secret
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        secret: String,

        /// Force refresh from source even if cached
        #[arg(long, conflicts_with = "no_refresh")]
        force_refresh: bool,

        /// Only return cached value, never refresh
        #[arg(long, conflicts_with = "force_refresh")]
        no_refresh: bool,

        /// Don't store the fetched secret in keyring
        #[arg(long)]
        no_store: bool,

        /// TTL in seconds for the cached secret
        #[arg(long)]
        ttl: Option<i64>,

        /// Source command to run via sh -c
        #[arg(long, conflicts_with = "source_cmd")]
        source_sh: Option<String>,

        /// Source command to run directly
        #[arg(long, conflicts_with = "source_sh")]
        source_cmd: Option<String>,
    },

    /// Delete a secret from the keyring
    Delete {
        /// Namespace for the secret
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        secret: String,
    },

    /// List stored secrets
    List {
        /// Filter by namespace
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Inspect a cached secret, showing full metadata as JSON
    Inspect {
        /// Namespace for the secret
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        secret: String,
    },

    /// Edit metadata of a cached secret (TTL, source command)
    Edit {
        /// Namespace for the secret
        #[arg(short, long)]
        namespace: String,

        /// Name of the secret
        secret: String,

        /// New TTL in seconds
        #[arg(long, conflicts_with = "clear_ttl")]
        ttl: Option<i64>,

        /// Remove TTL (secret will never expire)
        #[arg(long, conflicts_with = "ttl")]
        clear_ttl: bool,

        /// New source command (sh -c)
        #[arg(long, conflicts_with = "source_cmd")]
        source_sh: Option<String>,

        /// New source command (direct)
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
