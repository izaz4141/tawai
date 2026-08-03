<div align="center">
  <a href="https://github.com/izaz4141/tawai">
    <img src="assets/icons/tawai-filled.svg" alt="Tawai Logo" width="150">
  </a>
  <h3>Tawai</h3>
  <p>Music Discovery, Metadata Management, and Player</p>
  <p>
    <a href="https://github.com/izaz4141/tawai/releases"><img src="https://img.shields.io/github/v/release/izaz4141/tawai?style=for-the-badge&labelColor=101418&color=9ccbfb" alt="GitHub release"></a>
    <a href="https://github.com/izaz4141/tawai/blob/main/LICENSE.md"><img src="https://img.shields.io/github/license/izaz4141/tawai?style=for-the-badge&labelColor=101418&color=b9c8da" alt="GitHub License"></a>
    <a href="https://github.com/izaz4141/tawai/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/izaz4141/tawai/build.yml?branch=main&style=for-the-badge&labelColor=101418&label=BUILD" alt="Build Status"></a>
  </p>

  <p>
    <a href="#features">Features</a> •
    <a href="#screenshots">Screenshots</a> •
    <a href="#roadmap">Roadmap</a> •
    <a href="#installation">Installation</a> •
    <a href="#architecture">Architecture</a> •
    <a href="#contributing">Contributing</a> •
    <a href="#license">License</a>
  </p>

</div>

Tawai is an open-source music player and discovery tool. It combines automatic audio fingerprinting, metadata lookup from public databases, and ListenBrainz scrobbling/recommendations into a single cross-platform application. Built with a Rust backend for performance and a Flutter frontend for a responsive UI.

## Features

- **Music Discovery**: Fetch metadata and cover art via AcoustID + MusicBrainz. Integrate with ListenBrainz for personalized recommendations and scrobbling.
- **Library Management**: Scan local music directories, detect duplicates via fingerprinting, and automatically tag your files.
- **Audio Analysis**: Chromaprint-based fingerprinting for accurate track identification.
- **Cross-Platform**: Linux, Windows, Android, and Web — one codebase.
- **Modern UI**: Material Design 3 interface built with Flutter.
- **Scrobbling**: Automatic ListenBrainz scrobbling with user token configuration.
- **SQLite / PostgreSQL**: Local standalone mode uses SQLite; server mode uses PostgreSQL.
- **Remote Control**: Standalone Axum REST API server for remote clients and web access.
- **slskd Integration**: Connect to a slskd instance for Soulseek music discovery.

## Screenshots

<details close>
  <summary>Desktop</summary>
  <img src="assets/preview/desktop-library.png" alt="Tawai Desktop Library Preview" width="1200">  
</details>

<details close>
  <summary>Mobile</summary>
  <img src="assets/preview/mobile-library.png" alt="Tawai Mobbile Library Preview" width="200">  
</details>

## Roadmap

- [x] Audio fingerprinting (Chromaprint)
- [x] MusicBrainz + AcoustID metadata lookup
- [x] ListenBrainz scrobbling
- [ ] gapless playback
- [x] ReplayGain support

## Installation

### From Source

Prerequisites: Flutter 3.38.5 (FVM-managed), Rust toolchain, and FVM. See [DEVELOPMENT](DEVELOPMENT.md) for details.

1.  **Clone the repository:**
    ```sh
    git clone https://github.com/izaz4141/tawai.git
    cd tawai
    ```

2.  **Install Flutter and generate bindings:**
    ```sh
    fvm install
    cargo install rinf_cli
    rinf gen
    ```

3.  **Set required environment variables:**
    ```sh
    export TAWAI_SERVER_MASTER_KEY=$(openssl rand -hex 32) # required (64 hex chars)
    export TAWAI_DATABASE_URL=postgresql://user:pass@localhost/tawai # optional; defaults to SQLite
    ```

4.  **Build for your platform:**
    ```sh
    # Linux (x64)
    fvm flutter build linux --release --target-platform linux-x64

    # Windows
    fvm flutter build windows --release

    # Android (ARM64)
    fvm flutter build apk --release --target-platform android-arm64

    # Web
    fvm flutter build web --release
    ```

### Docker

```sh
curl -o docker-compose.yml https://raw.githubusercontent.com/izaz4141/tawai/refs/heads/main/docker-compose.yml
# Set the required master key first (see docker-compose.yml / .env)
echo "TAWAI_SERVER_MASTER_KEY=$(openssl rand -hex 32)" >> .env
docker compose up -d
```

The web UI is served at http://localhost:3000 and proxies `/api/` to the Rust server. Set `TAWAI_SLSKD_URL`/`TAWAI_SLSKD_API_KEY` and `TAWAI_NADEKODON_URL`/`TAWAI_NADEKODON_API_KEY` to connect downstream download sources.

## Architecture

```
native/
├── hub/           Rinf entry point. Dart↔Rust signal dispatch.
├── core/          Domain logic. Fingerprinting, metadata APIs, DB access.
└── server/        Standalone Axum REST API server.
```

- **hub** depends on `core` and `server`.
- **core** is a reusable library (no Rinf dependency, testable in isolation).
- **server** is a binary crate that can run standalone or embedded via hub.

| Direction | Mechanism |
|-----------|-----------|
| Dart → Rust | Rinf signal (request with unique ID) |
| Rust → Dart | Rinf signal (response with matching ID) |
| Remote client ↔ Server | REST API (JSON over HTTP) |

## Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `TAWAI_SERVER_MASTER_KEY` | Yes (server) | 64-hex master key; encrypts API keys + signs JWTs |
| `TAWAI_HOME` | No | Base data folder (`config/`, `data/`, `logs/`); default `/home/tawai` |
| `TAWAI_SERVER_HOST` | No | API bind address; default `127.0.0.1` (`0.0.0.0` in Docker) |
| `TAWAI_SERVER_PORT` | No | API port; default `8181` |
| `TAWAI_DATABASE_URL` | No | `postgresql://`/`postgres://` → PostgreSQL, else SQLite at `{TAWAI_HOME}/data/tawai.db` |
| `TAWAI_SLSKD_URL` | No | slskd server URL (e.g. `http://slskd:5030`) |
| `TAWAI_SLSKD_API_KEY` | No | slskd API key |
| `TAWAI_NADEKODON_URL` | No | Nadeko~don server URL |
| `TAWAI_NADEKODON_API_KEY` | No | Nadeko~don API key |

env vars override the values stored in `config.json` on every startup. ListenBrainz tokens are stored per-user in the database and are not read from the environment.

## Contributing

See [**DEVELOPMENT**](DEVELOPMENT.md) for detailed setup and code standards.

## License

This project is licensed under the AGPL-3.0 License. See [**LICENSE**](LICENSE.md) for details.
