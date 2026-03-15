#!/bin/sh

set -eu

source_path=/etc/shadowsocks-rust/config.json
rendered_path=/tmp/shadowsocks-config.json

awk '
function escape_json_string(value, escaped) {
  escaped = value
  gsub(/\\/, "\\\\", escaped)
  gsub(/"/, "\\\"", escaped)
  return escaped
}

{
  if (ENVIRON["SS_SERVER_PORT"] != "" && $0 ~ /^[[:space:]]*"server_port"[[:space:]]*:/) {
    sub(/: .*/, ": " ENVIRON["SS_SERVER_PORT"] ",")
  }

  if (ENVIRON["SS_SERVER_PASSWORD"] != "" && $0 ~ /^[[:space:]]*"password"[[:space:]]*:/) {
    password = escape_json_string(ENVIRON["SS_SERVER_PASSWORD"])
    sub(/: .*/, ": \"" password "\",")
  }

  print
}
' "$source_path" > "$rendered_path"

exec docker-entrypoint.sh ssserver --log-without-time -a nobody -c "$rendered_path"