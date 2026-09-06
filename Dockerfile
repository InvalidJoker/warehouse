FROM lukemathwalker/cargo-chef:latest-rust-1-alpine AS chef
WORKDIR /build

# No --target: on an Alpine toolchain the host triple is already *-unknown-linux-musl and
# musl links statically by default, so the same file builds an amd64 or an arm64 image
# depending only on the runner it lands on.
ENV RUSTFLAGS="-C debuginfo=0 -C strip=symbols"

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

# build-base, cmake and perl are for aws-lc-sys, which rustls builds from C sources.
RUN apk add --no-cache build-base cmake perl git ca-certificates

# The recipe is the dependency graph alone, so this layer is reused until a manifest or
# the lockfile changes - a source-only edit never refetches or rebuilds a crate.
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release -p warehouse --recipe-path recipe.json

COPY . .
RUN cargo build --release -p warehouse

RUN mkdir -p /catalogs

FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder --chown=10001:10001 /catalogs /data
COPY --from=builder /build/target/release/warehouse /usr/local/bin/warehouse

USER 10001:10001
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
