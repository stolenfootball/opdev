# OpDev verification extension for cargo-dist 0.32.0 (Apache-2.0).
# The release generator substitutes reviewed pins and the exact signing identity.
opdev_verify_archive() (
    set -eu
    umask 077
    archive=$1
    archive_url=$2
    case "$archive_url" in https://*) ;; *) echo 'OpDev requires HTTPS.' >&2; exit 1;; esac
    verification_dir=$(mktemp -d)
    trap 'rm -rf "$verification_dir"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    case "$(uname -s)/$(uname -m)" in
@VERIFIER_CASES@
        *) echo 'Unsupported signature-verifier platform.' >&2; exit 1;;
    esac
    download_verified_input() {
        curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' --tlsv1.2 --connect-timeout 15 --max-time 180 --output "$2" "$1"
    }
    verifier="$verification_dir/cosign"
    download_verified_input "https://github.com/sigstore/cosign/releases/download/@COSIGN_VERSION@/$verifier_asset" "$verifier"
    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$verifier")
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$verifier")
    else
        echo 'SHA-256 verification requires sha256sum or shasum.' >&2; exit 1
    fi
    actual=$(printf '%s\n' "$actual" | awk '{print $1}')
    [ "$actual" = "$verifier_digest" ] || { echo 'Signature verifier checksum mismatch.' >&2; exit 1; }
    chmod 700 "$verifier"
    download_verified_input "$archive_url.sigstore.json" "$verification_dir/bundle.json"
    "$verifier" verify-blob "$archive" --bundle "$verification_dir/bundle.json" --certificate-identity '@IDENTITY@' --certificate-oidc-issuer https://gitlab.com >&2
)
