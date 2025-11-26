FROM rust:alpine AS cargo_chef

RUN apk update
RUN apk add --no-cache pkgconfig libressl-dev musl-dev

ENV RUSTFLAGS='-C target-feature=-crt-static'

RUN cargo install cargo-chef --locked

FROM cargo_chef AS rust_planner

WORKDIR /usr/src/app

COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src
COPY openapi.yaml ./

RUN cargo chef prepare --recipe-path recipe.json

FROM cargo_chef AS rust_builder

WORKDIR /usr/src/app

COPY --from=rust_planner /usr/src/app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src
COPY openapi.yaml ./

RUN cargo build --release --bin geoip

FROM alpine AS runtime

WORKDIR /opt/app

RUN apk update
RUN apk add --no-cache libgcc libressl

ENV USER=appuser
ENV UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

COPY --from=rust_builder /usr/src/app/target/release/geoip ./

RUN mkdir -p /data && chown -R ${UID}:${UID} /data
VOLUME ["/data"]

USER appuser:appuser

ENV DATA_DIR=/data
ENV LISTEN_ADDR=0.0.0.0:8080

EXPOSE 8080

CMD ["/opt/app/geoip"]
