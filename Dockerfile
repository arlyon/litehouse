FROM clux/muslrust:nightly as chef
RUN cargo install cargo-chef
RUN curl https://github.com/google/flatbuffers/releases/download/v24.3.25/Linux.flatc.binary.clang++-15.zip -Lo flatc.zip && unzip flatc.zip && chmod +x flatc && mv flatc /usr/local/bin
WORKDIR /app

FROM chef as planner
COPY . .
RUN cargo chef prepare

FROM chef as cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook -p litehouse -p cockpit --release

FROM chef as builder-litehouse
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release -p litehouse

FROM chef as builder-cockpit
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release -p cockpit

FROM scratch as litehouse

LABEL io.containers.autoupdate=registry
LABEL org.opencontainers.image.source https://github.com/arlyon/litehouse
LABEL org.opencontainers.image.description A lightweight home automation server

COPY --from=builder-litehouse /app/target/x86_64-unknown-linux-musl/release/litehouse /litehouse

CMD ["/litehouse", "run"]


FROM scratch as cockpit

LABEL io.containers.autoupdate=registry
LABEL org.opencontainers.image.source https://github.com/arlyon/litehouse
LABEL org.opencontainers.image.description A webrtc signalling server for litehouse, the home automation server

COPY --from=builder-cockpit /app/target/x86_64-unknown-linux-musl/release/cockpit /cockpit

CMD ["/cockpit"]
