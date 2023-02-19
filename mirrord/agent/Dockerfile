FROM --platform=$BUILDPLATFORM instrumentisto/rust:nightly-2023-01-31 as build-env
ARG TARGETARCH
WORKDIR /app
RUN curl -sSL https://bootstrap.pypa.io/get-pip.py -o get-pip.py && python3 get-pip.py
RUN python3 -m pip install ziglang
RUN cargo install cargo-zigbuild cargo-chef --locked
COPY rust-toolchain.toml ./mirrord/agent/platform.sh .
COPY .cargo ./.cargo

RUN ./platform.sh
RUN rustup component add --toolchain nightly rustfmt
RUN rustup target add --toolchain nightly $(cat /.platform)
RUN apt-get update && apt-get install -y $(cat /.compiler)

FROM build-env AS planner

COPY Cargo.toml Cargo.lock CHANGELOG.md README.md LICENSE .
COPY sample/rust ./sample/rust
COPY mirrord ./mirrord
COPY tests ./tests

RUN cargo +nightly chef prepare --recipe-path recipe.json

FROM build-env AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN cargo +nightly chef cook --zigbuild --manifest-path /app/mirrord/agent/Cargo.toml --release --recipe-path recipe.json --target $(cat /.platform)

COPY Cargo.toml Cargo.lock CHANGELOG.md README.md LICENSE .
COPY sample/rust ./sample/rust
COPY mirrord ./mirrord
COPY tests ./tests

RUN cargo +nightly zigbuild -Z bindeps --manifest-path /app/mirrord/agent/Cargo.toml --target $(cat /.platform) --release

RUN cp /app/target/$(cat /.platform)/release/mirrord-agent /mirrord-agent

FROM debian:stable
COPY --from=builder /mirrord-agent /
RUN apt update && apt install -y iptables conntrack
RUN update-alternatives --set iptables /usr/sbin/iptables-legacy \
    && update-alternatives --set ip6tables /usr/sbin/ip6tables-legacy

CMD ["./mirrord-agent"]