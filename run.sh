#!/usr/bin/env bash
# run.sh - Launcher for Brainwash on NixOS / Linux
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

# If already inside a nix-shell or if LD_LIBRARY_PATH already contains libGL, run cargo directly;
# otherwise launch via nix-shell with the required graphics and windowing libraries.
if [[ -n "${IN_NIX_SHELL:-}" ]] || { command -v cargo >/dev/null 2>&1 && [[ -n "${LD_LIBRARY_PATH:-}" ]]; }; then
    exec cargo run -p mybrain-app --bin brainwash -- "$@"
else
    exec nix-shell --run "cargo run -p mybrain-app --bin brainwash -- \"\$@\""
fi
