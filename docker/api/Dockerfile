FROM rust:1.82

WORKDIR /usr/src/myapp
RUN pwd
COPY . .
RUN "ls -ali"

RUN cargo install --path .

CMD ["myapp"]
