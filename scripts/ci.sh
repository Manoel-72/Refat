#!/usr/bin/env bash
# CI local (Linux/macOS): mesmo check basico que o workflow do GitHub.
# Uso: na raiz do repo: bash scripts/ci.sh
# Ou: chmod +x scripts/ci.sh && ./scripts/ci.sh

set -euo pipefail
cd "$(dirname "$0")/.."

echo ">> cargo build"
cargo build --verbose

echo ">> cargo test"
cargo test --verbose
