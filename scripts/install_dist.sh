#!/bin/sh
# CI-only, pinned cargo-dist installation. Consumer installers never require Rust.
set -eu
destination=$1
[ ! -e "$destination" ] || { echo 'Refusing to overwrite cargo-dist tools.' >&2; exit 1; }
mkdir -p "$destination"
archive="$destination/dist.tar.xz"
curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 --max-time 180 https://github.com/axodotdev/cargo-dist/releases/download/v0.32.0/cargo-dist-x86_64-unknown-linux-gnu.tar.xz --output "$archive"
printf '%s  %s\n' eb52f9fae0d0506774e9f1801c1168f87fa2c87a45e2d64d3ae7c89401929946 "$archive" | sha256sum --check
tar -xJf "$archive" -C "$destination"
"$destination/cargo-dist-x86_64-unknown-linux-gnu/dist" --version
