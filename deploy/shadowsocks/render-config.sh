#!/bin/sh

set -eu

source_path=/etc/shadowsocks-rust/config.json
rendered_path=/tmp/shadowsocks-config.json
log_dir=/var/log/shadowsocks
log_path=$log_dir/service.log

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

mkdir -p "$log_dir"

exec docker-entrypoint.sh ssserver --log-without-time -a nobody -c "$rendered_path" >>"$log_path" 2>&1