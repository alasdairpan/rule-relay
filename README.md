# Rule Relay

Rule Relay is a split-responsibility filtering stack for personal use:

- `shadowsocks-rust` provides proxy transport.
- `AdGuard Home` provides domain filtering decisions.
- `relay` exposes a small authenticated API for the client-side Surge script.

The repository is intentionally scoped to the VPS-side components. It does not include the client-side Surge enforcement script, TLS termination, or AdGuard bootstrap automation.

## Repository Layout

- `docker-compose.yml` defines the VPS stack.
- `deploy/shadowsocks/config.json` is the mounted `ssserver-rust` config.
- `docs/configuration.md` documents environment variables and deployment assets that cannot carry inline comments.
- `relay/` contains the Rust relay service.

## Relay API

The relay currently exposes:

- `GET /healthz`
- `GET /v1/domain-check?domain=example.com`

Example response:

```json
{
  "domain": "example.com",
  "blocked": false,
  "reason": "allowed",
  "ttl": 3600,
  "checked_at": "2026-03-11T12:00:00Z",
  "source": "adguard",
  "cache_status": "miss"
}
```

All domain-check requests require:

```text
Authorization: Bearer <RELAY_AUTH_TOKEN>
```

The relay accepts hostnames only. Full URLs, paths, and IP literals are rejected.

## Required Environment

The Compose stack expects these values:

- `SS_SERVER_PORT`
- `SS_SERVER_PASSWORD`
- `RELAY_AUTH_TOKEN`
- `RELAY_PORT`
- `ADGUARD_ADMIN_PORT`
- `ADGUARD_USERNAME`
- `ADGUARD_PASSWORD`

Use [.env.example](/Users/adasdairpan/Workspace/rule-relay/.env.example) as the starting point for local or VPS deployment.

## Local Development

1. Copy `.env.example` to `.env` and fill in the secrets.
2. Start the stack with `docker compose up --build`.
3. Complete the AdGuard Home initial setup using the loopback-bound admin UI on `http://127.0.0.1:${ADGUARD_ADMIN_PORT}`.
4. Set `ADGUARD_USERNAME` and `ADGUARD_PASSWORD` in `.env` to the AdGuard Home credentials you configured during setup.
5. Enable the desired parental or adult-domain filters in AdGuard Home.
6. Test the relay with a bearer token once AdGuard is configured.

Example check:

```bash
curl \
  -H "Authorization: Bearer $RELAY_AUTH_TOKEN" \
  "http://127.0.0.1:${RELAY_PORT}/v1/domain-check?domain=example.com"
```

## Security Defaults

- `RELAY_AUTH_TOKEN` is required explicitly and has no insecure default.
- The relay writes debug-level file logs by default.
- AdGuard Home admin is loopback-bound on the host.
- The relay is loopback-bound on the host by default.
- Raw upstream AdGuard error bodies are not returned to API clients.

## Deployment Notes

- `shadowsocks-rust` is the only service publicly exposed by default in Compose.
- AdGuard Home admin is bound to loopback on the host.
- The relay is also bound to loopback on the host so you can put TLS termination and rate limiting in front of it separately.
- The relay currently talks to AdGuard Home over the internal Compose network using `http://adguardhome:3000`.
- A reverse proxy is expected if you want HTTPS and public access to the relay.
