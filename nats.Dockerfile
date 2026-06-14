# NATS with the WebSocket listener baked in, for platforms that can't mount a
# config volume (Railway, Render…). Locally, docker-compose mounts nats.conf
# onto the stock image instead.
FROM nats:latest
COPY nats.conf /etc/nats/nats.conf
EXPOSE 4222 8080
CMD ["-c", "/etc/nats/nats.conf"]
