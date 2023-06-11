FROM rust:1-slim as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo install --path server

FROM debian:buster-slim
RUN apt-get update & apt-get install -y extra-runtime-dependencies & rm -rf /var/lib/apt/lists/*
WORKDIR /usr/apps/server
COPY --from=builder /usr/src/app/server/migrations migrations/
COPY --from=builder /usr/src/app/server/assets assets/
COPY --from=builder /usr/local/cargo/bin/server server
CMD ["./server"]
