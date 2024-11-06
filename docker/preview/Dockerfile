FROM ghcr.io/chronolens/chronolens/libheif:latest

WORKDIR /app

COPY . .
WORKDIR /app/preview

RUN cargo install --path . --target-dir .

CMD ["./release/preview"]
