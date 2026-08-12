#!/bin/sh
set -eu

if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
  exec "$@"
fi

for rust_bin in "${HOME}/.cargo/bin" "/opt/homebrew/opt/rustup/bin" "/usr/local/opt/rustup/bin"; do
  if [ -x "${rust_bin}/cargo" ] && [ -x "${rust_bin}/rustc" ]; then
    PATH="${rust_bin}:${PATH}" exec "$@"
  fi
done

echo "Rust is required. Install it with rustup, then retry." >&2
exit 1
