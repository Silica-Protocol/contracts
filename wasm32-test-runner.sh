#!/usr/bin/env bash
set -euo pipefail

if command -v wasm-bindgen-test-runner >/dev/null 2>&1; then
    exec wasm-bindgen-test-runner "$@"
fi

cat >&2 <<'EOF'
error: wasm-bindgen-test-runner not found in PATH.
Install it with:
    cargo install wasm-bindgen-cli
Then rerun your tests (e.g. cargo test --target wasm32-unknown-unknown).
EOF
exit 127
