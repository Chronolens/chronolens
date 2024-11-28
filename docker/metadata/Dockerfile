FROM rust:1.82

WORKDIR /app
COPY . .

RUN cargo build --release --bin metadata

CMD ["./target/release/metadata"]
