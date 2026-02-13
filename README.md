# hemli

A secret management CLI tool for local development. hemli caches secrets in the OS-native keyring and fetches them on-demand from external providers via shell commands.

## Installation

### With mise (recommended)

```sh
mise use -g github:glennib/hemli
```

### With cargo-binstall

```sh
cargo binstall hemli-cli
```

### With mise from crates.io

```sh
mise use -g cargo:hemli-cli
```

### From source

```sh
cargo install --path .
```

## Quick start

```sh
# Fetch a secret and cache it
hemli get -n myapp db_password --source-sh "vault kv get -field=password secret/myapp/db"

# Retrieve cached secret (no external call)
hemli get -n myapp db_password

# List all cached secrets
hemli list

# Inspect a cached secret's metadata
hemli inspect -n myapp db_password

# Edit metadata of a cached secret
hemli edit -n myapp db_password --ttl 7200

# Delete a cached secret
hemli delete -n myapp db_password
```

## Commands

<!-- BEGIN GENERATED HELP -->

### `hemli`

```
Secret management CLI for local development

hemli caches secrets in the OS-native keyring and fetches them on-demand from external providers via shell commands. Secrets are organized by namespace and automatically re-fetched when their TTL expires.

Usage: hemli <COMMAND>

Commands:
  get          Get a secret, fetching from source if needed
  delete       Delete a secret from the keyring
  list         List stored secrets
  inspect      Inspect a cached secret, showing full metadata as JSON
  edit         Edit metadata of a cached secret (TTL, source command)
  completions  Generate shell completion scripts
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')
```

### `hemli get`

```
Get a secret, fetching from source if needed

Checks the keyring cache first. If the secret is missing or expired, fetches it from the source command, stores the result in the keyring, and prints the value to stdout.

When a secret is stored with a source command, subsequent calls will automatically re-fetch using that stored source when the TTL expires -- no need to pass --source-sh/--source-cmd again.

Usage: hemli get [OPTIONS] --namespace <NAMESPACE> <SECRET>

Arguments:
  <SECRET>
          Name of the secret
          
          The identifier for this secret within its namespace. Used as the keyring account name.

Options:
  -n, --namespace <NAMESPACE>
          Namespace for the secret
          
          Groups secrets by project or environment. The keyring service name is "hemli:<namespace>", so secrets in different namespaces are fully isolated.
          
          [env: HEMLI_NAMESPACE=]

      --force-refresh
          Force refresh from source even if cached
          
          Re-fetches the secret from the source command regardless of whether a valid cached value exists. Mutually exclusive with --no-refresh.
          
          [env: HEMLI_FORCE_REFRESH=]

      --no-refresh
          Only return cached value, never refresh
          
          Returns the cached secret even if expired. Errors if no cached value exists. Mutually exclusive with --force-refresh.
          
          [env: HEMLI_NO_REFRESH=]

      --no-store
          Don't store the fetched secret in keyring
          
          Fetches and prints the secret but does not persist it in the keyring or update the index. Useful for one-off lookups.
          
          [env: HEMLI_NO_STORE=]

      --ttl <TTL>
          TTL in seconds for the cached secret
          
          Sets how long the cached secret is considered valid. After this duration, the next get call will re-fetch from the source. If omitted, falls back to the TTL stored with the existing cached secret, or no expiration if none was ever set.

      --source-sh <SOURCE_SH>
          Source command to run via sh -c
          
          The command string is passed to the system shell as "sh -c <CMD>". Supports pipes, redirects, and shell syntax. Mutually exclusive with --source-cmd.

      --source-cmd <SOURCE_CMD>
          Source command to run directly
          
          The command string is split on whitespace and executed directly without a shell. Use this when you don't need shell features. Mutually exclusive with --source-sh.

  -h, --help
          Print help (see a summary with '-h')
```

<!-- END GENERATED HELP -->

## Stored secret format

Secrets are stored as JSON in the OS keyring under the service name `hemli:<namespace>`:

```json
{
  "value": "the-secret-value",
  "created_at": "2025-01-15T10:30:00Z",
  "source_command": "vault kv get -field=password secret/myapp/db",
  "source_type": "sh",
  "ttl_seconds": 3600,
  "expires_at": "2025-01-15T11:30:00Z"
}
```

## Namespacing

Namespaces let you group secrets by project or environment. The keyring service name is `hemli:<namespace>`, so secrets in different namespaces are fully isolated.

```sh
hemli get -n project-a api_key --source-sh "..."
hemli get -n project-b api_key --source-sh "..."  # independent secret
```

## Provider examples

### Google Cloud Secret Manager

```sh
hemli get -n myapp db_password \
  --source-sh "gcloud secrets versions access latest --secret=db-password --project=my-project" \
  --ttl 3600
```

### AWS Secrets Manager

```sh
hemli get -n myapp api_key \
  --source-sh "aws secretsmanager get-secret-value --secret-id my-secret --query SecretString --output text" \
  --ttl 3600
```

### HashiCorp Vault

```sh
hemli get -n myapp db_password \
  --source-sh "vault kv get -field=password secret/myapp/db" \
  --ttl 1800
```

### 1Password CLI

```sh
hemli get -n myapp api_token \
  --source-sh "op read 'op://vault/item/field'" \
  --ttl 7200
```

### Environment variable passthrough

```sh
hemli get -n myapp some_secret --source-sh "printenv SOME_SECRET"
```

## Development

```sh
# Format
cargo fmt

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Run unit tests
cargo nextest run

# Run integration tests (requires OS keyring access)
cargo nextest run -- --ignored

# Full CI pipeline
mise run ci
```

## Logging

hemli uses `tracing` for internal logging, output to stderr. Set the `RUST_LOG` environment variable to control verbosity:

```sh
RUST_LOG=debug hemli get -n myapp secret_name
```

## License

MIT OR Apache-2.0
