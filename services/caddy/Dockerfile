ARG CADDY_VERSION=2.11.4
FROM caddy:${CADDY_VERSION}-builder-alpine AS builder
RUN xcaddy build \
  --with github.com/caddy-dns/cloudflare \
  --with github.com/mholt/caddy-ratelimit \
  --with github.com/mholt/caddy-l4 \
  --with github.com/caddyserver/transform-encoder \
  --with github.com/hslatman/caddy-crowdsec-bouncer/crowdsec \
  --with github.com/hslatman/caddy-crowdsec-bouncer/http \
  --with github.com/hslatman/caddy-crowdsec-bouncer/appsec \
  --with github.com/hslatman/caddy-crowdsec-bouncer/layer4 \
  --with github.com/pberkel/caddy-storage-redis@v1.8.1

FROM caddy:${CADDY_VERSION}-alpine
COPY --from=builder /usr/bin/caddy /usr/bin/caddy
COPY Caddyfile /etc/caddy/Caddyfile
