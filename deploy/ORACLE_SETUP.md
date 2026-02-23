# Oracle Cloud Free Tier Setup — GlobalTelco Game Server

## Step 1: Create a Compute Instance

1. Log into [Oracle Cloud Console](https://cloud.oracle.com/)
2. Click the hamburger menu (top left) -> **Compute** -> **Instances**
3. Click **Create Instance**
4. Configure:
   - **Name:** `globaltelco-server`
   - **Image:** Click "Edit" -> **Ubuntu** -> **Ubuntu 24.04** (Canonical)
   - **Shape:** Click "Change Shape" -> **Ampere** (ARM) -> **VM.Standard.A1.Flex**
     - OCPUs: **4** (free tier max)
     - Memory: **24 GB** (free tier max)
   - **Networking:** Use default VCN, or create new one. **Assign a public IPv4 address.**
   - **SSH keys:** Click "Generate a key pair" and **download both keys** (save them somewhere safe!). Or paste your existing `~/.ssh/id_ed25519.pub`.
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
| 0.0.0.0/0   | TCP      | 80        | HTTP (nginx)         |
| 0.0.0.0/0   | TCP      | 443       | HTTPS (nginx)        |
| 0.0.0.0/0   | TCP      | 3001      | Game server (direct) |

## Step 3: Deploy

From your local machine, run:

```bash
# Set your instance IP (replace with your actual IP)
export ORACLE_IP=<your-instance-public-ip>

# If you generated keys in Oracle console, specify the path:
export SSH_KEY=~/.ssh/id_rsa   # or wherever you saved the Oracle private key

# Run the deploy script
./deploy/deploy.sh
```

The deploy script will:
1. Cross-compile gt-server for ARM Linux
2. Upload the binary + config to the instance
3. Install and start the systemd service
4. Set up nginx reverse proxy with SSL (optional)

## Step 4: Verify

```bash
# Check server is running
ssh -i $SSH_KEY ubuntu@$ORACLE_IP "sudo systemctl status globaltelco"

# Check health endpoint
curl http://$ORACLE_IP:3001/health

# WebSocket test
# In your frontend, set PUBLIC_API_URL=http://<ip>:3001 and PUBLIC_WS_URL=ws://<ip>:3001/ws
```

## Step 5: (Optional) Add a Domain + HTTPS

If you have a domain:

```bash
ssh -i $SSH_KEY ubuntu@$ORACLE_IP
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d game.yourdomain.com
```

Then point your frontend at `https://game.yourdomain.com` and `wss://game.yourdomain.com/ws`.

## Costs

All free, forever (not trial):
- Compute: 4 ARM OCPUs + 24GB RAM
- Storage: 200GB block volume
- Network: 10TB/month outbound
- Neon DB: Separate free tier (500MB storage, 100 hours compute)

## Troubleshooting

```bash
# View server logs
ssh -i $SSH_KEY ubuntu@$ORACLE_IP "sudo journalctl -u globaltelco -f"

# Restart server
ssh -i $SSH_KEY ubuntu@$ORACLE_IP "sudo systemctl restart globaltelco"

# Check if port is open (from your local machine)
nc -zv $ORACLE_IP 3001

# If port check fails, also open it in the OS firewall:
ssh -i $SSH_KEY ubuntu@$ORACLE_IP "sudo iptables -I INPUT -p tcp --dport 3001 -j ACCEPT"
```
