# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Build
cargo build                    # Development build
cargo build --release          # Production build

# Code quality (CI runs these on every PR)
cargo fmt --check              # Check formatting
cargo clippy -- -D warnings    # Lint (warnings are errors)

# Testing
cargo test                     # Run all tests
cargo test --test db_tests     # Integration tests only
cargo test test_name           # Run specific test

# Run the server
cargo run                      # Default: http://127.0.0.1:3000
```

## Code Style Requirements

- **Always run `cargo clippy`** and fix all warnings before committing
- **Prefer iterator chains** over mutation (`.iter().map().filter().collect()`)
- **No dead code** without an explanatory comment
- **Use the color palette** defined in COLORS.md (highlight #F7931A sparingly, accent #8B7355 for UI elements)

## Architecture Overview

This is a full-stack Rust web application for a Bitcoin Lightning-powered treasure hunting game.

### Core Stack
- **Axum 0.7** - Web framework with tower middleware
- **Maud** - Compile-time HTML templates (all in `src/templates/`)
- **SQLx + SQLite** - Database with compile-time query checking
- **Blitzi** - Lightning Network integration (LNURL-withdraw)

### Key Components

```
src/
├── main.rs          # Router setup, middleware, server entry point
├── config.rs        # CLI args + env vars (SH_* prefix)
├── db.rs            # All database operations (29+ methods)
├── models.rs        # Data structures, enums (User, Location, etc.)
├── lightning.rs     # LightningService trait for LN operations
├── lnurl.rs         # LNURL-withdraw (LUD-03) + LN address resolution
├── ntag424.rs       # NFC tag AES decryption & CMAC verification
├── refill.rs        # Background service: distributes sats to locations
├── donation.rs      # Background service: tracks donation payments
├── auth/            # Cookie-based auth, CookieUser extractor, role checks
├── handlers/
│   ├── api.rs       # API endpoints (locations, wallet, donations, admin)
│   └── pages.rs     # Page route handlers
└── templates/       # Maud templates (one file per page/component)
```

### Data Flow
1. Request → Axum router → Auth middleware (extracts `CookieUser`)
2. Handler processes request, calls `db.rs` methods
3. Templates render HTML using Maud macros
4. Background services (refill, donation) run on Tokio tasks

### Authentication
- Cookie-based sessions stored in SQLite
- Three auth methods: Password (Argon2), OAuth (Google/GitHub), Anonymous
- Role hierarchy: User < Creator < Admin (checked via `RequireRole`)

### Configuration (environment or CLI)
- `SH_HOST`, `SH_PORT` - Server binding
- `SH_DATA_DIR` - Database, uploads, and Blitzi data location
- `SH_BASE_URL` - Public URL for LNURL callbacks
- `SH_MAX_SATS_PER_LOCATION` - Global cap per location

### Database
- SQLite with migrations in `migrations/` (13 migrations total)
- Migrations run automatically on startup via `sqlx::migrate!()`
- Use `sqlx::query!` macros for compile-time SQL verification
