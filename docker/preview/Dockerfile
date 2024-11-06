# FROM rust:1.82

# RUN apt-get update && \
#     apt-get install -y curl git pkg-config libssl-dev libheif-dev libheif1 zlib1g-dev libclang-dev build-essential && \
#     rm -rf /var/lib/apt/lists/*

# WORKDIR /usr/src/myapp
# COPY . .

# WORKDIR /usr/src/myapp/preview

# RUN cargo install --path . --target-dir .

# CMD ["./release/preview"]



FROM ubuntu:latest

RUN apt-get update && \
    apt-get install -y curl git pkg-config libssl-dev libheif-dev libheif1 zlib1g-dev libclang-dev build-essential && \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:$PATH"

WORKDIR /app

COPY . .
WORKDIR /app/preview

RUN cargo build --release

CMD ["./release/preview"]
