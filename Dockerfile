FROM rust:latest AS build
WORKDIR /app
COPY . .
RUN cargo build --release --bin sutra

FROM debian:stable-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/sutra /usr/local/bin/sutra
EXPOSE 8080
CMD ["sutra", "server", "--port", "8080"]
