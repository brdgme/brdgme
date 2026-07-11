#!/usr/bin/env bash
# Send a JSON request (on stdin) to a game CLI binary, extract a render
# field with a jq filter, and render the markup to plain text with player
# names substituted.
#
# Usage:
#   echo '<request-json>' | scripts/render-compare/render.sh <cli-binary> <jq-filter> [player-name...]
#
# Example (category-5, Go, player 0's render, players named Alice/Bob):
#   echo '{"New":{"players":2}}' | \
#     scripts/render-compare/render.sh /tmp/render-parity/category_5_go \
#     '.New.player_renders[0].render' Alice Bob
#
# Works for both Go and Rust CLI binaries - both emit the same {{...}} markup
# tag syntax, so the same render_plain helper handles both.
set -euo pipefail

if [ $# -lt 2 ]; then
  echo "Usage: render.sh <cli-binary> <jq-filter> [player-name...]" >&2
  exit 1
fi

bin="$1"
filter="$2"
shift 2

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
render_plain="$script_dir/../../rust/target/debug/render_plain"

if [ ! -x "$render_plain" ]; then
  echo "render_plain not built - run: cd rust && cargo build --package brdgme_render_plain" >&2
  exit 1
fi

"$bin" | jq -r "$filter" | "$render_plain" "$@"
