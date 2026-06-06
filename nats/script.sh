#!/bin/sh
set -eu

NATS_SERVER="${NATS_SERVER:-nats://nats:4222}"

until nats --server "$NATS_SERVER" account info >/dev/null 2>&1; do
  sleep 1
done

nats --server "$NATS_SERVER" stream add EVENTS --config /nats/events.json \
  || nats --server "$NATS_SERVER" stream edit EVENTS --config /nats/events.json
nats --server "$NATS_SERVER" stream add COMMANDS --config /nats/commands.json \
  || nats --server "$NATS_SERVER" stream edit COMMANDS --config /nats/commands.json
