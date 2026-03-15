#!/bin/sh

set -eu

log_dir=/var/log/adguardhome
log_path=$log_dir/service.log

mkdir -p "$log_dir"

exec /opt/adguardhome/AdGuardHome \
  --no-check-update \
  -c /opt/adguardhome/conf/AdGuardHome.yaml \
  -w /opt/adguardhome/work \
  >>"$log_path" 2>&1