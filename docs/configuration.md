# Configuration Guide

This repository includes a few deployment assets that cannot hold inline comments safely, especially JSON files mounted directly into containers. This document explains the purpose of those files and the environment variables used by the stack.

## Environment Variables

### Proxy Transport

- `SS_SERVER_PORT`: Public TCP and UDP port exposed by `shadowsocks-rust`.
- `SS_SERVER_PASSWORD`: Shared secret used by Shadowsocks clients.

### Relay API

- `RELAY_AUTH_TOKEN`: Bearer token required by `GET /v1/domain-check`.
- `RELAY_PORT`: Loopback-bound host port published for the relay container.
- `RELAY_ALLOWED_TTL_SECS`: TTL returned for allowed-domain decisions.
- `RELAY_BLOCKED_TTL_SECS`: TTL returned for blocked-domain decisions.
- `ADGUARD_USERNAME`: AdGuard Home username used by the relay when calling the protected control API.
- `ADGUARD_PASSWORD`: AdGuard Home password used by the relay when calling the protected control API.

### AdGuard Home

- `ADGUARD_ADMIN_PORT`: Loopback-bound host port used for the AdGuard Home setup UI and admin access.

## Docker Compose Topology

The stack is intentionally split by exposure level:

- `shadowsocks`: Public by default. This is the transport layer.
- `adguardhome`: Private to the host and Docker network. This is the filtering decision engine.
- `relay`: Loopback-bound on the host by default. This is the authenticated API surface expected to sit behind a reverse proxy if you need public HTTPS access.

## deploy/shadowsocks/config.json

This file is valid JSON and carries sane defaults for local editing and review. Immediately before `ssserver-rust` starts, the container entrypoint rewrites the `server_port` and `password` fields from `SS_SERVER_PORT` and `SS_SERVER_PASSWORD` when those environment variables are set.

Field summary:

- `server`: Container bind address. `0.0.0.0` allows the server process to listen on all container interfaces.
- `server_port`: Default listening port, optionally overridden from `SS_SERVER_PORT` before the container starts `ssserver`.
- `password`: Default shared secret, optionally overridden from `SS_SERVER_PASSWORD` before the container starts `ssserver`.
- `timeout`: Idle timeout in seconds for inactive connections.
- `method`: Cipher suite used by the Shadowsocks server.
- `fast_open`: Enables TCP Fast Open when supported by the runtime and host.
- `log.level`: Runtime log verbosity for the Shadowsocks server.

The Compose service mounts `deploy/shadowsocks/config.json` read-only and writes a runtime copy just before launching `ssserver`, overriding only the environment-driven fields when present.

## AdGuard Home State Directories

The directories below are bind-mounted so AdGuard Home keeps state outside the container lifecycle:

- `deploy/adguard/conf`: Configuration and setup state.
- `deploy/adguard/work`: Runtime data such as filter state and working files.

Both paths are intentionally ignored by Git because they can contain host-specific state and sensitive configuration.

## Relay Container Build

The relay image is built in two stages:

- Builder stage: Compiles the Rust binary with the full toolchain.
- Runtime stage: Copies only the release binary and CA certificates into a slim Debian image.

This keeps the runtime image smaller and reduces the shipped toolchain surface.
