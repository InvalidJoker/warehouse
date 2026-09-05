FROM rust:1-slim-trixie AS builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends git pkg-config \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Warm the dependency cache on the manifests alone, so a source-only change does not
# refetch and rebuild every crate.
COPY Cargo.toml Cargo.lock ./
COPY warehouse/Cargo.toml warehouse/
COPY warehouse_common/Cargo.toml warehouse_common/
COPY warehouse_resolver/Cargo.toml warehouse_resolver/
RUN mkdir -p warehouse/src warehouse_common/src warehouse_resolver/src \
 && echo 'fn main() {}' > warehouse/src/main.rs \
 && touch warehouse_common/src/lib.rs warehouse_resolver/src/lib.rs \
 && cargo build --release -p warehouse \
 && rm -rf warehouse/src warehouse_common/src warehouse_resolver/src

COPY . .
# Cargo skips a rebuild when mtimes look unchanged; the copy above can preserve them.
RUN touch warehouse/src/main.rs warehouse_common/src/lib.rs warehouse_resolver/src/lib.rs \
 && cargo build --release -p warehouse \
 && strip target/release/warehouse

FROM debian:trixie-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home-dir /data --no-create-home warehouse \
 && mkdir -p /data \
 && chown warehouse:warehouse /data

COPY --from=builder /build/target/release/warehouse /usr/local/bin/warehouse

USER warehouse
WORKDIR /data
VOLUME ["/data"]
EXPOSE 8080

ENV WAREHOUSE_BIND=0.0.0.0:8080 \
    WAREHOUSE_DATA_DIR=/data

# Catalogs survive a restart in /data, so a crash loop cannot turn into a burst of
# upstream traffic. Mount it.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/warehouse", "healthcheck"]

ENTRYPOINT ["/usr/local/bin/warehouse"]
