# Deployment

The stack is **4 services** talking over NATS:

```
[browser] ──wss──► nats (WebSocket :8080, public)
                     ▲ TCP :4222 (private)
        ┌────────────┼────────────┐
   vanguard-control  vanguard-map   (workers, no public port)
   webui (nginx, public) ──serves the dashboard──► [browser]
```

Key wiring:
- Rust binaries read **`NATS_URL`** (default `nats://nats:4222`).
- The browser reaches NATS directly over WebSocket. The URL is injected at the
  webui container's start into `/config.js` from **`NATS_WS_URL`** — no rebuild
  needed to change it.

---

## Local / VPS — one command

```bash
docker compose up --build
# open http://localhost:5173
```

`docker-compose.yml` runs NATS (stock image + mounted `nats.conf`), both Rust
workers, and the webui. This is also the cheapest production option: run the
same command on any small VPS.

---

## Railway (multi-service, one project)

Railway deploys one repo as several services with private networking. There is
no literal one-click for a 4-service realtime app, but this is the full path.

1. **New Project → Deploy from GitHub repo.**

2. **Service `nats`**
   - Source: this repo. Settings → **Dockerfile Path** = `nats.Dockerfile`.
   - Networking → expose a public port mapped to **`8080`** (the WebSocket
     listener) → gives `wss://nats-xxx.up.railway.app`.
   - Port `4222` stays private (reachable at `nats.railway.internal:4222`).

3. **Service `control`**
   - Source: this repo, default `Dockerfile`.
   - Variables: `NATS_URL=nats://nats.railway.internal:4222`,
     `RECOGNITION_RANGE_M=4000`.
   - Start command: `vanguard-control`. No public port.

4. **Service `map`**
   - Same image/`Dockerfile`. Variables: `NATS_URL=nats://nats.railway.internal:4222`.
   - Start command: `vanguard-map`. No public port.

5. **Service `webui`**
   - Source: `webui/` (default `webui/Dockerfile`).
   - Variables: `NATS_WS_URL=wss://nats-xxx.up.railway.app` (the public domain
     from step 2).
   - Expose a public port mapped to `80` → that URL is your dashboard.

Notes:
- `nats.conf` already has `no_tls: true` — correct, because Railway terminates
  TLS at the edge (`wss` outside, plain `ws` inside the container).
- The webui injects `NATS_WS_URL` at **container start**, so changing it only
  needs a redeploy, not a rebuild.
- Railway has **no free tier** (trial credit, then ~usage-based). Four always-on
  services cost money — fine for a demo, not free.

---

## Render (closest to a one-click button)

A single `render.yaml` Blueprint can describe all four services and gives a
"Deploy to Render" button. Not included here yet — ask if you want it generated.
