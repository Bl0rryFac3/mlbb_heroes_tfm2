#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")"
cargo build --release
case "$(uname -s)" in
  Linux) cp target/release/libmlbb_heroes.so ./mlbb_heroes.so ;;
  Darwin) cp target/release/libmlbb_heroes.dylib ./mlbb_heroes.dylib ;;
  *) echo "Use build.ps1 on Windows." >&2; exit 1 ;;
esac
