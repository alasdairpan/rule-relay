#!/bin/sh

set -eu

: "${SS_SERVER_PORT:?SS_SERVER_PORT must be set}"
: "${SS_SERVER_PASSWORD:?SS_SERVER_PASSWORD must be set}"

template_path=/etc/shadowsocks-rust/config.template.json
rendered_path=/tmp/shadowsocks-config.json

awk '
{
  gsub(/\$\{SS_SERVER_PORT\}/, ENVIRON["SS_SERVER_PORT"]);
  gsub(/\$\{SS_SERVER_PASSWORD\}/, ENVIRON["SS_SERVER_PASSWORD"]);
  print;
}
' "$template_path" > "$rendered_path"

exec docker-entrypoint.sh ssserver --log-without-time -a nobody -c "$rendered_path"