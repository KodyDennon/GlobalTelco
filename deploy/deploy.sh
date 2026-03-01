#!/usr/bin/env bash
set -euo pipefail

# ── GlobalTelco Oracle Cloud Deploy Script ─────────────────────────────────
#
# Usage:
#   ORACLE_IP=<ip> ./deploy/deploy.sh              # Full deploy (build + upload + install)
#   ORACLE_IP=<ip> ./deploy/deploy.sh --upload-only # Skip build, just upload + restart
#   ORACLE_IP=<ip> ./deploy/deploy.sh --setup-only  # First-time server setup (no binary)
#   ORACLE_IP=<ip> ./deploy/deploy.sh --force-env   # Replace server .env with local version
#
# Required env vars:
#   ORACLE_IP    — Public IP of your Oracle Cloud instance
#
# Optional env vars:
#   SSH_KEY      — Path to SSH private key (default: ~/.ssh/oracle_globaltelco)
#   SSH_USER     — SSH username (default: ubuntu)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_DIR/target/x86_64-unknown-linux-gnu/release/gt-server"
ENV_FILE="$PROJECT_DIR/.env"

SSH_KEY="${SSH_KEY:-$HOME/.ssh/oracle_globaltelco}"
SSH_USER="${SSH_USER:-ubuntu}"
REMOTE="${SSH_USER}@${ORACLE_IP:?Set ORACLE_IP to your instance public IP}"
SSH_OPTS="-i $SSH_KEY -o StrictHostKeyChecking=no -o ConnectTimeout=10"

echo "╔══════════════════════════════════════════════════╗"
echo "║  GlobalTelco — Oracle Cloud Deploy               ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""
echo "  Target: $REMOTE"
echo "  SSH key: $SSH_KEY"
echo "  Env:     $ENV_FILE"
echo ""

# ── Helper ──────────────────────────────────────────────────────────────────

ssh_run() {
    ssh $SSH_OPTS "$REMOTE" "$@"
}

scp_to() {
    scp $SSH_OPTS "$1" "$REMOTE:$2"
}

# ── Parse args ──────────────────────────────────────────────────────────────

SKIP_BUILD=false
SETUP_ONLY=false
FORCE_ENV=false

for arg in "$@"; do
    case $arg in
        --upload-only) SKIP_BUILD=true ;;
        --setup-only) SETUP_ONLY=true ;;
        --force-env) FORCE_ENV=true ;;
    esac
done

# ── Step 1: First-time server setup ────────────────────────────────────────

setup_server() {
    echo ">>> Setting up server (first-time install)..."

    ssh_run "sudo bash -s" <<'SETUP_SCRIPT'
# Create service user
if ! id globaltelco &>/dev/null; then
    sudo useradd -r -s /usr/sbin/nologin -d /opt/globaltelco globaltelco
fi

# Create directories
mkdir -p /opt/globaltelco
chown globaltelco:globaltelco /opt/globaltelco

# Install nginx + certbot
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq nginx certbot python3-certbot-nginx > /dev/null

# Create certbot webroot
mkdir -p /var/www/certbot

# Open firewall ports (Ubuntu iptables)
iptables -I INPUT -p tcp --dport 80 -j ACCEPT
iptables -I INPUT -p tcp --dport 443 -j ACCEPT
iptables -I INPUT -p tcp --dport 3001 -j ACCEPT

# Save iptables rules so they persist across reboots
apt-get install -y -qq iptables-persistent > /dev/null
netfilter-persistent save

echo "Server setup complete."
SETUP_SCRIPT
}

# ── Step 2: Cross-compile ──────────────────────────────────────────────────

build_binary() {
    echo ">>> Cross-compiling gt-server for x86_64 Linux (Ubuntu 24.04 glibc 2.39)..."
    cd "$PROJECT_DIR"
    cargo zigbuild --target x86_64-unknown-linux-gnu.2.39 --release --bin gt-server --features postgres 2>&1 | tail -3
    # Update binary path to use gnu instead of musl
    BINARY="$PROJECT_DIR/target/x86_64-unknown-linux-gnu/release/gt-server"
    echo "    Binary: $(ls -lh "$BINARY" | awk '{print $5}') at $BINARY"
}

# ── Step 3: Upload & install ───────────────────────────────────────────────

deploy_binary() {
    # Verify .env exists
    if [[ ! -f "$ENV_FILE" ]]; then
        echo "ERROR: $ENV_FILE not found. Create it before deploying."
        exit 1
    fi

    echo ">>> Uploading binary..."
    scp_to "$BINARY" "/tmp/gt-server"

    echo ">>> Uploading config files..."
    scp_to "$ENV_FILE" "/tmp/gt-server.env"
    scp_to "$SCRIPT_DIR/gt-server.service" "/tmp/gt-server.service"
    scp_to "$SCRIPT_DIR/nginx.conf" "/tmp/globaltelco-nginx.conf"
    scp_to "$SCRIPT_DIR/nginx-pre-ssl.conf" "/tmp/globaltelco-nginx-pre-ssl.conf"

    # Handle .env file before the main install script
    if [[ "$FORCE_ENV" == true ]]; then
        echo ">>> Replacing server .env with local version (--force-env)..."
        ssh_run "sudo bash -c 'mv /tmp/gt-server.env /opt/globaltelco/.env && chown globaltelco:globaltelco /opt/globaltelco/.env && chmod 600 /opt/globaltelco/.env'"
    else
        echo ">>> Installing .env (only if not already present)..."
        ssh_run "sudo bash -c 'if [[ ! -f /opt/globaltelco/.env ]]; then mv /tmp/gt-server.env /opt/globaltelco/.env && chown globaltelco:globaltelco /opt/globaltelco/.env && chmod 600 /opt/globaltelco/.env && echo \"Installed .env (first deploy)\"; else rm /tmp/gt-server.env && echo \"Keeping existing .env\"; fi'"
    fi

    echo ">>> Installing on server..."
    ssh_run "sudo bash -s" <<'INSTALL_SCRIPT'
# Stop service if running
systemctl stop globaltelco 2>/dev/null || true

# Install binary
mv /tmp/gt-server /opt/globaltelco/gt-server
chmod +x /opt/globaltelco/gt-server
chown globaltelco:globaltelco /opt/globaltelco/gt-server

# Install systemd service
mv /tmp/gt-server.service /etc/systemd/system/globaltelco.service
systemctl daemon-reload
systemctl enable globaltelco

# Install nginx config (HTTP-only — Cloudflare handles SSL termination)
mv /tmp/globaltelco-nginx-pre-ssl.conf /etc/nginx/sites-available/globaltelco-http
mv /tmp/globaltelco-nginx.conf /etc/nginx/sites-available/globaltelco-ssl-origin

# Use HTTP-only config (Cloudflare proxy terminates TLS, connects to origin on port 80)
echo "Using HTTP-only nginx config (Cloudflare handles SSL)"
ln -sf /etc/nginx/sites-available/globaltelco-http /etc/nginx/sites-enabled/globaltelco
rm -f /etc/nginx/sites-enabled/default
nginx -t && systemctl reload nginx

# Start the game server
systemctl start globaltelco

echo ""
echo "Service status:"
systemctl status globaltelco --no-pager -l | head -15
INSTALL_SCRIPT
}

# ── Main ───────────────────────────────────────────────────────────────────

if [[ "$SETUP_ONLY" == true ]]; then
    setup_server
    echo ""
    echo "Setup complete! Next: run this script again without --setup-only to deploy."
    exit 0
fi

if [[ "$SKIP_BUILD" != true ]]; then
    build_binary
fi

setup_server
deploy_binary

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  Deploy complete!                                ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""
DOMAIN="server.globaltelco.online"
echo "  Health (HTTP):  curl http://$DOMAIN/health"
echo "  Health (HTTPS): curl https://$DOMAIN/health"
echo "  WebSocket:      wss://$DOMAIN/ws"
echo "  REST API:       https://$DOMAIN/api"
echo ""
echo "  Server logs:    ssh $SSH_OPTS $REMOTE 'sudo journalctl -u globaltelco -f'"
echo ""
if [[ "$FORCE_ENV" == true ]]; then
    echo "  .env: Replaced with local version"
else
    echo "  .env: Preserved existing (use --force-env to replace)"
fi
echo ""
