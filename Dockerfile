FROM rust:slim-bullseye as build

RUN apt-get update && apt-get install -y \
    build-essential autoconf automake libtool libssl-dev pkg-config

WORKDIR "/discord-bot-rust"

# Cache cargo build dependencies by creating a dummy source
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
COPY Cargo.toml ./
RUN cargo build --release

COPY . .
RUN cargo build --release

# Release image
# Necessary dependencies to run Parrot
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y python3-pip ffmpeg
RUN pip install -U yt-dlp

COPY --from=build /discord-bot-rust/target/release/discord-bot-rust .

CMD ["./discord-bot-rust"]
