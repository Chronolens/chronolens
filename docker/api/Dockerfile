# FROM rust:1.82
#
FROM ubuntu:latest

WORKDIR /usr/src/myapp
COPY . .
RUN pwd
RUN ls

RUN cargo install --path .

CMD ["myapp"]
