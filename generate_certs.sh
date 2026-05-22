#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# generate_certs.sh — generate a self-signed TLS certificate for testing
#
# Usage:
#   bash generate_certs.sh [OUTPUT_DIR]
#
#   OUTPUT_DIR defaults to /var/lib/offline-intelligence/tls  (server mode)
#   or ~/.local/share/OfflineIntelligence/tls               (desktop mode).
#   Override by passing a path:  bash generate_certs.sh /tmp/certs
#
# Output files:
#   server.crt  — self-signed certificate (valid 365 days)
#   server.key  — RSA-4096 private key
#
# After running this script, add these two lines to your .env:
#   TLS_CERT_PATH=<OUTPUT_DIR>/server.crt
#   TLS_KEY_PATH=<OUTPUT_DIR>/server.key
#
# NOTE: Self-signed certificates are suitable for internal testing only.
# For production (hospital / law firm networks) obtain a certificate from
# your organisation's CA or a public CA (e.g. Let's Encrypt via certbot).
# ---------------------------------------------------------------------------
set -euo pipefail

# ── Resolve output directory ────────────────────────────────────────────────
if [[ $# -ge 1 ]]; then
    OUT_DIR="$1"
elif [[ -d /var/lib/offline-intelligence ]]; then
    OUT_DIR="/var/lib/offline-intelligence/tls"
else
    OUT_DIR="${HOME}/.local/share/OfflineIntelligence/tls"
fi

mkdir -p "$OUT_DIR"
CERT="$OUT_DIR/server.crt"
KEY="$OUT_DIR/server.key"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Offline Intelligence — TLS Certificate Generator"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Output directory : $OUT_DIR"
echo "  Certificate      : $CERT"
echo "  Private key      : $KEY"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ── Check for openssl ───────────────────────────────────────────────────────
if ! command -v openssl &>/dev/null; then
    echo "ERROR: openssl is not installed."
    echo ""
    echo "Install it with:"
    echo "  Ubuntu / Debian : sudo apt-get install openssl"
    echo "  RHEL / CentOS   : sudo yum install openssl"
    echo "  macOS           : brew install openssl"
    exit 1
fi

# ── Generate certificate ────────────────────────────────────────────────────
echo "Generating RSA-4096 private key and self-signed certificate (365 days)..."
echo ""

openssl req \
    -x509 \
    -newkey rsa:4096 \
    -keyout "$KEY" \
    -out    "$CERT" \
    -days   365 \
    -nodes \
    -subj   "/C=US/ST=State/L=City/O=OfflineIntelligence/CN=localhost" \
    -addext "subjectAltName=DNS:localhost,DNS:$(hostname -f 2>/dev/null || echo localhost),IP:127.0.0.1,IP:::1"

# ── Lock down permissions ───────────────────────────────────────────────────
chmod 600 "$KEY"   # private key readable only by owner
chmod 644 "$CERT"  # certificate is public

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅  Done. Certificate valid for 365 days."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Add these two lines to your .env:"
echo ""
echo "  TLS_CERT_PATH=$CERT"
echo "  TLS_KEY_PATH=$KEY"
echo ""
echo "Then restart the server. It will serve HTTPS on the configured port."
echo ""
echo "⚠️  Self-signed certificates will trigger browser warnings."
echo "   For production, replace with a certificate from your CA."
echo ""
