# FROM rust:1.82
#
FROM ubuntu:latest

WORKDIR /usr/src/myapp
COPY . .
RUN "ls -ali"

RUN cargo install --path .

CMD ["myapp"]
