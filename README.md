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

### `hemli get`

Retrieve a secret, fetching from an external source if not cached or expired.

```
hemli get -n <namespace> <secret> [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-n, --namespace <NS>` | Namespace for the secret (required) |
| `--source-sh <CMD>` | Fetch command run via `sh -c` |
| `--source-cmd <CMD>` | Fetch command run directly (split on whitespace) |
| `--ttl <SECONDS>` | Cache TTL in seconds |
| `--force-refresh` | Refresh from source even if cached |
| `--no-refresh` | Only return cached value, error if not found |
| `--no-store` | Don't persist the fetched secret |

`--source-sh` and `--source-cmd` are mutually exclusive. `--force-refresh` and `--no-refresh` are mutually exclusive.

When a secret is stored with a source command, subsequent `get` calls will automatically re-fetch using the stored source when the secret expires -- no need to pass `--source-sh`/`--source-cmd` again.

### `hemli inspect`

Show full metadata for a cached secret as JSON.

```
hemli inspect -n <namespace> <secret>
```

Returns the stored JSON including `value`, `created_at`, `source_command`, `source_type`, `ttl_seconds`, and `expires_at`. Errors if the secret is not cached.

### `hemli edit`

Modify metadata of a cached secret without re-fetching from the source.

```
hemli edit -n <namespace> <secret> [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-n, --namespace <NS>` | Namespace for the secret (required) |
| `--ttl <SECONDS>` | Set a new TTL in seconds |
| `--clear-ttl` | Remove TTL (secret will never expire) |
| `--source-sh <CMD>` | Set a new source command run via `sh -c` |
| `--source-cmd <CMD>` | Set a new source command run directly |

At least one modification flag is required. `--ttl` and `--clear-ttl` are mutually exclusive. `--source-sh` and `--source-cmd` are mutually exclusive.

Editing the TTL recalculates the expiration time from the original `created_at` timestamp -- it does not reset the creation time.

### `hemli delete`

Remove a secret from the keyring.

```
hemli delete -n <namespace> <secret>
```

Deleting a non-existent secret is a no-op.

### `hemli list`

List cached secrets.

```
hemli list [-n <namespace>]
```

Output is tab-separated: `namespace\tsecret\tcreated_at`. If `-n` is provided, results are filtered to that namespace.

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
