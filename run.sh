#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")"
# On Nix-managed Rust installations, dlopen may not search the host's graphics
# libraries. This affects the app process only; normal installations need no fix.
if [ "$(uname -s)" = Linux ] && [ -f /usr/lib/x86_64-linux-gnu/libX11.so.6 ]; then
    export LD_LIBRARY_PATH="${LD_LIBRARY_PATH:+$LD_LIBRARY_PATH:}/usr/lib/x86_64-linux-gnu"
fi
exec cargo run --release --bin local-rift -- "$@"
