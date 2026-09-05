# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**Warehouse** aggregates release metadata from upstream projects — Mojang, PaperMC, PurpurMC, FabricMC, QuiltMC, MinecraftForge, NeoForged, SpigotMC's Jenkins and Docker Hub — and republishes it as a handful of small, stable JSON documents called **catalogs**.

It was extracted from the novium backend (`hosting-system`), whose templates used to scrape those upstreams themselves. That was wrong in three ways: every backend replica multiplied the upstream load, a cold start blocked on third parties being reachable, and Docker Hub's anonymous rate limit is per-address whether or not your processes coordinate. Warehouse does that work once, in one place. It is open source and licensed **AGPL-3.0-only** (forced by zelus, which is AGPL).

A Cargo workspace, Rust edition 2024, three crates:

- **`warehouse_common/`** — the public schema (`types/`), the zelus service definitions (`service/`), and the SDK (`sdk/`). This is what consumers depend on. Same `service`/`sdk` feature split as `database_agent_common` in hosting-system.
- **`warehouse_resolver/`** — the upstream fetchers. Every upstream URL in the project is a constant in here.
- **`warehouse/`** — the binary: refresh loops, disk persistence, the HTTP router.

## Commands

```bash
cargo check --workspace
cargo run -p warehouse                     # configured entirely from WAREHOUSE_* env vars
cargo test --workspace --all-features

# Lint & format (CI enforces both; the clippy config is inherited from hosting-system
# and is just as strict)
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features

# The identifier invariant (see below) has its own check
./scripts/check-invariant.sh
```

There is **no database and no Valkey** — nothing to stand up before building. Unlike hosting-system, `cargo check` needs no live services.

Swagger UI is at `/docs`, the OpenAPI document at `/docs/openapi.json`.

```bash
docker build -t warehouse:local .
```

CI publishes `ghcr.io/invalidjoker/warehouse` on every push to `main` (as `latest` and
`main`) and on `v*` tags (as semver). Pull requests build the image but never push it, so
a fork cannot publish. Both architectures are built on native runners rather than under
QEMU, then merged into one manifest by digest.

## The identifier invariant

**Catalogs contain version identifiers only — never download URLs, container image references, checksums, or anything else fetchable.**

This is the load-bearing design rule of the project, not a detail. Consumers build locations themselves from constants they control, which keeps a Warehouse instance *outside* their trust boundary: a compromised instance can offer a version that does not exist, but it can never point a consumer at an attacker-controlled artifact. Since catalogs feed `ServerBuildPayload` in the backend — docker images and download URLs — breaking this would turn Warehouse into a supply-chain vector.

Reject any change that would place a fetchable location in a catalog, however convenient it seems. Two things enforce it, and both must keep passing:

- `scripts/check-invariant.sh` greps `warehouse_common/src/types` for URL literals and location-shaped field names.
- `warehouse_common/tests/identifier_invariant.rs` walks a serialized document of every catalog type.

Upstream URLs live in `warehouse_resolver` as constants and stay there.

## Architecture

**`Warehouse` (`warehouse/src/state.rs`)** is the cloneable shared-state handle: the loaded config, the per-catalog map, the `Store`, the shared HTTP client, and the Docker Hub gate. It implements the service traits and is registered as an axum `Extension`.

**Services (`warehouse_common/src/service/`)** are zelus service traits, exactly like hosting-system's. Both sides of the wire are generated from them — the server implements them, the SDK calls them, the OpenAPI document is derived from them — so a route cannot drift between server, client and docs. Routes without `no_auth` land in the `with_auth` group, which `Warehouse::router` puts behind a bearer-token middleware; `/system/health` is the only public route.

Catalogs are served by **typed routes** (`/catalog/minecraft`, `/catalog/java`, `/catalog/runtime/{runtime}`) rather than one string-id endpoint, so the OpenAPI schemas and the SDK are precise.

**Catalog state** — each catalog has a `RwLock<Option<Held>>` (the document, its etag, its `updated_at`), a `Status`, and a `Notify` used to force a refresh. Exactly one task owns each catalog, so no locking or coordination is needed and `POST /catalog/{id}/refresh` is just a wake-up, never a second concurrent resolve.

**Scheduling (`warehouse/src/scheduler.rs`)** — one loop per catalog. A catalog is rebuilt when it is missing or past its max age (7 days for `minecraft`, 1 day for the rest, each overridable). Every non-immediate delay gets jitter so instances started together do not synchronize on upstream, and failures back off exponentially from `WAREHOUSE_RETRY_BASE` to `WAREHOUSE_RETRY_MAX`.

**Persistence (`warehouse/src/store.rs`)** — catalogs are written to `WAREHOUSE_DATA_DIR` as a versioned envelope, atomically (temp file + rename). This matters more than it looks: without it a restart re-resolves everything immediately, so a crash loop becomes a burst of upstream traffic — exactly what gets an instance blocked. A stored document whose schema does not match is ignored rather than served.

**Docker Hub gate (`warehouse_resolver/src/docker.rs`)** — five catalogs read from Docker Hub, which rate limits anonymous requests per address. `DockerGate` serializes those reads behind a mutex held across the request and enforces `WAREHOUSE_DOCKER_MIN_INTERVAL` between them. This is why running **one replica** is the right call: the gate is process-local, and there is no shared store to coordinate through. State is fully rebuildable and consumers hold their own copies, so a restart is invisible.

**Resolver error handling** — the Minecraft catalog treats Mojang's manifest as required and every distribution as optional: a distribution that fails is dropped with a warning rather than failing the catalog, because losing Purpur builds should not also cost the operator Paper and Fabric. The proxy catalog requires *both* halves, since a document with one proxy would silently remove the other from every consumer's choices. A resolver that succeeds but yields nothing returns `ResolveError::Empty` so a catalog is never replaced with an empty document.

## Configuration

No configuration file. `Config::from_env` (`warehouse/src/config.rs`) builds everything from `WAREHOUSE_*` variables and fails at startup naming the first one it cannot parse. Per-catalog max ages are resolved **once at startup** into a map — a refresh loop must never re-read the environment.

`WAREHOUSE_USER_AGENT` deserves care. Several upstreams are volunteer-run projects, and an anonymous scraper is the first thing they block; the default names the project, and an operator should override it with something that identifies them.

## Consumers

`hosting-system`'s backend is the reference consumer (`backend/src/warehouse.rs`). It polls the manifest, pulls only catalogs whose etag changed, and hands each to the matching template's `accept_catalog`. It never calls warehouse while serving a request — catalogs live in process — so warehouse being down costs nothing until the held catalogs go stale.

**Schema compatibility is a real contract now**, not a deploy-skew convenience. `SCHEMA_VERSION` in `warehouse_common/src/lib.rs` is a promise to outside users. Bump it on any change to a catalog document that is not purely additive; the manifest carries it and consumers refuse a mismatch rather than misreading a body. A change touching both a schema type and its consumer is two PRs across two repositories, gated by that version.

## Conventions

Inherited from hosting-system, deliberately — the clippy config in `Cargo.toml` is a copy of that workspace's (`pedantic` + `nursery` + a large restriction set, mostly `deny`/`forbid`). Notably denied: `unwrap_used`, `panic`, `indexing_slicing`, `print_stdout`/`print_stderr` (use `tracing`), `str_to_string`, `min_ident_chars`, `separated_literal_suffix`, `std_instead_of_core`. `expect_used` is allowed but `unwrap` is not.

### Comments

Same rule as hosting-system: **no comments in the code, period** — no file/module headers, no explanatory `//` or `///` notes, no commented-out code, no TODOs. Naming and structure carry the meaning; rationale belongs in this file.

The one exception is the same as hosting-system's: every `#[route(...)]` handler in `warehouse_common/src/service/` gets a `///` describing what the endpoint does, because those feed the OpenAPI document. Keep them to a description — the reasoning goes here, not there.

This is a departure from normal open-source Rust practice, where a published crate documents its public API for docs.rs. It is a deliberate trade to keep one convention across both repositories. If you ever want rustdoc on `warehouse_common`'s public types, that is a decision to make explicitly and apply consistently, not something to reintroduce one comment at a time.

### Before finishing a change

Run `cargo fmt --all`, then `cargo clippy --workspace --all-targets --all-features`, then `./scripts/check-invariant.sh`. All three are enforced in CI and the clippy config is strict, so a change isn't done until they pass clean.

## Known gaps

- The Docker Hub catalogs read the five newest tag pages per image, so they hold recent releases rather than full history. This was inherited from the backend's original implementation.
- Minecraft releases older than `1.7.10` are not tracked.
- Images carry no build provenance or SBOM: `push-by-digest`, which the multi-arch
  workflow needs, cannot be combined with inline attestations. Attesting the merged
  manifest afterwards would fix this.
