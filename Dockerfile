FROM clux/muslrust:nightly as chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef as planner
COPY . .
RUN ls
RUN cargo chef prepare

FROM chef as cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook -p litehouse --release

FROM chef as builder
COPY . .
COPY --from=cacher /app/target target
RUN cargo build --release -p litehouse

FROM scratch

LABEL io.containers.autoupdate=registry
LABEL org.opencontainers.image.source https://github.com/arlyon/litehouse
LABEL org.opencontainers.image.description A lightweight home automation server

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/litehouse /litehouse

CMD ["/litehouse", "run"]
