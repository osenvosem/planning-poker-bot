FROM rust:latest

WORKDIR /usr/src/devpoker
COPY . .

RUN cargo install --path .

CMD ["devpoker"]