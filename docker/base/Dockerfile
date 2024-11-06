FROM ubuntu:latest

RUN apt-get update && \
    apt-get install -y curl git pkg-config libssl-dev libheif-dev libheif1 zlib1g-dev libclang-dev build-essential && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
