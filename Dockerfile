FROM lukemathwalker/cargo-chef:latest AS chef
RUN apt-get update
RUN apt-get install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl
RUN strip target/x86_64-unknown-linux-musl/release/montreality-bot

FROM scratch
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/montreality-bot /
COPY config.toml /
CMD ["/montreality-bot"]