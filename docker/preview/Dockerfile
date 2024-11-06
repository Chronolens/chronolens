# FROM rust:1.82

# RUN apt-get update && \
#     apt-get install -y curl git pkg-config libssl-dev libheif-dev libheif1 zlib1g-dev libclang-dev build-essential && \
#     rm -rf /var/lib/apt/lists/*

# WORKDIR /usr/src/myapp
# COPY . .

# WORKDIR /usr/src/myapp/preview

# RUN cargo install --path . --target-dir .

# CMD ["./release/preview"]



FROM ghcr.io/chronolens/chronolens:latest

WORKDIR /app

COPY . .
WORKDIR /app/preview

RUN cargo build --release

CMD ["./release/preview"]
