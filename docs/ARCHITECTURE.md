# Architecture

`brdgme` is a platform for playing board games via the web or email, using
lo-fi ASCII rendering and plain text commands.

## System Overview

The platform consists of a small always-on core (the Rust monolith) and
independently deployed game microservices running as plain Kubernetes
Deployments (always on - their idle footprint is negligible). The monolith is
the only component that communicates with clients directly.

```mermaid
graph TD
    Browser[Browser]
    Monolith[Axum/Leptos Monolith]
    PG[(PostgreSQL)]
    NATS[NATS Core]
    Games[Game service Deployments]
    Operator[brdgme Operator]
    CRDs[GameVersion CRDs]

    Browser -->|HTTP + WebSocket| Monolith
    Monolith <-->|queries| PG
    Monolith <-->|pub/sub fan-out| NATS
    Monolith -->|game commands| Games
    CRDs --> Operator
    Operator -->|upserts game_types + game_versions| PG
```

## Core Components

### Monolith (`rust/web`)

**Language:** Rust
**Framework:** Axum (backend), Leptos (frontend, SSR + WASM hydration)

Handles:
- User authentication and sessions.
- Game orchestration: creating games, enforcing turns, routing commands.
- Real-time WebSocket updates (NATS Core pub/sub for cross-replica fan-out).
- Web frontend served via SSR with client-side WASM hydration.

Runs as multiple replicas. Clients connect via a single load balancer and hold
one WebSocket connection to whichever replica they land on. NATS ensures game
updates published by any replica reach all connected clients for that game.

### Game Services

Each game type is a standalone stateless microservice deployed as a plain
Kubernetes Deployment + Service, always on. The monolith communicates with
game services via the JSON contract defined in this document. (Knative
scale-to-zero was dropped 2026-07-03 - see `docs/VISION.md` for rationale.)

Game services are polyglot for now: 17 games are implemented in Go
(`brdgme-go/`) and the rest in Rust (`rust/game/`). The contract is
language-agnostic, but the Go games are being rewritten in Rust and the Go
stack removed once conversions finish (see `docs/VISION.md` and plan #31).

### brdgme Operator (`rust/operator`)

**Language:** Rust
**Framework:** kube-rs

Bridges Kubernetes infrastructure and the application database. The core API
has no knowledge of Kubernetes.

- Watches `GameVersion` custom resources (`gameversions.brdgme.com/v1`).
- Each CR represents one deployed game version (e.g. `acquire-1`, `lost-cities-2`).
- Upserts `game_types` and `game_versions` rows in PostgreSQL on reconcile.
- Uses Kubernetes finalizers to guarantee `is_public = false` is written
  to the database before a `GameVersion` resource is deleted.
- `is_deprecated: true` on a CR keeps the service running for in-progress games
  but excludes it from new game creation.
- Performs a full reconciliation on startup to recover from state drift.

## Data Flow: Game Move

```mermaid
sequenceDiagram
    participant Browser
    participant Monolith
    participant DB as PostgreSQL
    participant Game as Game Service
    participant NATS

    Browser->>Monolith: POST /api/game/{id}/command
    Monolith->>DB: fetch game state
    Monolith->>Game: POST command + state (JSON)
    Game-->>Monolith: new state + logs (JSON)
    Monolith->>DB: save new state
    Monolith->>NATS: publish game.{id} update
    NATS-->>Monolith: fan-out to all replicas
    Monolith-->>Browser: push via WebSocket
```

## Infrastructure

See `docs/VISION.md` for infrastructure choices and rationale.

- **Platform**: DigitalOcean Kubernetes (SYD1), provisioned via OpenTofu
- **CNI**: Cilium
- **Message bus**: NATS (in-cluster, JetStream enabled)
- **Database**: PostgreSQL (CloudNativePG operator)
- **Ingress**: DOKS managed Gateway API (Cilium)
- **DNS**: external-dns (DigitalOcean provider)

## Game Interface Contract

Communication between the monolith and game services is strictly HTTP/JSON.
The monolith sends a request object; the game service returns a response
object. This contract is stable and must not change.

All in-cluster callers reach game services through the KEDA HTTP
interceptor, which routes on a `Host: {version_name}.games.internal`
header. The shared crate `rust/lib/game_client` (`brdgme_game_client`)
owns this convention plus retry/backoff; web, bot, and the operator all
call game services through it. Never hand-roll a game service HTTP call -
a request without the Host header gets a 404 from the interceptor.

### Common Structures

**GameResponse:**
```json
{
  "state": "string (serialized internal game state)",
  "points": [0.0, 1.0],
  "status": {
    "Active": { "whose_turn": [0], "eliminated": [] },
    "Finished": { "placings": [0, 1], "stats": [] }
  }
}
```

**Log:**
```json
{
  "content": "string (markup)",
  "at": "timestamp",
  "public": true,
  "to": []
}
```

### Methods

#### New Game

Initialize a new game instance.

- **Request:** `{"New": {"players": 2}}`
- **Response:**
  ```json
  {
    "New": {
      "game": GameResponse,
      "logs": [Log],
      "public_render": { "pub_state": "...", "render": "..." },
      "player_renders": [{ "player_state": "...", "render": "...", "command_spec": {} }]
    }
  }
  ```

#### Get Status

Retrieve current status and renders for an existing game state.

- **Request:** `{"Status": {"game": "serialized_state_string"}}`
- **Response:**
  ```json
  {
    "Status": {
      "game": GameResponse,
      "public_render": { ... },
      "player_renders": [ ... ]
    }
  }
  ```

#### Make Move

Execute a player command.

- **Request:**
  ```json
  {
    "Play": {
      "player": 0,
      "command": "play card 1",
      "names": ["Alice", "Bob"],
      "game": "serialized_state_string"
    }
  }
  ```
- **Response:**
  ```json
  {
    "Play": {
      "game": GameResponse,
      "logs": [Log],
      "can_undo": true,
      "remaining_input": "",
      "public_render": { ... },
      "player_renders": [ ... ]
    }
  }
  ```

#### Player Counts

Get valid player counts for the game type.

- **Request:** `"PlayerCounts"`
- **Response:** `{"PlayerCounts": {"player_counts": [2, 3, 4]}}`

#### Rules

Get the game's rules text (markdown; empty string if the game provides
none). Sent by the operator on every reconcile, so all game services must
answer it. Note the serde encoding: unit requests like this and
`PlayerCounts` are bare JSON strings, not objects.

- **Request:** `"Rules"`
- **Response:** `{"Rules": {"rules": "..."}}`

#### Data Docs (V2)

Get the field dictionary describing the structured YAML states returned
in `pub_state` and `player_state`. Only served by V2 games; V1 games
return an empty string.

- **Request:** `"DataDocs"`
- **Response:** `{"DataDocs": {"data_docs": "..."}}`

#### Basic Strategy (V2)

Get hard rules against obviously bad moves (short, absolute don'ts).

- **Request:** `"BasicStrategy"`
- **Response:** `{"BasicStrategy": {"basic_strategy": "..."}}`

#### Advanced Strategy (V2)

Get optimal-play heuristics (longer, contextual guidance).

- **Request:** `"AdvancedStrategy"`
- **Response:** `{"AdvancedStrategy": {"advanced_strategy": "..."}}`

## Database Schema

Key tables in PostgreSQL:

- **`users`**: User identities, credentials, and preferences.
- **`game_types`**: Game type identities (e.g. "Lost Cities"). Managed by the operator.
- **`game_versions`**: Deployed game versions. Managed by the operator. Includes
  `is_public`, `is_deprecated`, and `interface_version` (1 or 2) flags;
  unique constraint on `(game_type_id, name)`.
- **`games`**: Active and finished game instances. Stores the serialized
  `game_state` blob.
- **`game_players`**: Links `users` to `games`, storing player position and
  player-specific state.
- **`game_logs`**: Immutable history of all actions and messages within a game.
- **`bots`**: Bot configurations (name, model, provider, thinking budget,
  temperature, docs flags). Three-layer enable gate (bot + provider +
  binding). Seeded with easy/medium/hard.
- **`llm_providers`**: LLM provider credentials (name, base_url,
  api_key_encrypted via AES-256-GCM, model). Enable flag.
- **`bot_providers`**: Join table binding bots to providers with priority
  (round-robin within priority, failover across priorities). Enable flag.
