FROM rust:1.82

WORKDIR /usr/src/myapp
COPY . .

WORKDIR /usr/src/myapp/api

RUN cargo install --path . --target-dir .

CMD ["./release/api"]
