FROM rust:1.82

WORKDIR /usr/src/myapp
COPY . .
WORKDIR api

RUN cargo install --path .

CMD ["myapp"]
