# Warehouse

Warehouse aggregates release metadata from upstream projects — Mojang, PaperMC, PurpurMC,
FabricMC, QuiltMC, MinecraftForge, NeoForged, SpigotMC's Jenkins and Docker Hub — and
republishes it as a handful of small, stable JSON documents called **catalogs**.

It exists because polling those upstreams is a poor fit for the services that need the
data. A control plane that answers user requests should not also be a scraper: every
replica multiplies the upstream load, a cold start blocks on a third party being up, and
Docker Hub's anonymous rate limit is shared whether or not your processes know about each
other. Warehouse does that work once, in one place.

## What it is not

Warehouse is **not on the request path** of the systems that consume it. A consumer
fetches whole catalogs in the background and holds them in memory. If Warehouse is down,
the consumer keeps serving the last documents it fetched — indefinitely. Plan for it as a
freshness source, not as a dependency.

## The identifier invariant

**Catalogs contain version identifiers only — never download URLs, image references or
checksums.**

This is the load-bearing design rule, not a detail. Consumers build locations themselves
from constants they control, which keeps a Warehouse instance outside their trust
boundary: a compromised instance can offer a version that does not exist, but it can
never point a consumer at an attacker-controlled artifact.

Contributions that would place a fetchable location in a catalog will be declined,
however convenient. `scripts/check-invariant.sh` and the tests in
`crates/warehouse-types/tests/identifier_invariant.rs` enforce this in CI.

## Catalogs

| id | source | contents | default max age |
|---|---|---|---|
| `minecraft` | Mojang, PaperMC, PurpurMC, FabricMC, QuiltMC, MinecraftForge, NeoForged | releases with per-distribution builds | 7 days |
| `minecraft-proxy` | PaperMC, SpigotMC Jenkins | Velocity lines and BungeeCord builds | 1 day |
| `go` `node` `python` `rust` | Docker Hub official images | `major.minor.patch` releases | 1 day |
| `java` | Docker Hub `eclipse-temurin` | Temurin releases grouped by feature version | 1 day |

## Running it

```bash
cargo run -p warehouse-server
```

```bash
docker run -p 8080:8080 -v warehouse-data:/data \
  -e WAREHOUSE_TOKEN=changeme \
  -e WAREHOUSE_USER_AGENT="warehouse/0.1 (+https://example.com; ops@example.com)" \
  ghcr.io/novium-dev/warehouse:latest
```

Run **one instance**. Its state is fully rebuildable from upstream, consumers hold their
own copies so a restart is invisible, and a single process is what makes the Docker Hub
gate meaningful. Multiple replicas are safe but multiply upstream traffic for no benefit.

### Configuration

Everything comes from the environment; there is no configuration file. Startup fails
naming the first variable it cannot parse.

| variable | default | meaning |
|---|---|---|
| `WAREHOUSE_BIND` | `0.0.0.0:8080` | listen address |
| `WAREHOUSE_TOKEN` | unset | bearer token required on every route but `/health` |
| `WAREHOUSE_DATA_DIR` | `./data` | where catalogs are persisted |
| `WAREHOUSE_USER_AGENT` | `warehouse/<version>` | sent to every upstream |
| `WAREHOUSE_CATALOGS` | all | comma-separated catalogs to serve |
| `WAREHOUSE_MAX_AGE_<CATALOG>` | see table | seconds before a catalog is rebuilt |
| `WAREHOUSE_DOCKER_MIN_INTERVAL` | `3600` | minimum spacing between Docker Hub listings |
| `WAREHOUSE_REFRESH_JITTER` | `600` | upper bound of random delay added to refreshes |
| `WAREHOUSE_RETRY_BASE` | `60` | delay after the first failure, doubled per failure |
| `WAREHOUSE_RETRY_MAX` | `3600` | ceiling for that backoff |
| `WAREHOUSE_LOG` | `warehouse=info` | `tracing` filter |

### Being a good upstream citizen

Several of these APIs are run by volunteers. Before you deploy:

- **Set `WAREHOUSE_USER_AGENT`** to something identifying you, with a way to make
  contact. Anonymous scrapers are the first thing to get blocked.
- **Leave the intervals alone** unless you have a reason. The defaults are already far
  more frequent than these projects publish releases.
- Warehouse honours `Retry-After` and backs off exponentially on failure. Do not work
  around that with an external retry loop.

## API

All routes but `/health` require `Authorization: Bearer <token>` when `WAREHOUSE_TOKEN`
is set.

| route | purpose |
|---|---|
| `GET /health` | liveness; unauthenticated, and healthy before anything is resolved |
| `GET /catalog` | manifest: every held catalog with its etag, age and staleness |
| `GET /catalog/{id}` | the catalog document; supports `If-None-Match` |
| `GET /status` | per-catalog refresh state, last error and next attempt |
| `POST /catalog/{id}/refresh` | asks the refresh loop to run now; returns `202` |

Pass `?schema=N` to state the schema you were built against. An instance serving a
different one answers `409 Conflict` with an `X-Warehouse-Schema` header rather than a
body you would misread — see [docs/schema.md](docs/schema.md).

## Consuming it from Rust

```toml
warehouse-types = { version = "0.1", features = ["client"] }
```

```rust
use warehouse_types::{CatalogId, MinecraftCatalog, client::{Client, Fetched}};

let client = Client::new("https://warehouse.internal", Some(token))?;

// Hold the etag and pass it back; unchanged catalogs cost one round trip and no body.
if let Fetched::Changed { document, etag } =
    client.catalog::<MinecraftCatalog>(CatalogId::Minecraft, previous_etag.as_deref()).await?
{
    // Swap `document` into your in-process cache and keep `etag` for next time.
}
```

## Layout

- `crates/warehouse-types` — the public schema, plus the client behind the `client`
  feature. This is the crate consumers depend on.
- `crates/warehouse-resolver` — the upstream fetchers. Every upstream URL in the project
  is a constant in here.
- `crates/warehouse-server` — the binary: refresh loops, persistence and the HTTP API.

## Known gaps

- The Docker Hub catalogs read the five newest tag pages per image, so they hold recent
  releases rather than full history. Older releases fall off as new ones are published.
- Minecraft releases older than `1.7.10` are not tracked.

## License

Apache-2.0. See [LICENSE](LICENSE).
