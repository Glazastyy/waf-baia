ARG CADDY_VERSION=2.11.4
FROM caddy:${CADDY_VERSION}-builder-alpine AS builder
RUN xcaddy build \
  --with github.com/caddy-dns/cloudflare@v0.2.4 \
  --with github.com/mholt/caddy-ratelimit@v0.1.0 \
  --with github.com/mholt/caddy-l4@v0.1.2 \
  --with github.com/caddyserver/transform-encoder@v0.0.0-20260423033309-ba4124974830 \
  --with github.com/hslatman/caddy-crowdsec-bouncer/crowdsec@v0.14.1 \
  --with github.com/hslatman/caddy-crowdsec-bouncer/http@v0.14.1 \
  --with github.com/hslatman/caddy-crowdsec-bouncer/appsec@v0.14.1 \
  --with github.com/hslatman/caddy-crowdsec-bouncer/layer4@v0.14.1 \
  --with github.com/pberkel/caddy-storage-redis@v1.8.1

FROM caddy:${CADDY_VERSION}-alpine
COPY --from=builder /usr/bin/caddy /usr/bin/caddy
COPY Caddyfile /etc/caddy/Caddyfile
