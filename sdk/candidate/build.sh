#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")"

# Cargo.toml follows the official stable-mod template and expects the SDK at ../mod-api-stable.
if [ ! -f ../mod-api-stable/Cargo.toml ]; then
  sdk=""
  if [ -n "${TFM2_STABLE_SDK_DIR:-}" ]; then
    if [ -f "${TFM2_STABLE_SDK_DIR}/Cargo.toml" ]; then
      sdk="$TFM2_STABLE_SDK_DIR"
    elif [ -f "${TFM2_STABLE_SDK_DIR}/mod-api-stable/Cargo.toml" ]; then
      sdk="${TFM2_STABLE_SDK_DIR}/mod-api-stable"
    fi
  fi
  if [ -z "$sdk" ] && [ -f ../../mod-sdk-stable/mod-api-stable/Cargo.toml ]; then
    sdk="../../mod-sdk-stable/mod-api-stable"
  fi
  if [ -z "$sdk" ]; then
    echo "Stable SDK not found. Install mod-api-stable at ../mod-api-stable or set TFM2_STABLE_SDK_DIR." >&2
    exit 1
  fi
  ln -s "$sdk" ../mod-api-stable
fi

cargo build --release
case "$(uname -s)" in
  Linux) cp target/release/libmlbb_heroes.so ./mlbb_heroes.so ;;
  Darwin) cp target/release/libmlbb_heroes.dylib ./mlbb_heroes.dylib ;;
  *) echo "Use build.ps1 on Windows." >&2; exit 1 ;;
esac
