#!/usr/bin/env bash
# Optional, toolchain-gated C# consumer round-trip (task 6.2).
#
# Builds nothing Rust-side itself — it expects the cdylib to already exist
# (`cargo build --release -p nordocs-ffi`) and a `dotnet` SDK on PATH. Skips
# cleanly (exit 0) when either is missing, so it is safe to call from a
# best-effort hook; pass --strict to fail instead.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# HERE = <repo>/crates/nordocs-ffi/bindings/consumer-test -> repo root is up 4.
ROOT="$(cd "$HERE/../../../.." && pwd)"

STRICT=0
[ "${1:-}" = "--strict" ] && STRICT=1

skip() {
  echo "consumer-test: SKIP — $1" >&2
  [ "$STRICT" -eq 1 ] && exit 1
  exit 0
}

command -v dotnet >/dev/null 2>&1 || skip "no dotnet SDK on PATH"

case "$(uname -s)" in
  Darwin) LIB="$ROOT/target/release/libnordocs.dylib" ;;
  Linux)  LIB="$ROOT/target/release/libnordocs.so" ;;
  *)      LIB="$ROOT/target/release/nordocs.dll" ;;
esac

[ -f "$LIB" ] || skip "cdylib not found at $LIB (run: cargo build --release -p nordocs-ffi)"

exec dotnet run --project "$HERE" -c Release "-p:NordocsNativeLib=$LIB"
