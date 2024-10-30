FROM rust:1.82

WORKDIR /usr/src/myapp
COPY . .

WORKDIR /usr/src/myapp/api

RUN ls
RUN cargo install --path .

RUN ls

CMD ["myapp"]
