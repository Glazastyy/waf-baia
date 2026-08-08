FROM oven/bun:1.3.14 AS builder
WORKDIR /workspace
COPY package.json bun.lock ./
COPY apps/web/package.json ./apps/web/package.json
RUN bun install --frozen-lockfile
COPY apps/web ./apps/web
RUN bun run --cwd apps/web build

FROM caddy:2.11.4-alpine
COPY --from=builder /workspace/apps/web/dist /usr/share/caddy
COPY apps/web/Caddyfile /etc/caddy/Caddyfile
EXPOSE 80
