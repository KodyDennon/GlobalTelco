# Oracle Cloud Free Tier Setup — GlobalTelco Game Server

## Current Deployment

- **Domain:** `globaltelco.gameservers.kodydennon.com`
- **IP:** `159.54.188.149`
- **Instance:** VM.Standard.E2.1.Micro (AMD x86, 1 OCPU, 1GB RAM) — always free
- **SSL:** Let's Encrypt (auto-renewing via certbot)
- **Endpoints:**
  - Health: `https://globaltelco.gameservers.kodydennon.com/health`
  - API: `https://globaltelco.gameservers.kodydennon.com/api`
  - WebSocket: `wss://globaltelco.gameservers.kodydennon.com/ws`
- **SSH:** `ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149`
- **Frontend:** `https://global-telco.vercel.app/`

## Architecture

```
Browser (HTTPS) ──► Nginx (port 443, SSL termination)
                        │
                        ▼
                    gt-server (port 3001, localhost only)
                        │
                        ▼
                    Neon PostgreSQL (us-west-2)
```

- Nginx handles SSL termination, HTTP→HTTPS redirect, and WebSocket upgrade proxying
- The Rust game server binds to `0.0.0.0:3001` (accessed only via nginx in production)
- CORS is restricted to `https://global-telco.vercel.app`

## Step 1: Create a Compute Instance

1. Log into [Oracle Cloud Console](https://cloud.oracle.com/)
2. Click the hamburger menu (top left) -> **Compute** -> **Instances**
3. Click **Create Instance**
4. Configure:
   - **Name:** `globaltelco-server`
   - **Image:** Click "Edit" -> **Ubuntu** -> **Ubuntu 24.04** (Canonical)
   - **Shape:** Click "Change Shape" -> **Specialty and previous generation** -> **VM.Standard.E2.1.Micro**
     - 1 OCPU, 1 GB RAM (Always Free)
     - Note: ARM A1.Flex (4 OCPU, 24GB) may show "Out of host capacity" in some regions
   - **Networking:** Use default VCN, or create new one. **Assign a public IPv4 address.**
   - **SSH keys:** Click "Generate a key pair" and **download both keys**, or paste your existing public key.
5. Click **Create**
6. Wait for the instance to show **Running** (takes 1-2 minutes)
7. Copy the **Public IP Address** from the instance details page

## Step 2: Open Firewall Ports (Security List)

Oracle blocks all inbound traffic by default. You need to open ports for the game server.

1. In your instance details, click the **Subnet** link (under Primary VNIC)
2. Click the **Default Security List**
3. Click **Add Ingress Rules** and add these rules:

| Source CIDR  | Protocol | Dest Port | Description          |
|-------------|----------|-----------|----------------------|
| 0.0.0.0/0   | TCP      | 80        | HTTP (nginx → HTTPS redirect) |
| 0.0.0.0/0   | TCP      | 443       | HTTPS (nginx SSL)    |
| 0.0.0.0/0   | TCP      | 3001      | Game server (direct, optional) |

## Step 3: DNS Setup

Point your domain to the instance IP:

```
globaltelco.gameservers.kodydennon.com  A  159.54.188.149
```

Verify DNS resolves before proceeding:
```bash
dig +short globaltelco.gameservers.kodydennon.com
# Should return: 159.54.188.149
```

## Step 4: Deploy

From your local machine:

```bash
# Set your instance IP
export ORACLE_IP=159.54.188.149

# Run the deploy script (builds + uploads + installs)
./deploy/deploy.sh

# Or skip the build if binary is already compiled:
./deploy/deploy.sh --upload-only

# Or just set up the server (first time, no binary):
./deploy/deploy.sh --setup-only
```

The deploy script will:
1. Create `globaltelco` service user
2. Install nginx + certbot
3. Cross-compile gt-server for x86_64 Linux (static musl via `cargo zigbuild`)
4. Upload binary, env, nginx config, systemd service
5. Auto-detect SSL certs and use HTTPS config if available

## Step 5: SSL Setup (First Time Only)

After the first deploy, obtain an SSL certificate:

```bash
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149

# Get SSL cert from Let's Encrypt
sudo certbot certonly --webroot -w /var/www/certbot \
  -d globaltelco.gameservers.kodydennon.com \
  --non-interactive --agree-tos -m your@email.com

# Switch nginx to SSL config
sudo ln -sf /etc/nginx/sites-available/globaltelco-ssl /etc/nginx/sites-enabled/globaltelco
sudo nginx -t && sudo systemctl reload nginx
```

Certbot auto-renews via systemd timer. Verify:
```bash
sudo certbot renew --dry-run
```

## Step 6: Verify

```bash
# Health check (HTTPS)
curl https://globaltelco.gameservers.kodydennon.com/health

# API info
curl https://globaltelco.gameservers.kodydennon.com/api/info

# Check CORS headers
curl -s -H "Origin: https://global-telco.vercel.app" \
  -I https://globaltelco.gameservers.kodydennon.com/api/info | grep access-control

# Server status
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "sudo systemctl status globaltelco"
```

## Frontend Configuration

The frontend at `web/src/lib/config.ts` auto-detects environment:
- **Local dev** (`localhost`): connects to `http://localhost:3001`
- **Production** (any other host): connects to `https://globaltelco.gameservers.kodydennon.com`

Override with Vercel env vars if needed:
```
PUBLIC_API_URL=https://globaltelco.gameservers.kodydennon.com
PUBLIC_WS_URL=wss://globaltelco.gameservers.kodydennon.com/ws
```

## Server Environment Variables

Stored at `/opt/globaltelco/.env` on the server. Source of truth: `deploy/.env.production`.

| Variable | Description | Default |
|----------|-------------|---------|
| `GT_HOST` | Bind address | `0.0.0.0` |
| `GT_PORT` | Bind port | `3001` |
| `GT_JWT_SECRET` | JWT signing secret | (generated) |
| `GT_ACCESS_TOKEN_EXPIRY` | Access token TTL (seconds) | `3600` |
| `GT_REFRESH_TOKEN_EXPIRY` | Refresh token TTL (seconds) | `2592000` |
| `GT_DEFAULT_WORLD` | Default world name | `Global Telco World` |
| `GT_MAX_PLAYERS` | Max players per world | `8` |
| `CORS_ORIGIN` | Allowed CORS origin | `https://global-telco.vercel.app` |
| `ADMIN_KEY` | Admin API key (`X-Admin-Key` header) | (generated) |
| `DATABASE_URL` | Neon PostgreSQL connection string | (required) |
| `RUST_LOG` | Log level filter | `gt_server=info,tower_http=info` |

## Costs

All free, forever (not trial):
- Compute: 1 AMD OCPU + 1GB RAM (Always Free Micro instance)
- Storage: 200GB block volume
- Network: 10TB/month outbound
- SSL: Let's Encrypt (free)
- Neon DB: Separate free tier (500MB storage, 100 hours compute)

## Troubleshooting

```bash
# View server logs
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "sudo journalctl -u globaltelco -f"

# Restart game server
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "sudo systemctl restart globaltelco"

# Restart nginx
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "sudo systemctl restart nginx"

# Check nginx config
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "sudo nginx -t"

# Check SSL cert expiry
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "sudo certbot certificates"

# Force SSL cert renewal
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "sudo certbot renew --force-renewal"

# Check if ports are open (from local machine)
curl -s https://globaltelco.gameservers.kodydennon.com/health

# Memory usage
ssh -i ~/.ssh/oracle_globaltelco ubuntu@159.54.188.149 "free -h"
```
