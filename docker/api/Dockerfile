FROM rust:1.82

WORKDIR /app
COPY . .

RUN cargo build --release --bin api

CMD ["./target/release/api"]
