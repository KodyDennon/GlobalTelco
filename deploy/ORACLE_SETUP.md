# Oracle Cloud Free Tier Setup — GlobalTelco Game Server

## Current Deployment

- **Frontend:** `https://globaltelco.online` (Vercel + Cloudflare DNS)
- **Server:** `https://server.globaltelco.online` (Oracle Cloud + Cloudflare proxy)
- **Origin IP:** `<your-server-ip>`
- **Instance:** VM.Standard.E2.1.Micro (AMD x86, 1 OCPU, 1GB RAM) — always free
- **SSL:** Cloudflare terminates TLS (free plan), origin serves HTTP only
- **DNS:** Cloudflare (zone: `globaltelco.online`)
- **Endpoints:**
  - Health: `https://server.globaltelco.online/health`
  - API: `https://server.globaltelco.online/api`
  - WebSocket: `wss://server.globaltelco.online/ws`
- **SSH:** `ssh -i ~/.ssh/oracle_globaltelco ubuntu@<your-server-ip>`

## Architecture

```
Browser (HTTPS/WSS) ──► Cloudflare (TLS termination, WebSocket proxy)
                              │
                              ▼ (HTTP, port 80)
                         Nginx (reverse proxy)
                              │
                              ▼
                         gt-server (port 3001, localhost)
                              │
                              ▼
                         Neon PostgreSQL (us-west-2)
```

- Cloudflare handles TLS termination (SSL mode: Flexible) and WebSocket proxying
- Nginx on origin listens on port 80, proxies to gt-server on port 3001
- CORS is restricted to `https://globaltelco.online`
- Cloudflare free plan: WebSocket support enabled, 100s idle timeout (game ticks every 1s, so no issue)

## Cloudflare Configuration

| Setting | Value |
|---------|-------|
| SSL mode | Flexible (Cloudflare → origin over HTTP) |
| WebSockets | Enabled |
| `globaltelco.online` | A record → `76.76.21.21` (Vercel), DNS-only |
| `www.globaltelco.online` | CNAME → `cname.vercel-dns.com`, DNS-only |
| `server.globaltelco.online` | A record → `<your-server-ip>`, Proxied (orange cloud) |

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
| 0.0.0.0/0   | TCP      | 80        | HTTP (Cloudflare → nginx) |
| 0.0.0.0/0   | TCP      | 3001      | Game server (direct, optional for debugging) |

Note: Port 443 is not needed — Cloudflare terminates TLS and connects to origin on port 80.

## Step 3: DNS Setup (Cloudflare)

DNS records are managed in the Cloudflare dashboard (or via API). Required records:

| Type | Name | Content | Proxy |
|------|------|---------|-------|
| A | `globaltelco.online` | `76.76.21.21` | DNS-only (gray) |
| CNAME | `www` | `cname.vercel-dns.com` | DNS-only (gray) |
| A | `server` | `<your-server-ip>` | Proxied (orange) |

Cloudflare zone settings:
- SSL/TLS → **Flexible**
- Network → WebSockets → **On**

## Step 4: Deploy

From your local machine:

```bash
# Set your instance IP
export ORACLE_IP=<your-server-ip>

# Run the deploy script (builds + uploads + installs)
./deploy/deploy.sh

# Or skip the build if binary is already compiled:
./deploy/deploy.sh --upload-only

# Or just set up the server (first time, no binary):
./deploy/deploy.sh --setup-only
```

The deploy script will:
1. Create `globaltelco` service user
2. Install nginx
3. Cross-compile gt-server for x86_64 Linux (via `cargo zigbuild`)
4. Upload binary, env, nginx config, systemd service
5. Use HTTP-only nginx config (Cloudflare handles SSL)

## Step 5: Verify

```bash
# Health check (HTTPS via Cloudflare)
curl https://server.globaltelco.online/health

# API info
curl https://server.globaltelco.online/api/info

# Check CORS headers
curl -s -H "Origin: https://globaltelco.online" \
  -I https://server.globaltelco.online/api/info | grep access-control

# Verify Cloudflare is proxying (look for cf-ray header)
curl -sI https://server.globaltelco.online/health | grep cf-ray

# Server status
ssh -i ~/.ssh/oracle_globaltelco ubuntu@<your-server-ip> "sudo systemctl status globaltelco"
```

## Frontend Configuration

The frontend at `web/src/lib/config.ts` auto-detects environment:
- **Local dev** (`localhost`): connects to `http://localhost:3001`
- **Production** (any other host): connects to `https://server.globaltelco.online`

Vercel environment variables:
```
PUBLIC_API_URL=https://server.globaltelco.online
PUBLIC_WS_URL=wss://server.globaltelco.online/ws
```

## Server Environment Variables

Stored at `/opt/globaltelco/.env` on the server. Source of truth: `.env` (project root).

| Variable | Description | Default |
|----------|-------------|---------|
| `GT_HOST` | Bind address | `0.0.0.0` |
| `GT_PORT` | Bind port | `3001` |
| `GT_JWT_SECRET` | JWT signing secret | (generated) |
| `GT_ACCESS_TOKEN_EXPIRY` | Access token TTL (seconds) | `3600` |
| `GT_REFRESH_TOKEN_EXPIRY` | Refresh token TTL (seconds) | `2592000` |
| `GT_DEFAULT_WORLD` | Default world name | `Global Telco World` |
| `GT_MAX_PLAYERS` | Max players per world | `8` |
| `CORS_ORIGIN` | Allowed CORS origin | `https://globaltelco.online` |
| `ADMIN_KEY` | Admin API key (`X-Admin-Key` header) | (required) |
| `DATABASE_URL` | Neon PostgreSQL connection string | (required) |
| `RUST_LOG` | Log level filter | `gt_server=info,tower_http=info` |

## Costs

All free, forever (not trial):
- Compute: 1 AMD OCPU + 1GB RAM (Always Free Micro instance)
- Storage: 200GB block volume
- Network: 10TB/month outbound
- SSL: Cloudflare (free plan)
- DNS: Cloudflare (free plan)
- Neon DB: Separate free tier (500MB storage, 100 hours compute)

## Troubleshooting

```bash
# View server logs
ssh -i ~/.ssh/oracle_globaltelco ubuntu@<your-server-ip> "sudo journalctl -u globaltelco -f"

# Restart game server
ssh -i ~/.ssh/oracle_globaltelco ubuntu@<your-server-ip> "sudo systemctl restart globaltelco"

# Restart nginx
ssh -i ~/.ssh/oracle_globaltelco ubuntu@<your-server-ip> "sudo systemctl restart nginx"

# Check nginx config
ssh -i ~/.ssh/oracle_globaltelco ubuntu@<your-server-ip> "sudo nginx -t"

# Check if ports are open (from local machine)
curl -s https://server.globaltelco.online/health

# Memory usage
ssh -i ~/.ssh/oracle_globaltelco ubuntu@<your-server-ip> "free -h"
```
