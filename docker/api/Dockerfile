FROM rust:1.82

WORKDIR /usr/src/myapp
COPY ./api .

RUN cargo install --path .

CMD ["myapp"]
