#!/bin/sh
set -eu
root=$(mktemp -d)
trap 'rm -rf -- "$root"' EXIT
export OPDEV_DATA_DIR="$root/data with spaces"
bootstrap="$(dirname "$0")/../plugins/opdev/scripts/runtime.sh"
binary=$(sh "$bootstrap" --install)
sh "$bootstrap" --run version
[ "$(sh "$bootstrap" --install)" = "$binary" ]
[ "$(sh "$bootstrap" --path)" = "$binary" ]
