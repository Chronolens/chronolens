FROM ghcr.io/chronolens/libheif:latest

WORKDIR /app
COPY . .

RUN cargo build --release --bin preview

CMD ["./target/release/preview"]
