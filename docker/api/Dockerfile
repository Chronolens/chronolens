FROM rust:1.82

WORKDIR /usr/src/myapp
COPY . .

RUN cargo build --release --bin api

CMD ["./target/release/api"]
