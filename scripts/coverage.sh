#!/usr/bin/env bash
# Coverage of everything outside tests/, measured the way CI and `make
# coverage` measure it: the tests, then the built program run as a user runs
# it, for what only the real process does (main, and MCP over stdin and
# stdout). Every run adds to one report.
#
#   scripts/coverage.sh [report arguments...]
#
# e.g. scripts/coverage.sh --json --output-path cov.json
#      scripts/coverage.sh --fail-under-lines 100 --fail-under-regions 100
set -euo pipefail

# The toolchain counts: Rust 1.100 and later count the regions inside macros
# that older ones (1.97, the oldest supported) do not, so an older toolchain
# reads 100% where CI's stable does not. CI measures with the latest stable;
# this refuses to measure with anything older than 1.100, so that a local run
# measures what CI measures. To pick one:
#
#   RUSTUP_TOOLCHAIN=1.100.0 scripts/coverage.sh ...
#
# (needs `rustup toolchain install 1.100.0 --component llvm-tools-preview`)
min_minor=100
minor=$(rustc --version | sed -E 's/^rustc 1\.([0-9]+)\..*/\1/')
if ! [ "$minor" -ge "$min_minor" ] 2>/dev/null; then
  echo "coverage needs Rust 1.$min_minor or later, as CI's stable is; this is $(rustc --version)." >&2
  echo "Run it with RUSTUP_TOOLCHAIN=1.$min_minor.0 (or a later one) set." >&2
  exit 1
fi

cargo llvm-cov clean --workspace
cargo llvm-cov --no-report

# The program itself: its version, and an MCP session that initializes, lists
# the tools and leaves (the server answers before stdin closes)
cargo llvm-cov run --no-report --bin forge -- --version
mcp_session() {
  printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"coverage","version":"1.0"}}}'
  sleep 0.5
  printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
  printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
  sleep 1
}
tools=$(mcp_session | cargo llvm-cov run --no-report --bin forge -- mcp |
  jq -r 'select(.result.tools) | .result.tools | length')
if [ "$tools" != "23" ]; then
  echo "forge mcp listed ${tools:-no} tools, not 23" >&2
  exit 1
fi

cargo llvm-cov report --ignore-filename-regex 'tests/' "$@"
