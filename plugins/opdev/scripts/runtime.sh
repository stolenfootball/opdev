#!/bin/sh
# Managed OpDev CLI installation. Only --install accesses the network.
set -eu
umask 077
fail() { printf 'OpDev setup: %s\n' "$*" >&2; exit 1; }
plugin_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
lock="$plugin_root/runtime.lock"
field() { awk -v key="$1" '$1 == key {print $2}' "$lock"; }
version=$(field version)
tag=$(field tag)
identity=$(field identity)
cosign_version=$(field cosign)
[ -n "$version" ] && [ "$tag" = "v$version" ] || fail 'Invalid runtime version lock.'
case "$tag" in *[!a-zA-Z0-9.-]*) fail 'Invalid release tag.';; esac
mode=${1:---path}
[ "$#" -eq 0 ] || shift
case "$mode" in --path|--install) [ "$#" -eq 0 ] || fail 'Unexpected arguments.';; --run) ;; *) fail 'Usage: runtime.sh [--path | --install | --run ARGS...]';; esac
os=$(uname -s)
arch=$(uname -m)
case "$os" in Darwin|Linux) ;; *) fail 'Unsupported OS. On Windows use runtime.ps1.';; esac
row=$(awk -v os="$os" -v arch="$arch" '$1 == "target" && $2 == os && $3 == arch {print $4, $5, $6}' "$lock")
[ -n "$row" ] || fail "Unsupported platform: $os $arch."
# The lock contains only fixed, reviewed words; do not evaluate it as shell code.
triple=$(printf '%s\n' "$row" | awk '{print $1}')
verifier_asset=$(printf '%s\n' "$row" | awk '{print $2}')
verifier_digest=$(printf '%s\n' "$row" | awk '{print $3}')
case "$triple" in *[!a-zA-Z0-9_-]*) fail 'Invalid target lock.';; esac
data=${OPDEV_DATA_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/opdev}
case "$data" in /*) ;; *) fail 'Runtime data directory must be absolute.';; esac
destination="$data/runtimes/$tag/$triple"
binary="$destination/opdev"
hash() {
  if command -v sha256sum >/dev/null 2>&1; then raw_digest=$(sha256sum "$1") || return 1
  elif command -v shasum >/dev/null 2>&1; then raw_digest=$(shasum -a 256 "$1") || return 1
  else fail 'SHA-256 requires sha256sum or shasum.'; fi
  digest=$(printf '%s\n' "$raw_digest" | awk '{print $1}')
  [ "${#digest}" = 64 ] || return 1
  printf '%s\n' "$digest"
}
valid_runtime() {
  [ ! -L "$destination" ] && [ ! -L "$binary" ] && [ -f "$binary" ] && [ -x "$binary" ] &&
  [ ! -L "$destination/opdev.sha256" ] && [ -f "$destination/opdev.sha256" ] && [ "$(hash "$binary")" = "$(cat "$destination/opdev.sha256")" ]
}
if [ "$mode" != --install ]; then
  if [ ! -e "$destination" ] && [ ! -L "$destination" ]; then
    printf 'OpDev setup: No managed runtime installed. Run runtime.sh --install.\n' >&2
    exit 3
  fi
  valid_runtime || fail 'Managed CLI is missing or damaged. Run runtime.sh --install to set it up.'
  if [ "$mode" = --path ]; then printf '%s\n' "$binary"; exit 0; fi
  "$binary" plugin verify --contract "$plugin_root/opdev-compatibility.json" >/dev/null || fail 'CLI/plugin compatibility check failed.'
  exec "$binary" "$@"
fi
if [ -e "$destination" ] || [ -L "$destination" ]; then
  valid_runtime || fail "Existing runtime is damaged; move aside this directory before retrying: $destination"
  "$binary" plugin verify --contract "$plugin_root/opdev-compatibility.json" >/dev/null || fail 'CLI/plugin compatibility check failed.'
  printf '%s\n' "$binary"
  exit 0
fi
for tool in curl tar mktemp awk; do command -v "$tool" >/dev/null 2>&1 || fail "Required OS utility is unavailable: $tool"; done
mkdir -p "$data/runtimes/$tag"
install_lock="$destination.lock"
mkdir "$install_lock" 2>/dev/null || fail "Another setup may be running. If it was interrupted, remove the stale lock: $install_lock"
staging=
cleanup() { [ -z "$staging" ] || rm -rf -- "$staging"; rmdir "$install_lock" 2>/dev/null || :; }
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
[ ! -e "$destination" ] && [ ! -L "$destination" ] || fail 'Runtime appeared during setup; retry.'
staging=$(mktemp -d "$data/runtimes/$tag/.install.XXXXXXXX")
download() { curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 --max-time 180 --output "$2" "$1"; }
verifier="$staging/cosign"
printf 'Downloading pinned signature verifier %s...\n' "$cosign_version" >&2
download "https://github.com/sigstore/cosign/releases/download/$cosign_version/$verifier_asset" "$verifier"
[ "$(hash "$verifier")" = "$verifier_digest" ] || fail 'Signature verifier checksum mismatch.'
chmod 700 "$verifier"
archive="opdev-$version-$triple.tar.gz"
base="https://gitlab.com/stolenfootball-tools/opdev/-/releases/$tag/downloads"
printf 'Downloading and verifying OpDev %s for %s...\n' "$version" "$triple" >&2
download "$base/$archive" "$staging/$archive"
download "$base/$archive.sigstore.json" "$staging/$archive.sigstore.json"
# Verify the signed bytes before reading or extracting the archive.
"$verifier" verify-blob "$staging/$archive" --bundle "$staging/$archive.sigstore.json" --certificate-identity "$identity" --certificate-oidc-issuer https://gitlab.com >&2
# Extract only the expected regular executable; never unpack arbitrary entries.
entry=$(tar -tzf "$staging/$archive" | awk '$0 == "opdev" {n++} END {print n+0}')
[ "$entry" = 1 ] || fail 'Archive must contain exactly one opdev entry.'
tar -tvzf "$staging/$archive" opdev | awk 'substr($0,1,1) != "-" {exit 1}' || fail 'CLI archive entry is not a regular file.'
mkdir "$staging/runtime"
tar -xOzf "$staging/$archive" opdev > "$staging/runtime/opdev"
chmod 700 "$staging/runtime/opdev"
version_output=$("$staging/runtime/opdev" version)
[ "$(printf '%s\n' "$version_output" | sed -n '1p')" = "opdev $version" ] || fail 'Downloaded CLI version does not match the pin.'
"$staging/runtime/opdev" plugin verify --contract "$plugin_root/opdev-compatibility.json" >&2
hash "$staging/runtime/opdev" > "$staging/runtime/opdev.sha256"
mv "$staging/runtime" "$destination"
printf '%s\n' "$binary"
