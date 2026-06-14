# Builds the dashboard and serves the static bundle with nginx.
# The browser talks to NATS directly over WebSocket; the URL is injected at
# container start into /config.js from $NATS_WS_URL (see docker-entrypoint.d).
FROM node:22-alpine AS build
WORKDIR /app
RUN corepack enable
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM nginx:alpine
# nginx listens on $PORT (Railway sets it; defaults to 80 locally).
ENV PORT=80
ENV NATS_WS_URL=ws://127.0.0.1:8080
COPY --from=build /app/dist /usr/share/nginx/html
COPY default.conf.template /etc/nginx/templates/default.conf.template
COPY docker-entrypoint.d/40-nats-url.sh /docker-entrypoint.d/40-nats-url.sh
RUN chmod +x /docker-entrypoint.d/40-nats-url.sh
EXPOSE 80
