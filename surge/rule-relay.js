/*
 * Surge rule script for rule-relay.
 *
 * Required arguments:
 *   relay-url=https://relay.example.com
 *   auth-token=replace-with-relay-token
 *
 * Optional arguments:
 *   api-policy=DIRECT
 *   timeout=5
 *   fail-open=true
 *   debug=false
 *
 * Example [Script] entry:
 * rule-relay = type=rule,script-path=/path/to/rule-relay.js,timeout=10,argument=relay-url=https://relay.example.com&auth-token=token&api-policy=DIRECT&fail-open=true
 *
 * Example [Rule] entry:
 * SCRIPT,rule-relay,REJECT
 */

const ARGUMENTS = parseArguments(typeof $argument === "string" ? $argument : "");
const RELAY_URL = normalizeRelayUrl(ARGUMENTS["relay-url"] || ARGUMENTS.relay_url || "");
const AUTH_TOKEN = ARGUMENTS["auth-token"] || ARGUMENTS.auth_token || "";
const API_POLICY = ARGUMENTS["api-policy"] || ARGUMENTS.api_policy || "DIRECT";
const TIMEOUT = parsePositiveNumber(ARGUMENTS.timeout, 5);
const FAIL_OPEN = parseBoolean(ARGUMENTS["fail-open"], true);
const DEBUG = parseBoolean(ARGUMENTS.debug, false);
const CACHE_PREFIX = "rule-relay:v1:";

main();

function main() {
  const hostname = normalizeHostname($request.hostname || "");

  if (!hostname) {
    log("missing hostname on request, skipping");
    return finish(false);
  }

  if (!RELAY_URL || !AUTH_TOKEN) {
    log("relay-url or auth-token missing, using fail-open behavior");
    return finishFailure("missing relay configuration");
  }

  const relayHost = extractHostname(RELAY_URL);
  if (relayHost && hostname === relayHost) {
    log("skipping relay hostname to avoid recursion: " + hostname);
    return finish(false);
  }

  const cached = readCache(hostname);
  if (cached) {
    log("cache hit for " + hostname + ": blocked=" + cached.blocked);
    return finish(Boolean(cached.blocked));
  }

  const requestOptions = {
    url: RELAY_URL + "/v1/domain-check?domain=" + encodeURIComponent(hostname),
    headers: {
      Authorization: "Bearer " + AUTH_TOKEN,
      Accept: "application/json"
    },
    timeout: TIMEOUT,
    policy: API_POLICY
  };

  log("querying relay for " + hostname + " via policy=" + API_POLICY);

  $httpClient.get(requestOptions, function(error, response, data) {
    if (error) {
      log("relay request failed for " + hostname + ": " + error);
      return finishFailure("request failed");
    }

    if (!response || response.status !== 200) {
      log("relay returned unexpected status for " + hostname + ": " + (response ? response.status : "no-response"));
      return finishFailure("unexpected status");
    }

    let payload;
    try {
      payload = JSON.parse(data);
    } catch (parseError) {
      log("failed to parse relay response for " + hostname + ": " + String(parseError));
      return finishFailure("invalid json");
    }

    const blocked = Boolean(payload.blocked);
    const ttl = parsePositiveNumber(payload.ttl, blocked ? 86400 : 3600);
    writeCache(hostname, blocked, ttl, payload.reason || "");

    log("relay decision for " + hostname + ": blocked=" + blocked + ", ttl=" + ttl + ", reason=" + (payload.reason || ""));
    finish(blocked);
  });
}

function finishFailure(reason) {
  if (FAIL_OPEN) {
    log("failing open: " + reason);
    return finish(false);
  }

  log("failing closed: " + reason);
  return finish(true);
}

function finish(matched) {
  $done({ matched: matched });
}

function readCache(hostname) {
  const raw = $persistentStore.read(CACHE_PREFIX + hostname);
  if (!raw) {
    return null;
  }

  let entry;
  try {
    entry = JSON.parse(raw);
  } catch (_error) {
    return null;
  }

  if (!entry || typeof entry.expiresAt !== "number" || entry.expiresAt <= Date.now()) {
    return null;
  }

  return entry;
}

function writeCache(hostname, blocked, ttlSeconds, reason) {
  const entry = {
    blocked: blocked,
    reason: reason,
    expiresAt: Date.now() + ttlSeconds * 1000
  };

  $persistentStore.write(JSON.stringify(entry), CACHE_PREFIX + hostname);
}

function parseArguments(argumentString) {
  const parsed = {};
  if (!argumentString) {
    return parsed;
  }

  argumentString.split("&").forEach(function(part) {
    if (!part) {
      return;
    }

    const index = part.indexOf("=");
    const rawKey = index === -1 ? part : part.slice(0, index);
    const rawValue = index === -1 ? "" : part.slice(index + 1);
    const key = decodeComponent(rawKey).trim();
    if (!key) {
      return;
    }

    parsed[key] = decodeComponent(rawValue).trim();
  });

  return parsed;
}

function decodeComponent(value) {
  try {
    return decodeURIComponent(value);
  } catch (_error) {
    return value;
  }
}

function normalizeRelayUrl(value) {
  return String(value || "").replace(/\/+$/, "");
}

function normalizeHostname(value) {
  return String(value || "").trim().toLowerCase().replace(/\.$/, "");
}

function extractHostname(url) {
  const match = String(url || "").match(/^[a-z]+:\/\/([^\/:?#]+)/i);
  return match ? normalizeHostname(match[1]) : "";
}

function parsePositiveNumber(value, fallback) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function parseBoolean(value, fallback) {
  if (value === undefined || value === null || value === "") {
    return fallback;
  }

  const normalized = String(value).trim().toLowerCase();
  if (normalized === "true" || normalized === "1" || normalized === "yes" || normalized === "on") {
    return true;
  }
  if (normalized === "false" || normalized === "0" || normalized === "no" || normalized === "off") {
    return false;
  }

  return fallback;
}

function log(message) {
  if (DEBUG) {
    console.log("[rule-relay] " + message);
  }
}