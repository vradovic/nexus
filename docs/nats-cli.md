# NATS CLI

Start a temporary interactive container with the NATS CLI:

```bash
docker compose run --rm nats-box sh
```

Inside the container, connect to the Compose NATS service with:

```bash
nats --server nats://nats:4222 account info
```

Useful examples:

```bash
nats --server nats://nats:4222 stream ls
nats --server nats://nats:4222 stream info EVENTS
nats --server nats://nats:4222 pub event.game.test '{"hello":"world"}'
```
