# Surge Client Integration

This repository now includes a reference Surge rule script at [surge/rule-relay.js](/Users/adasdairpan/Workspace/rule-relay/surge/rule-relay.js).

The script calls the relay endpoint for each hostname that reaches the `SCRIPT` rule and returns `matched: true` when the relay says the domain should be blocked. In Surge, that means the policy on the matching rule line will be applied.

## Prerequisite

The relay must be reachable from the client device.

The current Docker Compose deployment keeps the relay loopback-bound on sv01, so the script cannot call it directly from your Mac unless you add a public entrypoint in front of it. In practice that means one of these:

- Put HTTPS reverse proxying in front of `127.0.0.1:8080` on sv01.
- Expose the relay through another secure tunnel or private network path that your client can reach.

## Script Configuration

Add the script in Surge `[Script]`:

```ini
[Script]
rule-relay = type=rule,script-path=/absolute/path/to/rule-relay.js,timeout=10,argument=relay-url=https%3A%2F%2Frelay.example.com&auth-token=REPLACE_WITH_RELAY_TOKEN&api-policy=DIRECT&fail-open=true&debug=false
```

Add the blocking rule in Surge `[Rule]`:

```ini
[Rule]
SCRIPT,rule-relay,REJECT
```

## Arguments

- `relay-url`: Public base URL for the relay, without a trailing slash preferred.
- `auth-token`: The same bearer token configured as `RELAY_AUTH_TOKEN` on the server.
- `api-policy`: Surge policy used for the script's own HTTP request. Default is `DIRECT`. Keeping this separate from the proxy path avoids request recursion.
- `timeout`: Relay request timeout in seconds. Default is `5`.
- `fail-open`: When `true`, relay errors allow the request instead of blocking it. Default is `true`.
- `debug`: When `true`, the script logs diagnostic messages to the Surge script console.

## Behavior

- The script uses `$request.hostname` from the Surge rule-script API.
- Relay lookups are sent to `GET /v1/domain-check?domain=...` with `Authorization: Bearer <token>`.
- Results are cached in `$persistentStore` until the TTL returned by the relay expires.
- The relay hostname itself is excluded from matching to avoid recursive self-lookups.

## Notes

- `SCRIPT,rule-relay,REJECT` is the intended pairing for blocking. If you want a different policy action for matched domains, change the policy on the `[Rule]` line.
- If you leave the relay loopback-bound, this script is still useful as a reference implementation, but it will not work from external clients until the relay has a reachable URL.
