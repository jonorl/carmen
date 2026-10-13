# Built from the repository root (the server shares a crate with the game):
#   docker build -f server/Dockerfile .
FROM rust:1-slim-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p carmen-server

FROM debian:bookworm-slim
COPY --from=build /src/target/release/carmen-server /usr/local/bin/carmen-server
EXPOSE 8085
CMD ["carmen-server"]
