# Warehouse

Warehouse collects release metadata from upstream projects — Mojang, PaperMC, PurpurMC,
FabricMC, QuiltMC, MinecraftForge, NeoForged, SpigotMC and Docker Hub — and serves it as
a few small JSON documents called **catalogs**.

It exists so the services that need this data don't have to scrape for it. A control
plane that answers user requests shouldn't also be a scraper: every replica multiplies
upstream load, a cold start blocks on a third party being up, and Docker Hub's anonymous
rate limit is per-address whether or not your processes coordinate.

Warehouse is **not on your request path**. Fetch catalogs in the background, hold them in
memory, and keep serving the last ones you got if Warehouse goes away.

## Catalogs

| id | source | contents |
|---|---|---|
| `minecraft` | Mojang + Paper, Purpur, Fabric, Quilt, Forge, NeoForge | releases with per-distribution builds |
| `minecraft-proxy` | PaperMC, SpigotMC | Velocity lines, BungeeCord builds |
| `java` | Docker Hub `eclipse-temurin` | Temurin releases by feature version |
| `go` `node` `python` `rust` | Docker Hub official images | `major.minor.patch` releases |

Minecraft is rebuilt weekly, everything else daily.

## Version identifiers only

**Catalogs never contain download URLs, image references or checksums** — only version
identifiers. You build locations yourself from constants you control.

That keeps a Warehouse instance out of your trust boundary: a compromised instance can
offer you a version that doesn't exist, but it can't point you at an attacker's artifact.
Changes that would put a fetchable location in a catalog get declined; `scripts/check-invariant.sh`
and `warehouse_common/tests/identifier_invariant.rs` enforce it in CI.

## Running it

```bash
docker run -p 8080:8080 -v warehouse-data:/data \
  -e WAREHOUSE_TOKEN=changeme \
  -e WAREHOUSE_USER_AGENT="warehouse/0.1 (+https://example.com; ops@example.com)" \
  ghcr.io/invalidjoker/warehouse:latest
```

Run **one instance**. Its data is fully rebuildable, consumers hold their own copies, and
a single process is what makes the Docker Hub rate-limit gate work.

Mount `/data`. Catalogs persist there, so a restart serves immediately instead of
re-scraping every upstream.

**Set `WAREHOUSE_USER_AGENT`** to something that identifies you and gives upstreams a way
to reach you. Several of them are volunteer-run and block traffic they can't attribute.

### Configuration

Environment only, no config file. Startup fails naming the first variable it can't parse.

| variable | default | meaning |
|---|---|---|
| `WAREHOUSE_BIND` | `0.0.0.0:8080` | listen address |
| `WAREHOUSE_TOKEN` | unset | bearer token required on every route but `/system/health` |
| `WAREHOUSE_DATA_DIR` | `./data` | where catalogs are persisted |
| `WAREHOUSE_USER_AGENT` | `warehouse/<version>` | sent to every upstream |
| `WAREHOUSE_CATALOGS` | all | comma-separated catalogs to serve |
| `WAREHOUSE_MAX_AGE_<CATALOG>` | see above | seconds before a catalog is rebuilt |
| `WAREHOUSE_DOCKER_MIN_INTERVAL` | `3600` | minimum spacing between Docker Hub reads |
| `WAREHOUSE_REFRESH_JITTER` | `600` | random delay added to refreshes |
| `WAREHOUSE_RETRY_BASE` | `60` | delay after first failure, doubled per failure |
| `WAREHOUSE_RETRY_MAX` | `3600` | ceiling for that backoff |
| `WAREHOUSE_LOG` | `warehouse=info` | `tracing` filter |

## API

Swagger UI at `/docs`, OpenAPI at `/docs/openapi.json`. Full schema: [docs/schema.md](docs/schema.md).

| route | purpose |
|---|---|
| `GET /system/health` | liveness; public, and healthy before anything is resolved |
| `GET /system/status` | per-catalog freshness, last error, next attempt |
| `GET /catalog` | manifest: schema, etag and staleness per catalog |
| `GET /catalog/minecraft` | the Minecraft catalog |
| `GET /catalog/minecraft-proxy` | the proxy catalog |
| `GET /catalog/java` | the Temurin catalog |
| `GET /catalog/runtime/{go\|node\|python\|rust}` | a runtime catalog |
| `POST /catalog/{id}/refresh` | rebuild now instead of waiting |

Poll `GET /catalog` and refetch only the catalogs whose `etag` changed.

## Using it from Rust

```toml
warehouse_common = { git = "https://github.com/InvalidJoker/warehouse.git", features = ["sdk"] }
```

```rust
use warehouse_common::sdk::WarehouseSDK;
use warehouse_common::service::catalog::CatalogService as _;

let client = WarehouseSDK::new(url, Some(token))?;

let manifest = client.manifest().await?;
if manifest.schema != warehouse_common::SCHEMA_VERSION {
    // the instance serves a different schema; keep what you already have
}

let catalog = client.minecraft_catalog().await?;
```

The SDK and the server are generated from the same service definitions, so routes can't
drift between them.

## Development

```bash
cargo run -p warehouse
cargo test --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features
./scripts/check-invariant.sh
```

No database, no cache — nothing to stand up first.

Three crates: `warehouse_common` (schema, service definitions, SDK — what consumers
depend on), `warehouse_resolver` (upstream fetchers; every upstream URL lives here), and
`warehouse` (the binary).

See [CLAUDE.md](CLAUDE.md) for architecture and conventions.

## Known gaps

- Docker Hub catalogs read the five newest tag pages per image, so they hold recent
  releases rather than full history.
- Minecraft releases older than `1.7.10` aren't tracked.

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
