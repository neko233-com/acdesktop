#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${ACDESKTOP_E2E_BINARY:?ACDESKTOP_E2E_BINARY is required}"

export ACDESKTOP_E2E_INHERIT_LEASE=1
exec python3 "$here/json_tool.py" exec "$ACDESKTOP_E2E_BINARY" "$@"
