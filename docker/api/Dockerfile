FROM rust:1.82

WORKDIR /usr/src/myapp
RUN "ls -ali"
COPY ./api .

RUN cargo install --path .

CMD ["myapp"]
