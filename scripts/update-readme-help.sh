#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
README="$REPO_ROOT/README.md"

BEGIN_MARKER="<!-- BEGIN GENERATED HELP -->"
END_MARKER="<!-- END GENERATED HELP -->"

# Build the binary and locate the executable
HEMLI=$(cargo build --message-format json 2>/dev/null \
  | jq -r 'select(.executable | . == null | not) | .executable')

if [[ -z "$HEMLI" ]]; then
  echo "ERROR: Could not find hemli executable after build" >&2
  exit 1
fi

# Unset HEMLI_* env vars to prevent them leaking into help output
for var in $(env | grep '^HEMLI_' | cut -d= -f1 || true); do
  unset "$var"
done

# Generate help content into a temp file
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

{
  echo '### `hemli`'
  echo ""
  echo '```'
  "$HEMLI" --help
  echo '```'

  # Document only the 'get' subcommand (the most complex one)
  echo ""
  echo '### `hemli get`'
  echo ""
  echo '```'
  "$HEMLI" get --help
  echo '```'
} > "$TMPFILE"

# Replace content between markers in README.md
awk -v begin="$BEGIN_MARKER" -v end="$END_MARKER" -v content_file="$TMPFILE" '
  $0 == begin {
    print
    print ""
    while ((getline line < content_file) > 0) print line
    print ""
    skip = 1
    next
  }
  $0 == end {
    skip = 0
    print
    next
  }
  !skip { print }
' "$README" > "$README.tmp"

mv "$README.tmp" "$README"
