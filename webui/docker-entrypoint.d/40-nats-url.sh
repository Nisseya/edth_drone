#!/bin/sh
# Regenerate the runtime config from the environment before nginx starts.
set -e
: "${NATS_WS_URL:=ws://127.0.0.1:8080}"
echo "window.__NATS_URL__ = '${NATS_WS_URL}';" > /usr/share/nginx/html/config.js
echo "config.js -> NATS at ${NATS_WS_URL}"
