FROM rust:alpine as builder

WORKDIR /usr/src/agent
COPY . .

RUN cargo add openssl --features vendored
RUN apk add --no-cache make musl-dev perl
RUN cargo build --release

FROM alpine:latest

RUN apk add --no-cache openssl ca-certificates bash

WORKDIR /app

COPY --from=builder /usr/src/agent/target/release/agent /app/agent
COPY --from=builder /usr/src/agent/config.yaml /app/config.yaml.template

ENTRYPOINT ["/app/agent", "--config", "/app/config.yaml"] 
