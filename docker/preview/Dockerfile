FROM rust:1.82

WORKDIR /usr/src/myapp
COPY . .

WORKDIR /usr/src/myapp/preview

RUN cargo install --path . --target-dir .

CMD ["./release/preview"]
