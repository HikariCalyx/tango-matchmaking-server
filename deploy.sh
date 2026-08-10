#!/usr/bin/env bash
#
# Trill Signaling Server — One-shot VPS deploy script
# =====================================================
# Usage:
#   sudo bash deploy.sh
#
# This script bootstraps a fresh Linux VPS (amd64 / arm64) with:
#   • coturn (TURN server)
#   • trilld  (Trill signaling server binary)
#   • systemd service
#   • basic firewall rules
# ------------------------------------------------------------------

set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# ── 1. OS check (Linux only) ───────────────────────────────────
if [[ "$(uname -s)" != "Linux" ]]; then
    err "This script only supports Linux. Detected: $(uname -s). Aborting."
    exit 1
fi

# ── 2. Architecture check ───────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)  BIN_ARCH="amd64" ;;
    aarch64) BIN_ARCH="arm64" ;;
    *)
        err "Unsupported architecture: $ARCH. Only amd64 (x86_64) and arm64 (aarch64) are supported."
        exit 1
        ;;
esac
info "Detected architecture: $ARCH → binary arch: $BIN_ARCH"

# ── 3. Root check ───────────────────────────────────────────────
if [[ "$EUID" -ne 0 ]]; then
    err "This script must be run as root."
    echo "Please download the script and run with:"
    echo "  sudo bash deploy.sh"
    exit 1
fi

# ── 4. Detect distro ────────────────────────────────────────────
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO_ID="${ID}"
else
    err "Cannot detect Linux distribution (/etc/os-release missing). Aborting."
    exit 1
fi
info "Detected distribution: $NAME"

# ── 5. Install base dependencies ───────────────────────────────
# Fresh VPS images often lack curl, openssl, xz-utils, etc.
info "Installing base dependencies (curl, openssl, xz-utils, ca-certificates, tar)..."

case "$DISTRO_ID" in
    ubuntu|debian|deepin|uos)
        apt-get update -qq
        apt-get install -y -qq curl openssl xz-utils ca-certificates tar
        ;;
    centos|rhel|fedora|rocky|almalinux)
        if command -v dnf &>/dev/null; then
            dnf install -y curl openssl xz ca-certificates tar
        else
            yum install -y curl openssl xz ca-certificates tar
        fi
        ;;
    arch|manjaro)
        pacman -S --noconfirm curl openssl xz ca-certificates tar
        ;;
    alpine)
        apk add curl openssl xz ca-certificates tar
        ;;
    opensuse*|sles)
        zypper install -y curl openssl xz ca-certificates tar
        ;;
    *)
        err "Unsupported distribution: $DISTRO_ID. Cannot install dependencies automatically."
        err "Supported: Ubuntu, Debian, Deepin/UOS, CentOS, RHEL, Fedora, Rocky, AlmaLinux, Arch, Manjaro, Alpine, openSUSE."
        exit 1
        ;;
esac
info "Base dependencies installed."

# ── 6. Get server IP(s) ────────────────────────────────────────
# Cloudflare's trace endpoint returns both ip= and ipv6= fields; the
# "ip=" field is always present (IPv4 or IPv6), "ipv6=" is present
# when the request arrived via IPv6.
info "Querying server public IP via Cloudflare trace..."

CF_TRACE=$(curl -fsSL --max-time 10 "https://www.qualcomm.cn/cdn-cgi/trace" 2>/dev/null) || {
    err "Failed to reach Cloudflare trace endpoint. Check network connectivity."
    exit 1
}

# Extract IP from the "ip=" line
SERVER_IP_MAIN=$(echo "$CF_TRACE" | grep -E '^ip=' | cut -d= -f2)
# Extract optional "ipv6=" line
SERVER_IPV6=$(echo "$CF_TRACE" | grep -E '^ipv6=' | cut -d= -f2 || true)

# Determine IPv4 / IPv6
IPV4=""
IPV6=""
if [[ "$SERVER_IP_MAIN" =~ .*:.* ]]; then
    # Main is IPv6 → try to get IPv4 separately
    IPV6="$SERVER_IP_MAIN"
    info "Main IP appears to be IPv6: $IPV6"
    IPV4=$(curl -fsSL --max-time 10 -4 "https://www.qualcomm.cn/cdn-cgi/trace" 2>/dev/null | grep -E '^ip=' | cut -d= -f2 || true)
    if [[ -n "$IPV4" ]]; then
        info "Got IPv4 via -4 request: $IPV4"
    else
        warn "Could not obtain an IPv4 address — TURN will be IPv6-only."
    fi
else
    IPV4="$SERVER_IP_MAIN"
    info "Main IP appears to be IPv4: $IPV4"
    IPV6="${SERVER_IPV6:-}"
    if [[ -n "$IPV6" ]]; then
        info "Also detected IPv6: $IPV6"
    fi
fi

# ── 7. Install coturn ──────────────────────────────────────────
info "Installing coturn..."
case "$DISTRO_ID" in
    ubuntu|debian|deepin|uos)
        apt-get update -qq
        apt-get install -y -qq coturn
        ;;
    centos|rhel|fedora|rocky|almalinux)
        if command -v dnf &>/dev/null; then
            dnf install -y coturn
        else
            yum install -y coturn
        fi
        ;;
    arch|manjaro)
        pacman -S --noconfirm coturn
        ;;
    alpine)
        apk add coturn
        ;;
    opensuse*|sles)
        zypper install -y coturn
        ;;
    *)
        err "Unsupported distribution: $DISTRO_ID. Cannot install coturn automatically."
        err "Supported: Ubuntu, Debian, Deepin/UOS, CentOS, RHEL, Fedora, Rocky, AlmaLinux, Arch, Manjaro, Alpine, openSUSE."
        exit 1
        ;;
esac
info "coturn installed successfully."

# ── 8. Generate shared secret ──────────────────────────────────
info "Generating 128-character shared secret..."
# Use openssl for reliable random ASCII generation across platforms.
TURN_SECRET=$(openssl rand -base64 96 | tr -dc 'A-Za-z0-9!#$%&()*+,-./:;<=>?@[\]^_`{|}~' | head -c 128)
if [[ ${#TURN_SECRET} -lt 128 ]]; then
    # Fallback if openssl not available
    TURN_SECRET=$(tr -dc 'A-Za-z0-9!#$%&()*+,-./:;<=>?@[\]^_`{|}~' </dev/urandom 2>/dev/null | head -c 128)
fi
info "Shared secret generated (${#TURN_SECRET} chars)."

# ── 9. Create turnserver.conf ──────────────────────────────────
TURN_CONF="/etc/turnserver.conf"

# Build external-ip value
if [[ -n "$IPV4" ]] && [[ -n "$IPV6" ]]; then
    EXTERNAL_IP="${IPV4}/${IPV6}"
elif [[ -n "$IPV4" ]]; then
    EXTERNAL_IP="$IPV4"
else
    EXTERNAL_IP="$IPV6"
fi

# realm should be the IPv4 if available, otherwise whatever we have
if [[ -n "$IPV4" ]]; then
    REALM="$IPV4"
else
    REALM="$IPV6"
fi

cat > "$TURN_CONF" <<EOF
listening-port=3478
listening-ip=0.0.0.0
listening-ip=::
external-ip=${EXTERNAL_IP}
realm=${REALM}
lt-cred-mech
use-auth-secret
static-auth-secret=${TURN_SECRET}

syslog
no-rfc5780
no-stun-backward-compatibility
response-origin-only-with-rfc5780
EOF

info "Created $TURN_CONF"

# Enable and start coturn (if the service exists)
if systemctl list-unit-files coturn.service &>/dev/null; then
    systemctl enable coturn --now
    info "coturn service enabled and started."
else
    warn "coturn.service not found — you may need to start coturn manually."
fi

# ── 10. Download & install trilld binary ────────────────────────
BIN_URL_BASE="https://github.com/HikariCalyx/trill-matchmaking-server/releases/download/v5.1.0-260806-1918"
BIN_TARBALL="trill-signaling-server_${BIN_ARCH}-linux.xz"
BIN_URL="${BIN_URL_BASE}/${BIN_TARBALL}"

info "Downloading trilld binary for $BIN_ARCH..."
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

curl -fsSL --max-time 300 -o "$TMP_DIR/$BIN_TARBALL" "$BIN_URL" || {
    err "Failed to download binary from: $BIN_URL"
    exit 1
}

info "Extracting trilld..."
# xz → binary; some builds are .xz-compressed raw binary, not tar.xz
# Try both: raw xz first, then tar.xz
if xz -dc "$TMP_DIR/$BIN_TARBALL" > "$TMP_DIR/trilld" 2>/dev/null; then
    : # raw binary decompressed
else
    # Maybe it's a tar.xz?
    tar -xJf "$TMP_DIR/$BIN_TARBALL" -C "$TMP_DIR" || {
        err "Failed to extract binary. Ensure the release contains a valid xz-compressed file."
        exit 1
    }
    # Check if there's a trilld file somewhere in extracted contents
    if [[ ! -f "$TMP_DIR/trilld" ]]; then
        # Look for it
        FOUND=$(find "$TMP_DIR" -type f -name 'trilld' -o -name 'trill-signaling-server' | head -1)
        if [[ -n "$FOUND" ]]; then
            mv "$FOUND" "$TMP_DIR/trilld"
        else
            err "Could not find trilld binary in the extracted archive."
            exit 1
        fi
    fi
fi

install -m 755 "$TMP_DIR/trilld" /usr/local/bin/trilld
chmod +x /usr/local/bin/trilld
info "trilld installed to /usr/local/bin/trilld"

# ── 11. Determine server port ──────────────────────────────────
# Check if tcp/80 is already in use (multiple methods for portability)
PORT80_IN_USE=false
if command -v ss &>/dev/null; then
    ss -tlnp | grep -q ':80 ' && PORT80_IN_USE=true
elif command -v netstat &>/dev/null; then
    netstat -tlnp | grep -q ':80 ' && PORT80_IN_USE=true
elif command -v lsof &>/dev/null; then
    lsof -i :80 -sTCP:LISTEN &>/dev/null && PORT80_IN_USE=true
fi

if $PORT80_IN_USE; then
    SERVER_PORT=8000
    info "Port 80 is in use — will use port 8000 for the signaling server."
else
    SERVER_PORT=80
    info "Port 80 is free — will use port 80 for the signaling server."
fi

# ── 12. Create systemd service ─────────────────────────────────
# Build TURN_ADDR: prefer IPv4 for TURN, use <ip>:3478
if [[ -n "$IPV4" ]]; then
    TURN_ADDR="${IPV4}:3478"
else
    TURN_ADDR="${IPV6}:3478"
fi

SERVICE_FILE="/etc/systemd/system/trilld.service"

cat > "$SERVICE_FILE" <<EOF
# Contents of /etc/systemd/system/trilld.service
[Unit]
Description=Trill Signaling Server
After=network.target

[Service]
Type=simple
Environment=SERVER_PORT=${SERVER_PORT} USE_X_REAL_IP=true TURN_ADDR=${TURN_ADDR} TURN_CREDENTIAL_TTL=3600 TURN_SECRET=${TURN_SECRET}
ExecStart=/usr/local/bin/trilld
StandardOutput=append:/var/log/trilld.log
StandardError=append:/var/log/trilld_err.log

[Install]
WantedBy=multi-user.target
EOF

info "Created $SERVICE_FILE"

# ── 13. Enable and start the service ───────────────────────────
systemctl daemon-reload
systemctl enable trilld --now
info "trilld service enabled and started."

# ── 14. Configure firewall ─────────────────────────────────────
info "Configuring firewall rules..."

configure_firewall() {
    local tcp_ports=("$SERVER_PORT" "3478")
    local udp_ports=("3478")
    # UDP 49152-65535 is a range
    local udp_range_start=49152
    local udp_range_end=65535

    # iptables
    if command -v iptables &>/dev/null; then
        info "Using iptables..."
        for port in "${tcp_ports[@]}"; do
            iptables -I INPUT -p tcp --dport "$port" -j ACCEPT 2>/dev/null || true
        done
        for port in "${udp_ports[@]}"; do
            iptables -I INPUT -p udp --dport "$port" -j ACCEPT 2>/dev/null || true
        done
        # UDP TURN relay range
        iptables -I INPUT -p udp --dport ${udp_range_start}:${udp_range_end} -j ACCEPT 2>/dev/null || true
        # Save rules for persistence
        if command -v iptables-save &>/dev/null; then
            if [ -d /etc/iptables ]; then
                iptables-save > /etc/iptables/rules.v4 2>/dev/null || true
            fi
        fi
    fi

    # ufw
    if command -v ufw &>/dev/null && ufw status | grep -q 'Status: active'; then
        info "Using ufw..."
        for port in "${tcp_ports[@]}"; do
            ufw allow "$port/tcp" 2>/dev/null || true
        done
        for port in "${udp_ports[@]}"; do
            ufw allow "$port/udp" 2>/dev/null || true
        done
        ufw allow ${udp_range_start}:${udp_range_end}/udp 2>/dev/null || true
    fi

    # firewalld
    if command -v firewall-cmd &>/dev/null && systemctl is-active --quiet firewalld 2>/dev/null; then
        info "Using firewalld..."
        for port in "${tcp_ports[@]}"; do
            firewall-cmd --permanent --add-port="$port/tcp" 2>/dev/null || true
        done
        for port in "${udp_ports[@]}"; do
            firewall-cmd --permanent --add-port="$port/udp" 2>/dev/null || true
        done
        firewall-cmd --permanent --add-port=${udp_range_start}-${udp_range_end}/udp 2>/dev/null || true
        firewall-cmd --reload 2>/dev/null || true
    fi
}

configure_firewall

info "Firewall rules applied (if a supported firewall was detected)."

# ── Done ───────────────────────────────────────────────────────
echo ""
echo "============================================================"
echo -e "${GREEN}  Trill Signaling Server deployment complete!${NC}"
echo "============================================================"
echo ""
echo "  Server port : ${SERVER_PORT}"
echo "  TURN addr   : ${TURN_ADDR}"
echo "  Binary      : /usr/local/bin/trilld"
echo "  Service     : trilld.service"
echo ""
echo -e "${YELLOW}⚠  IMPORTANT: Also open these ports in your VPS provider's${NC}"
echo -e "${YELLOW}   firewall / security-group control panel:${NC}"
echo ""
echo "    TCP : ${SERVER_PORT}, 3478"
echo "    UDP : 3478, 49152-65535"
echo ""
echo "  Check service status:"
echo "    systemctl status trilld"
echo ""
echo "  View logs:"
echo "    journalctl -u trilld -f"
echo "    tail -f /var/log/trilld.log"
echo ""
echo "  Your matchmaking server address is:"
echo "    ws://${TURN_ADDR}:${SERVER_PORT}"
echo "============================================================"
