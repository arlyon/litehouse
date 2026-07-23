FROM debian:bookworm-slim AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates clang curl mold pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain nightly-2026-07-15 \
        --target wasm32-wasip2
ENV PATH="/root/.cargo/bin:${PATH}"
ENV RUSTUP_TOOLCHAIN=nightly-2026-07-15

WORKDIR /app
COPY . .
RUN FLATC_DIR="$(bash scripts/install-flatc.sh)" \
    && PATH="${FLATC_DIR}:${PATH}" cargo build --release \
        --target wasm32-wasip2 \
        -p esphome -p tasmota -p ntfy -p open-meteo \
    && PATH="${FLATC_DIR}:${PATH}" cargo build --release -p litehouse

FROM debian:bookworm-slim AS litehouse

LABEL org.opencontainers.image.source="https://github.com/arlyon/litehouse"
LABEL org.opencontainers.image.description="Litehouse local MCP home automation daemon"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 litehouse \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin litehouse \
    && mkdir -p /config /opt/litehouse/components \
    && chown litehouse:litehouse /config

COPY --from=builder /app/target/release/litehouse /usr/local/bin/litehouse
COPY --from=builder /app/target/wasm32-wasip2/release/esphome.wasm /opt/litehouse/components/esphome.wasm
COPY --from=builder /app/target/wasm32-wasip2/release/tasmota.wasm /opt/litehouse/components/tasmota.wasm
COPY --from=builder /app/target/wasm32-wasip2/release/ntfy.wasm /opt/litehouse/components/ntfy.wasm
COPY --from=builder /app/target/wasm32-wasip2/release/open_meteo.wasm /opt/litehouse/components/open_meteo.wasm

USER 10001:10001
WORKDIR /config
EXPOSE 8080
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD ["curl", "--fail", "--silent", "http://127.0.0.1:8080/healthz"]
ENTRYPOINT ["/usr/local/bin/litehouse"]
CMD ["run", "--wasm-path", "/opt/litehouse/components", "--bind", "0.0.0.0:8080", "--allowed-host", "localhost", "--allowed-origin", "http://localhost:8080"]
