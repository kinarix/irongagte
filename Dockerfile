# ── Stage 1: build ────────────────────────────────────────────────────────────
FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY crates/core/Cargo.toml       crates/core/Cargo.toml
COPY crates/crypto/Cargo.toml     crates/crypto/Cargo.toml
COPY crates/store/Cargo.toml      crates/store/Cargo.toml
COPY crates/auth/Cargo.toml       crates/auth/Cargo.toml
COPY crates/webauthn/Cargo.toml   crates/webauthn/Cargo.toml
COPY crates/federation/Cargo.toml crates/federation/Cargo.toml
COPY crates/authz/Cargo.toml      crates/authz/Cargo.toml
COPY crates/scim/Cargo.toml       crates/scim/Cargo.toml
COPY crates/api/Cargo.toml        crates/api/Cargo.toml

# Stub each crate so cargo can resolve the dependency graph
RUN for dir in core crypto store auth webauthn federation authz scim; do \
      mkdir -p crates/$dir/src && echo "pub fn _stub() {}" > crates/$dir/src/lib.rs; \
    done && \
    mkdir -p crates/api/src && \
    printf 'fn main() {}' > crates/api/src/main.rs

RUN cargo build --release --bin irongate 2>/dev/null || true

# Now copy real source and build for real
COPY crates/ crates/
COPY migrations/ migrations/

RUN touch crates/*/src/*.rs && \
    cargo build --release --bin irongate

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --no-create-home --shell /bin/false irongate

WORKDIR /app

COPY --from=builder /build/target/release/irongate ./
COPY config/ config/
COPY migrations/ migrations/

USER irongate

EXPOSE 8080

ENTRYPOINT ["./irongate"]
CMD ["serve"]
