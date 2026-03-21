# brdgme Vision

## Documentation Principles

Documentation exists to get a developer or agent productive immediately. Every
sentence must carry information a reader cannot trivially infer from the code.

- No prose for its own sake. No summaries of what the code already says.
- Capture decisions, constraints, and non-obvious behavior only.
- Prefer dense lists over paragraphs.
- A short accurate document is better than a long stale one. Delete freely.

## What brdgme Is

brdgme is a lo-fi multiplayer board gaming platform. Games are played via web
browser or email. All game output is ASCII text with color and basic decoration.
All moves are plain text commands.

Core principles that do not change:

- Play-by-email, not notify-by-email: a full game can be played from an email
  client alone.
- Accessible in network-hostile environments: if you can send and receive email,
  you can play.
- ASCII-first rendering: no images, no canvas, no WebGL.
- Text commands: moves like `play a4` or `buy 3 sackson`.
- Bot support: every game ships with at least one bot implementation.
- Open source: the platform, all dependencies, and all tooling are open source.

## Target Architecture

The target is a small always-on core with serverless game workloads, running on
managed Kubernetes.

```mermaid
graph TD
    Browser[Browser]
    Monolith[Axum/Leptos Monolith]
    PG[(PostgreSQL)]
    NATS[NATS Core]
    Knative[Knative Services\ngame microservices]
    CRDs[GameVersion CRDs]
    Operator[brdgme Operator]

    Browser -->|HTTP + WebSocket| Monolith
    Monolith <-->|queries| PG
    Monolith <-->|pub/sub fan-out| NATS
    Monolith -->|game commands| Knative
    CRDs --> Operator
    Operator -->|upserts game_types + game_versions| PG
```

### Always-On Core

The Rust monolith (`rust/web`, Axum + Leptos) handles:

- User authentication and sessions.
- Game orchestration: creating games, enforcing turns, routing commands.
- Real-time WebSocket updates.
- Web frontend: server-side rendering with WASM hydration.

The monolith runs as multiple replicas for resilience. WebSocket fan-out across
replicas is handled by NATS Core pub/sub (in-cluster). NATS Core is sufficient
here - persistence is not required, as clients reconnect and fetch full state
on reconnect.

### Serverless Game Services

Each game type runs as a Knative Service (scale-to-zero). The monolith routes
commands to the appropriate service via the JSON contract defined in
`ARCHITECTURE.md`. Game services are stateless: they receive the full game
state per request and return the new state.

Scale-to-zero is appropriate because most game types are inactive at any given
time. The contract is stable and does not change.

### brdgme Kubernetes Operator

A custom Kubernetes operator (Rust, `kube-rs`) bridges Kubernetes and the
application database without the core API having any knowledge of Kubernetes:

- Watches `GameVersion` custom resources (`gameversions.brdgme.com/v1`).
- Upserts `game_types` and `game_versions` rows in PostgreSQL on reconcile.
- Uses finalizers to guarantee `is_public = false` is written before a resource is deleted.
- `is_deprecated: true` on a CR keeps the service running for in-progress games
  but excludes it from new game creation.

Long-term goal: operator also manages the Knative Service lifecycle for each
game version (currently game services are plain Deployments managed by Tilt).

## Infrastructure

- **Platform**: DigitalOcean Kubernetes, Sydney region (SYD1).
- **CNI**: Cilium (default on DOKS, no additional setup required).
- **Serverless runtime**: Knative Serving.
- **Database**: PostgreSQL.
- **Message bus**: NATS Core (in-cluster).
- **Ingress**: Kourier (Knative's networking layer), single DO load balancer.

### Domain routing

All services are Knative Services. Routing uses Knative `DomainMapping` to
assign custom hostnames. Kourier routes by hostname; no separate nginx Ingress
is needed.

| Domain | Knative Service | Notes |
|---|---|---|
| `brdg.me` | `web` | Leptos monolith, always on (minScale: 1) |
| `legacy.brdg.me` | `web-legacy` | Legacy React frontend, side-by-side only |
| `api.brdg.me` | `api` | Legacy Rocket API, side-by-side only |
| `ws.brdg.me` | `websocket` | Legacy Node.js WS, side-by-side only |

Legacy services (`web-legacy`, `api`, `websocket`) are removed after cutover.
TLS via cert-manager on each `DomainMapping`.

Estimated baseline cost: ~$63/month (3x 2GB nodes + load balancer + PostgreSQL
storage minimum).

## What is Removed

The following legacy services are removed after cutover:

- `rust/api`: Rocket API server (replaced by `rust/web`).
- `web`: React/Redux/Webpack frontend (replaced by Leptos in `rust/web`).
- `websocket`: Node.js WebSocket service (replaced by NATS + monolith).
- Redis: previously required for WebSocket fan-out (replaced by NATS Core).

## Planned Features (Long-Term, Out of Scope for Current Migration)

### Email

- Outbound: game notifications and invitations via a third-party provider
  (Mailgun, Postmark, or similar). No self-hosted SMTP.
- Inbound: play-by-email via provider webhook. The provider receives the reply
  email and POSTs it to a Knative Service endpoint, which parses the command
  and submits it to the game.
- This replaces the legacy Go SMTP service, which had persistent deliverability
  problems.

### Bots

- LLM-based. System prompt: brdgme context + game rules + command grammar.
  User prompt: current game state + available command spec from the game
  service.
- Invoked as a Knative Service (scale-to-zero).
- Constrained generation (grammar-based output) to ensure bot moves always
  produce valid commands.
- Initial implementation: external LLM API (Groq or similar) for fast
  iteration.
- Long-term target: Ollama in-cluster on CPU inference. Latency of 30-60
  seconds per move is acceptable for async turn-based play.
