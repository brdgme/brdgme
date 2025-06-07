# Board Game Platform Migration Plan: Rocket/React to Dioxus Monolith

## Overview

This document outlines the migration plan for consolidating the existing board game platform from a distributed architecture (Rocket API + React frontend + WebSocket server) into a unified Dioxus monolith application. The migration will also replace Diesel with SQLx and unify command parsing in Rust.

## Architecture Changes

### Current Architecture
- **API Server**: Rust/Rocket/Diesel (`rust/api`)
- **Web Frontend**: TypeScript/React (`web`)
- **WebSocket Server**: TypeScript/Node.js (`websocket`)
- **Game Engines**: Go (`brdgme-go`) and Rust (`rust/game`)
- **Communication**: Redis pub/sub between API and WebSocket servers

### Target Architecture
- **Monolith**: Rust/Dioxus/SQLx (`rust/web`)
- **Game Engines**: Unchanged (still separate processes)
- **Communication**: Direct in-process communication for real-time updates

## Prerequisites

Before starting the migration:

1. Set up PostgreSQL database with the same schema as the existing system
2. Install SQLx CLI: `cargo install sqlx-cli`
3. Configure environment variables in `rust/web/.env`:
   ```
   DATABASE_URL=postgresql://user:password@localhost/brdgme
   REDIS_URL=redis://localhost:6379
   ```
4. Set up SQLx migrations directory and baseline schema:
   ```bash
   # Create migrations directory
   mkdir -p rust/web/migrations
   
   # Copy and prepare schema dump
   cp schema.sql rust/web/migrations/001_initial_schema.sql
   ```
   - Note: SQLx uses `_sqlx_migrations` table, which won't conflict with Diesel's `__diesel_schema_migrations`

## Migration Milestones

### Milestone 1: Foundation and Database Layer (Week 1-2)

#### 1.1 SQLx Setup and Schema Migration
1. **Use existing schema dump as baseline**
   - Create `rust/web/migrations/` directory
   - Copy `schema.sql` as the first migration with modifications for idempotency
   - Future migrations will be incremental changes only
   
   **Prepare schema.sql for use as migration:**
   ```bash
   # Copy schema dump to migrations
   cp schema.sql rust/web/migrations/001_initial_schema.sql
   
   # Edit the file to make it idempotent
   # Replace all CREATE statements with CREATE IF NOT EXISTS
   sed -i 's/CREATE TABLE/CREATE TABLE IF NOT EXISTS/g' rust/web/migrations/001_initial_schema.sql
   sed -i 's/CREATE INDEX/CREATE INDEX IF NOT EXISTS/g' rust/web/migrations/001_initial_schema.sql
   sed -i 's/CREATE UNIQUE INDEX/CREATE UNIQUE INDEX IF NOT EXISTS/g' rust/web/migrations/001_initial_schema.sql
   sed -i 's/CREATE SEQUENCE/CREATE SEQUENCE IF NOT EXISTS/g' rust/web/migrations/001_initial_schema.sql
   ```
   
   **Manual adjustments needed:**
   ```sql
   -- For foreign key constraints, wrap in exception handling
   -- Replace lines like:
   -- ALTER TABLE ONLY games
   --     ADD CONSTRAINT games_game_version_id_fkey FOREIGN KEY (game_version_id) REFERENCES game_versions(id);
   
   -- With:
   DO $$
   BEGIN
       ALTER TABLE ONLY games
           ADD CONSTRAINT games_game_version_id_fkey FOREIGN KEY (game_version_id) REFERENCES game_versions(id);
   EXCEPTION
       WHEN duplicate_object THEN NULL;
   END$$;
   ```
   
   **Future migrations pattern:**
   ```sql
   -- migrations/002_add_new_feature.sql
   -- Only incremental changes from baseline
   ALTER TABLE users ADD COLUMN IF NOT EXISTS new_field TEXT;
   CREATE INDEX IF NOT EXISTS idx_users_new_field ON users(new_field);
   ```

2. **Create SQLx model structs**
   - Create `rust/web/src/models/` directory
   - Port models from `rust/api/src/db/models.rs`
   - Add SQLx derive macros:
     ```rust
     // rust/web/src/models/user.rs
     use sqlx::FromRow;
     use uuid::Uuid;
     use chrono::{DateTime, Utc};
     
     #[derive(Debug, FromRow, Clone)]
     pub struct User {
         pub id: Uuid,
         pub created_at: DateTime<Utc>,
         pub updated_at: DateTime<Utc>,
         pub name: String,
         pub pref_colors: Vec<String>,
         pub login_confirmation: Option<String>,
         pub login_confirmation_at: Option<DateTime<Utc>>,
     }
     ```



3. **Set up database connection pool**
   - Add to `rust/web/Cargo.toml`:
     ```toml
     sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "migrate"] }
     ```
   - Create `rust/web/src/db.rs`:
     ```rust
     use sqlx::postgres::PgPool;
     
     pub async fn create_pool() -> Result<PgPool, sqlx::Error> {
         let database_url = std::env::var("DATABASE_URL")
             .expect("DATABASE_URL must be set");
         let pool = PgPool::connect(&database_url).await?;
         
         // Run migrations (will skip existing tables)
         sqlx::migrate!("./migrations").run(&pool).await?;
         
         Ok(pool)
     }
     ```

#### 1.2 Dioxus Application Structure
1. **Set up Dioxus router and basic pages**
   - Create `rust/web/src/routes/` directory
   - Implement basic route structure:
     ```rust
     // rust/web/src/main.rs
     use dioxus::prelude::*;
     
     #[derive(Clone, Routable, Debug, PartialEq)]
     enum Route {
         #[route("/")]
         Home,
         #[route("/login")]
         Login,
         #[route("/games")]
         Games,
         #[route("/games/:id")]
         Game { id: String },
     }
     ```

2. **Create shared state management**
   - Implement global state using Dioxus signals:
     ```rust
     // rust/web/src/state.rs
     use dioxus::prelude::*;
     
     #[derive(Clone)]
     pub struct AppState {
         pub current_user: Signal<Option<User>>,
         pub db_pool: PgPool,
     }
     ```

### Milestone 2: Authentication System (Week 3)

#### 2.1 Port Authentication Logic
1. **Migrate auth endpoints from Rocket to Dioxus server functions**
   - Port `rust/api/src/controller/auth.rs` logic
   - Create server functions for login/logout:
     ```rust
     // rust/web/src/auth/server.rs
     #[server(Login)]
     async fn login(email: String) -> Result<LoginResponse, ServerFnError> {
         // Port login logic from Rocket controller
     }
     ```

2. **Implement session management**
   - Use Dioxus fullstack session features
   - Store auth tokens in cookies
   - Create middleware for protected routes

3. **Create login UI component**
   - Port React login component to Dioxus
   - Use Dioxus forms and validation

#### 2.2 User Management
1. **User profile pages**
   - Create/edit user profiles
   - Preference management (colors, notifications)

2. **Friends system**
   - Port friend request/accept logic
   - Create friends list component

### Milestone 3: Game Listing and Management (Week 4-5)

#### 3.1 Game Type Registry
1. **Port game type management**
   - Migrate game type discovery logic
   - Create game catalog page
   - Implement game filtering/searching

2. **Game creation flow**
   - Port game creation logic from Rocket
   - Create game setup UI components
   - Player invitation system

#### 3.2 Active Games Display
1. **Games list page**
   - Show user's active games
   - Display game status and turn information
   - Implement real-time updates using Dioxus signals

2. **Game state management**
   - Port game state serialization/deserialization
   - Create game state cache using Dioxus signals

### Milestone 4: Unified Command Parser (Week 6)

#### 4.1 Extend Rust Command Parser
1. **Add partial parsing support**
   - Modify `rust/lib/cmd` to support incomplete commands
   - Implement autocomplete suggestions:
     ```rust
     // rust/web/src/command/parser.rs
     pub struct ParseResult {
         pub kind: MatchKind,
         pub offset: usize,
         pub length: Option<usize>,
         pub suggestions: Vec<Suggestion>,
         pub value: Option<String>,
     }
     
     pub enum MatchKind {
         Full,
         Partial,
         Error(String),
     }
     ```

2. **Port TypeScript command spec features**
   - Implement all command spec types from `web/src/command.ts`
   - Add player name autocomplete
   - Add command documentation support

#### 4.2 Command UI Component
1. **Create command input component**
   - Real-time parsing feedback
   - Autocomplete dropdown
   - Command history

2. **Integrate with game view**
   - Connect to game state
   - Submit commands via server functions

### Milestone 5: Game Client Integration (Week 7-8)

#### 5.1 Game Process Communication
1. **Port game client communication**
   - Migrate from `rust/api/src/game_client.rs`
   - Implement process spawning and IPC
   - Handle game engine responses

2. **Game state synchronization**
   - Update game state in database
   - Broadcast updates to connected clients

#### 5.2 Game Rendering
1. **Port game rendering logic**
   - Migrate `rust/api/src/render.rs`
   - Create Dioxus components for game display
   - Implement markup rendering for game logs

2. **Game-specific UI components**
   - Board visualization
   - Player status displays
   - Move history

### Milestone 6: Real-time Communication (Week 9-10)

#### 6.1 WebSocket Replacement
1. **Implement Server-Sent Events (SSE) or WebSockets in Dioxus**
   - Replace Node.js WebSocket server functionality
   - Use Dioxus's built-in real-time capabilities
   - Implement connection management

2. **Real-time game updates**
   - Push game state changes to clients
   - Update UI reactively using signals
   - Handle connection recovery

#### 6.2 Chat System
1. **Port chat functionality**
   - Migrate chat models and logic
   - Create chat UI components
   - Implement real-time message delivery

2. **Chat integration**
   - In-game chat
   - Global chat rooms
   - Private messaging

### Milestone 7: Additional Features (Week 11-12)

#### 7.1 Game History and Stats
1. **Game logs and replay**
   - View completed games
   - Step through game history
   - Export game logs

2. **Statistics and ratings**
   - Player statistics pages
   - Rating calculations
   - Leaderboards

#### 7.2 Admin Features
1. **Game administration**
   - Pause/resume games
   - Handle disputes
   - Game version management

2. **User administration**
   - User management interface
   - Moderation tools

### Milestone 8: Testing and Migration (Week 13-14)

#### 8.1 Testing Strategy
1. **Unit tests**
   - Test all server functions
   - Test command parser
   - Test game client communication

2. **Integration tests**
   - Full game flow tests
   - Authentication flow tests
   - Real-time communication tests

#### 8.2 Data Migration
1. **Database Compatibility**
   - SQLx migrations using `IF NOT EXISTS` are safe to run on existing databases
   - Existing production database can be used directly
   - Test migrations on a copy of production first
   
   **Pre-migration testing:**
   ```bash
   # Backup production database
   pg_dump production_db > backup.sql
   
   # Create test database from backup
   createdb test_migration
   psql test_migration < backup.sql
   
   # Test SQLx migrations (baseline should skip existing objects)
   DATABASE_URL=postgresql://user:pass@localhost/test_migration sqlx migrate run
   
   # For fresh database test
   createdb test_fresh
   DATABASE_URL=postgresql://user:pass@localhost/test_fresh sqlx migrate run
   
   # Compare schemas to ensure they match
   pg_dump --schema-only test_migration > test_migration_schema.sql
   pg_dump --schema-only test_fresh > test_fresh_schema.sql
   diff test_migration_schema.sql test_fresh_schema.sql
   ```

2. **Migration validation**
   - Verify all SQLx queries compile against existing schema
   - Use `cargo sqlx prepare` to generate offline query data
   - Test application startup and basic operations

### Milestone 9: Deployment (Week 15)

#### 9.1 Production Setup
1. **Configure production environment**
   - Set up production database
   - Configure environment variables
   - Set up monitoring

2. **Deployment process**
   - Create Docker container
   - Set up CI/CD pipeline
   - Configure reverse proxy

#### 9.2 Cutover Plan
1. **Phased migration**
   - Run both systems in parallel initially
   - Gradually migrate users
   - Monitor for issues

2. **Rollback procedures**
   - Document rollback steps
   - Test rollback process
   - Keep old system available

## Technical Guidelines

### Code Organization
```
rust/web/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── home.rs
│   │   ├── auth.rs
│   │   ├── games.rs
│   │   └── game.rs
│   ├── components/
│   │   ├── mod.rs
│   │   ├── layout.rs
│   │   ├── game_list.rs
│   │   └── command_input.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── game.rs
│   │   └── chat.rs
│   ├── auth/
│   │   ├── mod.rs
│   │   └── server.rs
│   ├── game/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── server.rs
│   ├── command/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   └── spec.rs
│   ├── db.rs
│   └── state.rs
├── assets/
│   └── main.css
├── migrations/
└── Cargo.toml
```

### Best Practices

1. **Use Dioxus idioms**
   - Prefer signals over manual state management
   - Use server functions for all backend logic
   - Leverage Dioxus router for navigation

2. **SQLx best practices**
   - Use compile-time query verification
   - Implement proper connection pooling
   - Use transactions for complex operations

3. **Error handling**
   - Use `Result` types consistently
   - Implement proper error boundaries in UI
   - Log errors appropriately

4. **Performance considerations**
   - Implement pagination for large lists
   - Use lazy loading for game states
   - Cache frequently accessed data

## Risk Mitigation

1. **Gradual migration**
   - Keep existing system running during development
   - Test each milestone thoroughly
   - Have rollback plan for each phase
   - Use feature flags to enable/disable new system components

2. **Data integrity**
   - Regular backups during migration
   - SQLx migrations are non-destructive to existing data
   - Validate schema compatibility before each deployment
   - Maintain audit logs
   
   **Zero-downtime migration strategy:**
   ```
   1. Deploy Dioxus app in read-only mode
   2. Verify all read operations work correctly
   3. Enable write operations for subset of users
   4. Gradually increase user percentage
   5. Disable old system once fully migrated
   ```

3. **User communication**
   - Announce migration schedule
   - Provide migration status updates
   - Maintain support channels

4. **Database compatibility safeguards**
   - Use `IF NOT EXISTS` for all CREATE statements
   - Never DROP existing tables or columns
   - Test migrations on production snapshot first
   - Foreign keys may need special handling if they already exist

## Success Metrics

1. **Performance**
   - Page load time < 2 seconds
   - Real-time updates < 100ms latency
   - Support 1000+ concurrent users

2. **Reliability**
   - 99.9% uptime
   - Zero data loss during migration
   - Graceful error handling

3. **User satisfaction**
   - Feature parity with existing system
   - Improved user experience
   - Positive user feedback

## Conclusion

This migration plan provides a structured approach to consolidating the board game platform into a Dioxus monolith. By following these milestones and guidelines, the migration can be completed systematically while minimizing risks and ensuring a smooth transition for users.