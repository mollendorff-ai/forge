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
