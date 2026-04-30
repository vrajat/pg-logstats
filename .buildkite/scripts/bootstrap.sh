#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required on the Buildkite agent" >&2
  exit 1
fi

if command -v rustup >/dev/null 2>&1; then
  rustup component add rustfmt clippy
fi

cargo --version
rustc --version
