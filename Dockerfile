FROM rust:latest

WORKDIR /usr/src/devpoker
COPY . .

RUN cargo build --release
