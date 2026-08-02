# Development

## Project Structure

```
tawai/
├── lib/                       # Flutter/Dart frontend
│   ├── main.dart              # App entry point
│   ├── src/bindings/          # Rinf-generated Dart signal bindings
│   ├── ui/                    # Flutter UI pages, widgets, theme
│   ├── utils/                 # Dart utilities: settings, API, platform, IO
│   └── models/                # Data models
├── native/
│   ├── core/                  # `tawai-core` crate — domain logic library
│   │   └── src/
│   │       ├── app_context.rs # Shared state (DB pool, reqwest client, cache)
│   │       ├── audio/         # Audio analysis, fingerprinting
│   │       ├── signals/       # Signal type definitions (shared with hub)
│   │       └── utils/         # Database, library, slskd, security, encryption…
│   ├── hub/                   # hub crate — Rinf entry point
│   │   └── src/
│   │       ├── lib.rs         # tokio::main, spawns all signal listeners
│   │       ├── signals/       # Rinf signal structs (Dart ↔ Rust)
│   │       └── utils/         # Signal handlers: database, server, library, slskd
│   └── server/                # tawai-server crate — standalone axum binary
│       └── src/
│           ├── main.rs        # Server binary entry point
│           ├── lib.rs
│           ├── server.rs      # Router setup, AppState, run_server_loop
│           ├── security.rs    # JWT auth
│           ├── docs/          # API docs router
│                       └── tawai/         # REST API handlers (auth, system, download…)
├── android/                   # Android platform files
├── ios/                       # iOS platform files
├── linux/                     # Linux platform files
├── windows/                   # Windows platform files
├── web/                       # Web platform entry point (index.html)
└── test/                      # Dart tests
```

## App Startup Flow

### Desktop / Android (Full Instance)

On platforms with FFI access (`dart:io` available: desktop, Android), the Rust
binary is embedded inside Flutter via Rinf and runs in a separate thread.

1. **Dart `main()`** (`lib/main.dart:24`) initializes Flutter, then calls
   `initializeRust(assignRustSignal)` — this starts the hub crate's
   `#[tokio::main(flavor = "current_thread")]` function in a native thread.

2. **Hub `main()`** (`native/hub/src/lib.rs:20`) starts immediately:
   - Creates `AppContext` with a `reqwest::Client` and shutdown signal
   - Spawns `database_manager` — waits for `InitDatabase` Rinf signal
   - Spawns `server_listener` — waits for `StartServer` Rinf signal
   - Spawns library list/handle signal listeners
   - Spawns slskd search/download/cancel/poll signal listeners
   - Spawns encryption/keygen signal handlers

3. **Dart continues** after `initializeRust`:
   - `APIService.init()` — starts polling for server status
   - `SettingsManager.init()` — loads config from local filesystem
   - `SystemService().init()` — system-level init
   - Sends `InitDatabase(path: dbPath)` via Rinf signal → hub receives in
     `utils/database.rs:start_database_manager` → calls
     `AppContext::start_database_manager()` → creates `DatabaseManager`
     (SQLite or PostgreSQL via `sqlx`)
   - Sends `StartServer(...)` via Rinf signal → hub receives in
     `utils/server.rs:start_server_listener` → builds axum `Router` →
     starts HTTP server bound to localhost

4. **Communication**: Dart ↔ Rust exclusively through Rinf signals
   (request/response paired by unique `id` string). All database access,
   filesystem operations, and external API calls happen in Rust.

```
┌──────────────────────┐     Rinf signals      ┌──────────────────────┐
│   Flutter / Dart     │ ◄────────────────────► │  Rust (hub crate)   │
│                      │                        │                      │
│  main()              │  InitDatabase           │  database_manager    │
│  initializeRust()    │  StartServer            │  server_listener     │
│  APIService.init()   │  ListTracks/Albums/…    │  library handlers    │
│  SettingsManager     │  SlskdSearch/Download…  │  slskd handlers      │
│  SystemService       │  Encrypt/Decrypt        │  crypto handlers     │
└──────────────────────┘                        └──────────────────────┘
                                                         │
                                                         ▼
                                                  ┌──────────────────┐
                                                  │  axum HTTP       │
                                                  │  (localhost,     │
                                                  │   embedded)      │
                                                  └──────────────────┘
```

### Web (Remote Client)

On web, Dart runs in a browser and cannot use FFI (`dart:io` is unavailable).
The tawai-server binary runs as a **separate standalone process** (or remote
server). There is no embedded Rust.

1. **tawai-server** (`native/server/src/main.rs`) starts independently:
   - Loads config from filesystem / environment variables
   - Initializes database (PostgreSQL or SQLite)
   - Starts the axum HTTP server on a configurable host:port

2. **Dart `main()`** (`lib/main.dart:24`):
   - `kIsWeb` is `true`, so `initializeRust` is **skipped** — no Rinf, no FFI
   - `APIService.init()` starts polling `GET /api/tawai/system/status`
     to detect the server
   - On first load, tries cookie-based login (JWT + CSRF)

3. **All communication** goes through REST API:
   - `APIService` (`lib/utils/api_service.dart`) wraps every endpoint
     with `http` package calls
   - `baseUrl` defaults to `Uri.base.origin` (same origin in production)
   - Settings are fetched/stored on the server via API

4. **Filesystem**: `WasmIOService` (`lib/utils/src/io_service_wasm.dart`)
   implements the `IOService` interface but **throws `UnsupportedError`**
   for every filesystem operation (`getConfigDir`, `getDatabasePath`,
   `readFile`, `writeFile`, etc.). Only `getCookie()` is functional
   (reads browser cookies for CSRF token).

```
┌──────────────────────┐    REST (JSON/HTTP)    ┌──────────────────────┐
│   Flutter / Dart     │ ◄────────────────────► │  tawai-server        │
│   (browser)          │                        │  (standalone binary) │
│                      │                        │                      │
│  main()              │  GET  /system/status    │  axum HTTP server    │
│  APIService.init()   │  POST /auth/login       │  Auth (JWT)          │
│  (no initializeRust) │  GET  /library/tracks   │  Library queries     │
│  (no Filesystem IO)  │  POST /slskd/search     │  slskd proxy         │
│                      │  …                      │  …                   │
└──────────────────────┘                        └──────────────────────┘
                                                         │
                                                         ▼
                                                  ┌──────────────────┐
                                                  │  PostgreSQL /    │
                                                  │  SQLite          │
                                                  └──────────────────┘
```

**Key limitations on Web:**
- No `dart:io` — no FFI, no filesystem access, no network sockets
- No embedded Rust — all Rust logic runs in the server process
- `WasmIOService` throws on any filesystem I/O
- Cookies are used for JWT/CSRF auth instead of local config files
- Image proxying: external cover art URLs are wrapped via
  `APIService.wrapImageUrl()` to avoid CORS issues in the browser

## Crate Architecture

### `tawai-core` (`native/core`)
Domain logic library. No Rinf dependency — testable in isolation.
Provides: `AppContext`, `DatabaseManager`, library queries, audio analysis,
fingerprinting, slskd HTTP client, encryption, security utilities.

### `hub` (`native/hub`)
Rinf bridge crate. Depends on `tawai-core` and `tawai-server`.
Owns `#[tokio::main]` — the entry point for embedded Rust.
Listens for Dart signals and dispatches to core/server logic.
Thin dispatch layer — no business logic, just wiring.

### `tawai-server` (`native/server`)
Standalone axum REST API binary. Depends on `tawai-core`.
Can run in two modes:
- **Embedded**: started via hub's `StartServer` signal (Desktop/Android)
- **Standalone**: `cargo run --package tawai-server` (server deployment, or
  alongside web Flutter)

## Communication

### Desktop / Android: Rinf Signals

Every request/response pair shares a unique string `id`:

```dart
// Dart side
final id = DateTime.now().microsecondsSinceEpoch.toString();
final stream = SomeResponse.rustSignalStream.where((s) => s.message.id == id);
SomeRequest(id: id, ...).sendSignalToRust();
final result = await stream.first;
```

```rust
// Rust side
let receiver = SomeRequest::get_dart_signal_receiver();
while let Some(pack) = receiver.recv().await {
    let msg = pack.message;
    // ... process ...
    SomeResponse { id: msg.id, ... }.send_signal_to_dart();
}
```

### Web: REST API

All endpoints are under `/api/tawai/`. Authentication via `X-API-Key` header
or JWT cookie. Rate-limited (30 req/s auth, 20 req/burst global).

### Server ↔ Core (Embedded)

Direct Rust function calls. Same process, no serialization overhead.
`AppContext` is shared via `Arc` between signal handlers and server routes.

## Deployment Modes

| Mode | Platforms | Rust | Database | Communication |
|------|-----------|------|----------|---------------|
| Full instance | Desktop, Android | Embedded (hub) | Local SQLite | Rinf signals |
| Remote client | Web, Android | Standalone server | Server PostgreSQL | REST API |
| Offline | Desktop, Android | Embedded (hub) | Local SQLite | Rinf signals |
