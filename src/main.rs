mod cli;
mod error;
mod index;
mod model;
mod source;
mod store;

use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::generate;
use tracing::debug;
use tracing_subscriber::EnvFilter;

use crate::cli::Cli;
use crate::cli::Command;
use crate::error::HemliError;
use crate::model::SourceType;
use crate::model::StoredSecret;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Get {
            namespace,
            secret,
            force_refresh,
            no_refresh,
            no_store,
            ttl,
            source_sh,
            source_cmd,
        } => cmd_get(
            &namespace,
            &secret,
            force_refresh,
            no_refresh,
            no_store,
            ttl,
            source_sh,
            source_cmd,
        )?,
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "hemli", &mut std::io::stdout());
        }
        Command::Delete { namespace, secret } => cmd_delete(&namespace, &secret)?,
        Command::List { namespace } => cmd_list(namespace.as_deref())?,
        Command::Inspect { namespace, secret } => cmd_inspect(&namespace, &secret)?,
        Command::Edit {
            namespace,
            secret,
            ttl,
            clear_ttl,
            source_sh,
            source_cmd,
        } => cmd_edit(&namespace, &secret, ttl, clear_ttl, source_sh, source_cmd)?,
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_get(
    namespace: &str,
    secret: &str,
    force_refresh: bool,
    no_refresh: bool,
    no_store: bool,
    ttl: Option<i64>,
    source_sh: Option<String>,
    source_cmd: Option<String>,
) -> Result<()> {
    let existing = store::get_secret(namespace, secret)?;

    if no_refresh {
        match existing {
            Some(entry) => {
                print!("{}", entry.value);
                return Ok(());
            }
            None => {
                return Err(HemliError::NotFound {
                    namespace: namespace.to_string(),
                    secret: secret.to_string(),
                }
                .into());
            }
        }
    }

    let needs_refresh =
        force_refresh || existing.is_none() || existing.as_ref().is_some_and(|e| e.is_expired());

    if !needs_refresh {
        let entry = existing.unwrap();
        debug!("returning cached secret");
        print!("{}", entry.value);
        return Ok(());
    }

    // Determine source: CLI args take priority, fall back to stored source
    let (cmd_str, src_type) = if let Some(ref sh) = source_sh {
        (sh.clone(), SourceType::Sh)
    } else if let Some(ref cmd) = source_cmd {
        (cmd.clone(), SourceType::Cmd)
    } else if let Some(ref entry) = existing {
        match (&entry.source_command, &entry.source_type) {
            (Some(cmd), Some(st)) => (cmd.clone(), *st),
            _ => return Err(HemliError::NoSource.into()),
        }
    } else {
        return Err(HemliError::NoSource.into());
    };

    debug!(command = %cmd_str, source_type = ?src_type, "fetching secret from source");
    let value = source::fetch_secret(&cmd_str, &src_type)?;

    // Determine TTL: CLI arg takes priority, fall back to existing entry's TTL
    let effective_ttl = ttl.or_else(|| existing.as_ref().and_then(|e| e.ttl_seconds));

    let stored = StoredSecret::new(value.clone(), Some(cmd_str), Some(src_type), effective_ttl);

    if !no_store {
        store::set_secret(namespace, secret, &stored)?;

        let idx_path = index::index_path();
        let mut idx = index::load_index(&idx_path)?;
        index::upsert_entry(&mut idx, namespace, secret, stored.created_at);
        index::save_index(&idx_path, &idx)?;

        debug!("stored secret in keyring and index");
    }

    print!("{}", value);
    Ok(())
}

fn cmd_delete(namespace: &str, secret: &str) -> Result<()> {
    store::delete_secret(namespace, secret)?;

    let idx_path = index::index_path();
    let mut idx = index::load_index(&idx_path)?;
    index::remove_entry(&mut idx, namespace, secret);
    index::save_index(&idx_path, &idx)?;

    eprintln!("Deleted secret '{secret}' from namespace '{namespace}'");
    Ok(())
}

fn cmd_inspect(namespace: &str, secret: &str) -> Result<()> {
    let entry = store::get_secret(namespace, secret)?;
    match entry {
        Some(stored) => {
            let json = serde_json::to_string_pretty(&stored)?;
            println!("{json}");
            Ok(())
        }
        None => Err(HemliError::NotFound {
            namespace: namespace.to_string(),
            secret: secret.to_string(),
        }
        .into()),
    }
}

fn cmd_edit(
    namespace: &str,
    secret: &str,
    ttl: Option<i64>,
    clear_ttl: bool,
    source_sh: Option<String>,
    source_cmd: Option<String>,
) -> Result<()> {
    if ttl.is_none() && !clear_ttl && source_sh.is_none() && source_cmd.is_none() {
        return Err(HemliError::NoModifications.into());
    }

    let mut stored = store::get_secret(namespace, secret)?.ok_or_else(|| HemliError::NotFound {
        namespace: namespace.to_string(),
        secret: secret.to_string(),
    })?;

    if clear_ttl {
        stored.ttl_seconds = None;
        stored.recalculate_expires_at();
    } else if let Some(ttl) = ttl {
        stored.ttl_seconds = Some(ttl);
        stored.recalculate_expires_at();
    }

    if let Some(sh) = source_sh {
        stored.source_command = Some(sh);
        stored.source_type = Some(SourceType::Sh);
    } else if let Some(cmd) = source_cmd {
        stored.source_command = Some(cmd);
        stored.source_type = Some(SourceType::Cmd);
    }

    store::set_secret(namespace, secret, &stored)?;
    eprintln!("Updated secret '{secret}' in namespace '{namespace}'");
    Ok(())
}

fn cmd_list(namespace: Option<&str>) -> Result<()> {
    let idx_path = index::index_path();
    let idx = index::load_index(&idx_path)?;
    let entries = index::filter_entries(&idx, namespace);

    for entry in entries {
        println!(
            "{}\t{}\t{}",
            entry.namespace, entry.secret, entry.created_at
        );
    }

    Ok(())
}
