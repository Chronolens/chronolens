FROM ubuntu:latest

RUN apt-get update && \
    apt-get install -y curl git pkg-config libssl-dev libheif-dev libheif1 build-essential && \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

ARG TAG
RUN git clone --single-branch --branch ${TAG} https://github.com/chronolens/chronolens /app

WORKDIR /app

RUN cargo build --release
