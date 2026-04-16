#!/usr/bin/env bash
set -euo pipefail

MIRRORS=(
  "http://ftp.jaist.ac.jp/pub/Linux/ubuntu"
  "http://ftp.iij.ad.jp/pub/linux/ubuntu/archive"
  "http://jp.archive.ubuntu.com/ubuntu"
  "http://archive.ubuntu.com/ubuntu"  # fallback
)

PROBE_PATH="/ubuntu/dists/noble/Release"
TIMEOUT=3
best_mirror=""
best_time=9999

for mirror in "${MIRRORS[@]}"; do
  url="${mirror}${PROBE_PATH}"
  elapsed=$(curl -o /dev/null -s -w "%{time_total}" \
    --max-time "$TIMEOUT" \
    --connect-timeout "$TIMEOUT" \
    "$url" 2>/dev/null || echo "9999")
  echo "  ${mirror}: ${elapsed}s" >&2
  if awk "BEGIN {exit !($elapsed < $best_time)}"; then
    best_time="$elapsed"
    best_mirror="$mirror"
  fi
done

echo "$best_mirror"
