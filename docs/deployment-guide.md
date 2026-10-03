# Deployment Guide: Arktos Wallet

This guide outlines the deployment strategy for the Arktos Wallet HTTP MCP server, focusing on containerization using Docker.

## Containerization with Docker

The project utilizes a multi-stage Docker build process to create efficient and secure production-ready images. This approach separates the dependency caching and build steps from the final runtime image, resulting in smaller image sizes and improved security.

### Docker Strategy

The [`Dockerfile`](../Dockerfile) has three stages:

1.  **Planner**: Uses [`cargo-chef`](https://github.com/LukeMathWalker/cargo-chef) to compute a dependency recipe.
2.  **Builder**: Cooks (pre-builds) dependencies as a cached layer, then builds the `arktos-wallet` binary with `--locked`. The `rust:<version>` base image matches the toolchain pinned in `rust-toolchain.toml`.
3.  **Runtime**: Copies the binary into `gcr.io/distroless/cc-debian13:nonroot` and runs it as the non-root user (UID 65532), exposing port 8080.

The image has no shell and defines no Docker `HEALTHCHECK`; configure your orchestrator to probe `GET /healthz` (liveness) and `GET /readyz` (readiness: database accessible).

The image sets `DATABASE_PATH=/data/arktos.db`; `/data` is owned by the runtime user (UID 65532), and a named volume mounted there inherits that ownership.

CI builds the image and lints the Dockerfile with hadolint on every push and pull request (`make docker-build` / `make docker-lint` locally).

## Running with Docker

To build the Docker image:

```sh
docker build -t arktos-wallet .
```

To run the Docker container (example, adjust port mapping as needed):

```sh
docker run -p 8080:8080 arktos-wallet
```

## Configuration

| Variable | Required | Description |
|----------|----------|-------------|
| `ADMIN_API_KEY` | yes | Key for the `/admin/*` API |
| `MASTER_KEY` | yes | 32 random bytes, base64 (`make secret`). Root of the HKDF key hierarchy: API-key HMAC and wallet-seed encryption keys. Losing it makes wallets unrecoverable. |
| `DATABASE_KEY` | yes | SQLCipher database key. Generate independently of `MASTER_KEY` (`make secret`); must not be equal to it. |
| `DATABASE_PATH` | no | Database file. Image default `/data/arktos.db`; outside the image `data/arktos.db` relative to the working directory. Missing parent directories are created (mode `0700`); the file is `0600`. |
| `MCP_ALLOWED_HOSTS` | no | Comma-separated `Host` values accepted on `/mcp` (default `localhost,127.0.0.1,::1`). Set it to the hostname(s) clients use, e.g. `wallet.example.com`; other hosts get `403` (DNS-rebinding protection). |
| `RUST_LOG` | no | Log filter, e.g. `info` |

## Generating Secrets

```sh
make secret   # prints 32 random bytes from the OS RNG, base64-encoded
```

Run it once for `MASTER_KEY` and once for `DATABASE_KEY`. Store both in
your secret manager and back them up **separately from the database**: without
`DATABASE_KEY` the database cannot be opened, and without `MASTER_KEY` the
wallet recovery phrases inside it cannot be decrypted.

## Persistence

```sh
docker volume create arktos-data
docker run -d -p 8080:8080 -v arktos-data:/data \
  -e ADMIN_API_KEY=… -e MASTER_KEY=… -e DATABASE_KEY=… arktos-wallet
```

- Migrations run automatically at startup; `docker run … arktos-wallet migrate`
  applies them without starting the server, and `… arktos-wallet db-info`
  prints diagnostics (no secrets).
- The database runs in WAL mode: the volume contains `arktos.db`,
  `arktos.db-wal` and `arktos.db-shm`. Keep them together.
- **Backups**: run `VACUUM INTO '/path/backup.db';` from a keyed `sqlcipher`
  shell for a consistent online copy (encrypted with the same key), or stop the
  container and copy all three files. The `sqlcipher` `.backup` command does not
  work with encrypted databases. See
  [Architecture — Files, Permissions and Backups](./architecture.md#files-permissions-and-backups).

## MCP and Scaling

`/mcp` implements MCP `2026-07-28` over the stateless Streamable HTTP transport:
there are no MCP sessions, so no sticky sessions or session store are needed
in front of Arktos.

The persistence layer is a local SQLCipher (SQLite) file, so deploy **a single
Arktos instance per database** on a persistent local or block-storage volume.
Do not point several containers at the same database file or put it on a
network file system: that configuration is not supported. Multiple instances
would require a different persistence and signing architecture.

## Shutdown

Arktos shuts down gracefully on `SIGTERM` (sent by `docker stop` and
Kubernetes) and on `SIGINT`/Ctrl+C: it stops accepting connections, ends
in-flight MCP streams and exits.
