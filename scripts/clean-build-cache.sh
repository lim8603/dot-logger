#!/usr/bin/env bash
# Free build space once a unit of work is done.
#
# When the filesystem holding CARGO_TARGET_DIR is above THRESHOLD percent used,
# remove build output that any later build recreates: the debug profile, the
# out-of-workspace consumer build and cross-target checks (`$CARGO_TARGET_DIR/cross`).
# Release binaries (performance scripts) and coverage output are kept. Nothing is removed while cargo or rustc is running,
# so a background build or regression is never broken.
#
# Usage: scripts/clean-build-cache.sh [--threshold PERCENT] [--dry-run]
set -euo pipefail

threshold=60
dry_run=0
while [ $# -gt 0 ]; do
  case "$1" in
    --threshold) threshold="$2"; shift 2 ;;
    --dry-run) dry_run=1; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

target="${CARGO_TARGET_DIR:-$(cd "$(dirname "$0")/.." && pwd)/target}"
if [ ! -d "$target" ]; then
  echo "no build directory at $target"
  exit 0
fi

used() { df --output=pcent "$target" | tail -1 | tr -dc '0-9'; }
before=$(used)
if [ "$before" -le "$threshold" ]; then
  echo "build disk ${before}% used (threshold ${threshold}%): nothing to do"
  exit 0
fi
if pgrep -x cargo >/dev/null || pgrep -x rustc >/dev/null; then
  echo "build disk ${before}% used, but cargo/rustc is running: skipped" >&2
  exit 3
fi

for dir in "$target/debug" "$target/external-consumer" "$target/cross"; do
  [ -e "$dir" ] || continue
  size=$(du -sh "$dir" 2>/dev/null | cut -f1)
  if [ "$dry_run" -eq 1 ]; then
    echo "would remove $dir ($size)"
  else
    rm -rf "$dir"
    echo "removed $dir ($size)"
  fi
done
echo "build disk ${before}% -> $(used)% used"
