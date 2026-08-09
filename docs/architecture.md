# Baia WAF Architecture

## 1. Arquitetura Geral

Baia WAF é uma plataforma self-hosted para gerenciamento centralizado de reverse proxy, WAF, DNS, certificados, reputação, auditoria e observabilidade. A arquitetura separa o plano de controle do plano de dados.

O plano de dados é o Caddy, compilado com módulos adicionais e configurado dinamicamente pela Admin API. O plano de controle é o Core em Rust, responsável por autenticação, autorização, persistência, validação de configuração, geração de configuração do Caddy e integração com serviços auxiliares. O painel Svelte consome a API administrativa do Core.

## 2. Diagrama dos Componentes

```text
Browser Admin
  |
  v
Web Svelte + Bootstrap
  |
  v
Core Rust API
  |---- PostgreSQL
  |---- Redis
  |---- Caddy Admin API
  |---- PowerDNS API
  |---- Cloudflare API
  |---- CrowdSec Local API
  |---- SMTP or Resend
  v
Caddy Reverse Proxy / WAF
  |
  v
Protected Applications
```

## 3. Responsabilidades de Cada Serviço

Core: API administrativa, RBAC, bootstrap do primeiro admin, validação de configuração, persistência, auditoria, geração de JSON para Caddy, integrações DNS, ACME, CrowdSec, notificações e jobs.

Web: painel administrativo responsivo em Svelte 5 com Bootstrap real e i18n para multiplos idiomas.

Caddy: terminação TLS, reverse proxy, roteamento, headers de segurança, rate limiting, bloqueios e integração com CrowdSec.

PostgreSQL: armazenamento persistente de usuários, roles, aplicações, upstreams, regras, DNS, certificados, auditoria e eventos.

Redis: sessões, cache, locks distribuídos, filas leves, rate limiting distribuído e estado temporário.

CrowdSec: análise de reputação, decisões de bloqueio e enriquecimento de eventos.

PowerDNS: DNS autoritativo integrado ou externo por API.

PowerAdmin: administração operacional opcional do PowerDNS integrado.

## 4. Fluxo de Uma Requisição HTTP

O cliente acessa um domínio protegido. O Caddy recebe a requisição, normaliza IP real conforme `trusted_proxies`, aplica regras de bloqueio, rate limit, headers, challenge ou proxy. Quando a requisição é permitida, o Caddy encaminha para um upstream saudável conforme a política da aplicação. Eventos relevantes são enviados para logs e posteriormente correlacionados pelo Core, Redis, PostgreSQL e CrowdSec.

## 5. Tecnologias e Bibliotecas Recomendadas

Rust: Axum para HTTP, Tokio para runtime assíncrono, SQLx para PostgreSQL com queries verificáveis, Argon2 para senhas, Serde para configuração/JSON, JsonSchema para validação, Redis crate para cache e locks, Tracing para logs estruturados.

Svelte 5: Vite, TypeScript, Svelte Check, Bootstrap 5 e catalogo i18n tipado. O painel deve usar componentes oficiais do Bootstrap em vez de recriar padrões visuais.

Infra: Docker Compose para desenvolvimento, PostgreSQL, Redis, Caddy, CrowdSec e PowerDNS.

## 6. Plugins e Módulos Recomendados Para o Caddy

`github.com/caddy-dns/cloudflare`: DNS-01 com Cloudflare usando token com escopo mínimo.

`github.com/mholt/caddy-ratelimit`: rate limiting HTTP com sliding window, zonas dinâmicas e modo distribuído via storage compartilhado.

`github.com/mholt/caddy-l4`: suporte Layer4 para bloqueio e proxy TCP/UDP quando a proteção precisar ir além de HTTP.

`github.com/caddyserver/transform-encoder`: encoder estruturado adicional para logs compatíveis com integrações operacionais.

`github.com/hslatman/caddy-crowdsec-bouncer/crowdsec`: app CrowdSec interno para consultar a Local API e manter cache de decisões.

`github.com/hslatman/caddy-crowdsec-bouncer/http`: bloqueio HTTP por decisões do CrowdSec.

`github.com/hslatman/caddy-crowdsec-bouncer/appsec`: integração AppSec do CrowdSec quando habilitada.

`github.com/hslatman/caddy-crowdsec-bouncer/layer4`: matcher Layer4 para decisões CrowdSec em conexões TCP/UDP.

`github.com/pberkel/caddy-storage-redis`: storage Redis para CertMagic/Caddy em cenários distribuídos.

Módulos nativos do Caddy: reverse proxy, active/passive health checks, headers, métricas, ACME HTTP-01 e Admin API.

## 7. Estrutura Completa do Repositório

```text
apps/
  core/
    src/
    tests/
    migrations/
    Dockerfile
  web/
    src/
    Dockerfile
services/
  caddy/
  crowdsec/
  powerdns/
  poweradmin/
config/
deploy/
  compose/
docs/
```

## 8. Modelo de Dados Inicial do PostgreSQL

O schema inicial está em `apps/core/migrations/0001_initial.sql`.

Entidades principais: `users`, `roles`, `user_roles`, `applications`, `upstreams`, `waf_rules`, `rate_limit_rules`, `dns_zones`, `dns_records`, `certificates`, `audit_events` e `security_events`.

O modelo usa UUIDs, constraints explícitas, JSONB somente para condições flexíveis e tabelas separadas para regras de segurança, rate limit e auditoria.

## 9. Estratégia de Utilização do Redis

Sessões administrativas com TTL curto. Rate limits distribuídos usando chaves compostas por tenant, aplicação, regra e identidade de cliente. Locks distribuídos para ACME, aplicação de config do Caddy, sincronização DNS e jobs. Filas leves para notificações e enriquecimento de eventos. Cache com namespace por usuário ou aplicação para evitar vazamento entre contextos.

## 10. Design da API do Core

Rotas iniciais implementadas como contrato em `apps/core/src/api.rs`:

```text
GET /api/health
POST /api/auth/login
POST /api/auth/change-password
POST /api/auth/logout
GET /api/components
GET /api/configuration
PATCH /api/configuration
POST /api/configuration/apply
POST /api/configuration/reload
GET /api/users
POST /api/users
GET /api/applications
POST /api/applications
GET /api/waf/rules
POST /api/waf/rules
GET /api/rate-limits
POST /api/rate-limits
GET /api/dns/zones
POST /api/dns/records
GET /api/certificates
GET /api/crowdsec/decisions
GET /api/audit/events
GET /api/metrics
POST /api/caddy/apply
```

A API deve retornar erros sem detalhes internos, exigir CSRF quando cookies forem usados e aplicar autorização por recurso.

## 11. Sistema de Configuração Central

`config/platform.example.yaml` contém a configuração pública. `config/secrets.env.example` lista secrets esperados por ambiente. `config/platform.schema.json` valida estrutura, tipos e chaves permitidas.

Secrets não entram no YAML público; o YAML referencia nomes de variáveis como `BAIA_POWERDNS_API_KEY`.

O objetivo operacional é que `config/platform.yaml` e `config/secrets.env` sejam o bootstrap local e também uma representação auditável da configuração efetiva. Depois do primeiro start, o Core deve ser a superfície principal de configuração, como acontece em plataformas self-hosted maduras: o painel altera estado validado, o Core grava configuração persistente, mascara secrets, audita a mudança e aplica o componente correto quando possível.

Sincronização bidirecional é obrigatória. Mudanças feitas pelo painel passam por `PATCH /api/configuration`, são validadas e persistidas de volta em `config/platform.yaml` antes do apply. Mudanças feitas manualmente no arquivo podem ser carregadas por `POST /api/configuration/reload`; o Core valida o YAML, rejeita estados inválidos sem aplicar e atualiza o painel com a versão carregada. Secrets continuam fora do YAML público e entram por `config/secrets.env` ou provider de secrets.

Cada componente tem um descritor no Core com settings, secrets, capacidades e modo de aplicação:

`HotReload`: alteração aplicada sem reiniciar container, como Caddy via Admin API JSON.

`ExternalApi`: alteração enviada para API de provider ou serviço, como PowerDNS, Cloudflare e CrowdSec.

`RestartRequired`: alteração estrutural que exige recriação ou reinício coordenado, como host/porta de PostgreSQL e Redis.

`NoRuntimeApply`: ajuste apenas visual ou já resolvido pelo cliente, como preferências locais do painel.

O painel deve consumir `GET /api/components` para renderizar módulos, estado de saúde, pendências, secrets ausentes, ações disponíveis e avisos de risco sem exigir edição manual de arquivos.

## 12. Estratégia de Autenticação e Bootstrap do Primeiro Administrador

O Core verifica se existe usuário admin. Se não existir, gera usuário `admin`, senha temporária criptograficamente aleatória e hash Argon2id. A senha é exposta apenas uma vez para log inicial. O primeiro login marca a sessão como `password_change_required`, exige troca de senha e cadastro de e-mail antes de liberar o painel.

## 13. Integração com PowerDNS

Modo integrado: Compose sobe PowerDNS Authoritative e PowerAdmin usando a mesma instância PostgreSQL da plataforma, em um banco `powerdns` separado do banco `baia`. O Core usa a API local do PowerDNS para criar zonas e registros.

Modo externo: o Core recebe URL e API key via configuração/secret e usa o mesmo cliente HTTP, com allowlist de host e timeout explícito.

A configuração operacional do PowerDNS é feita pela API HTTP nativa com `X-API-Key`. O Core deve gerenciar zonas, RRsets, DNSSEC e testes de conectividade; o PowerAdmin permanece opcional para diagnóstico manual, não como caminho principal.

## 14. Integração com Cloudflare

Cloudflare é usado para DNS gerenciado e DNS-01. O token deve ter escopo mínimo por zona: leitura de zona e edição de DNS. O painel deve separar zonas importadas de zonas gerenciadas pela plataforma.

Ao colocar um domínio no WAF, o Core pode planejar e aplicar registros A e AAAA automaticamente na zona Cloudflare, com `proxied` ativo ou desativado. Quando `proxied` estiver ativo na Cloudflare e o tráfego também passar pelo Baia WAF, o plano deve exibir aviso explícito de duplo proxy, pois uma composição incorreta pode deixar a aplicação offline ou criar loop.

O Core também mantém um catálogo de CAs ACME conhecidas para sugerir o domínio CAA correto. Para Let's Encrypt o CAA padrão é `letsencrypt.org`; para Google Trust Services é `pki.goog`; para Sectigo e ZeroSSL é `sectigo.com`. Para CAs não reconhecidas, o administrador informa manualmente o domínio CAA.

## 15. Integração com CrowdSec

CrowdSec fornece reputação e decisões. O Caddy bouncer aplica bloqueios no caminho de requisição. O Core consulta a Local API para exibição, remoção autorizada de decisões e auditoria. Logs do Caddy alimentam a coleção `crowdsecurity/caddy`.

O Core deve tratar decisões, bouncers, coleções, allowlists e estado da Local API como recursos administrativos. Tokens ficam em secrets, ações sensíveis exigem RBAC e auditoria, e o painel mostra se o bouncer do Caddy está sincronizado.

## 16. Estratégia de ACME

HTTP-01 é o padrão para certificados simples. DNS-01 é obrigatório para wildcard. O Caddy/CertMagic gerencia emissão e renovação; o Core registra estado, erros, domínio, challenge e datas no PostgreSQL. Em ambientes distribuídos, storage Redis para Caddy evita inconsistência entre instâncias.

Quando a emissão usar uma CA ACME diferente da Let's Encrypt, o Core deve planejar registros CAA `issue` e, para wildcard, `issuewild`, utilizando o catálogo de CAs conhecidas ou um domínio CAA customizado informado pelo administrador.

## 17. Sistema de Regras do WAF

Regras têm prioridade, condições e ação. Condições incluem IP, CIDR, país, continente, ASN, hostname, domínio, path, método, headers, query string, User-Agent, cookie, origem e reputação. Ações incluem permitir, bloquear, challenge, CAPTCHA, rate limit, redirect, headers, log e regra adicional.

## 18. Rate Limiting

O Core modela regras com algoritmo, janela, limite, burst e chave. Caddy aplica sliding window quando a regra puder ser expressa no módulo. Redis mantém estado distribuído para a API administrativa e locks. Casos que exigirem token bucket ou composição complexa devem ser implementados no Core ou em serviço auxiliar antes do proxy.

## 19. Load Balancing

Cada aplicação possui múltiplos upstreams com peso, prioridade, health checks, timeouts e retry policy. O Core gera configuração de `reverse_proxy` com política de seleção compatível com Caddy. O painel mostra estado dos upstreams com base nos eventos e health checks.

## 20. Páginas de Erro

Templates padrão cobrem 400, 401, 403, 404, 408, 429, 500, 502, 503, 504, bloqueio WAF, challenge e manutenção. Customizações devem ser HTML estático validado e sanitizado, sem execução de código no servidor.

## 21. Notificações e E-mail

SMTP e Resend são providers alternativos. Usos: alertas, recuperação de conta, login, eventos administrativos e segurança. Envios sensíveis usam idempotência, rate limit e auditoria.

## 22. Logging, Auditoria e Métricas

Logs estruturados com correlação por request. Auditoria em `audit_events` para ações administrativas. Segurança em `security_events`. Métricas expostas por Core e Caddy para Prometheus/Grafana posteriormente.

## 23. Docker Compose Inicial

`deploy/compose/docker-compose.yml` sobe Core, Web, Caddy, PostgreSQL, Redis, CrowdSec, PowerDNS e PowerAdmin. O PostgreSQL mantém bancos separados para a plataforma e para o PowerDNS na mesma instância. Cada serviço fica em container separado, com volumes próprios e health checks onde aplicável.

## 24. Dockerfiles Necessários

`apps/core/Dockerfile`: build Rust e runtime não-root.

`apps/web/Dockerfile`: build Bun/Svelte e serving estático via Caddy.

`services/caddy/Dockerfile`: Caddy customizado com módulos de DNS, rate limit, CrowdSec e storage Redis.

## 25. Ordem Recomendada de Implementação por Fases

1. Consolidar Core HTTP real com Axum, SQLx, migrations e health checks.
2. Implementar autenticação, bootstrap persistente, sessões Redis, CSRF e RBAC.
3. Implementar CRUD de aplicações, upstreams e geração/aplicação via Caddy Admin API.
4. Implementar regras WAF, allowlists, blocklists e eventos de segurança.
5. Implementar DNS PowerDNS integrado/externo e Cloudflare.
6. Implementar ACME HTTP-01 e DNS-01 com visualização de certificados.
7. Implementar CrowdSec com decisões, auditoria e painéis.
8. Implementar rate limiting avançado e testes de abuso.
9. Implementar páginas de erro customizáveis com sanitização.
10. Implementar notificações, métricas, dashboards e hardening de produção.
