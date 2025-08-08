FROM lukemathwalker/cargo-chef:latest-rust-1.88.0-slim as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json

# Build project dependencies (not the application)
RUN cargo chef cook --release --recipe-path recipe.json

# All dependencies should be cached until now
COPY . .

# offline compile time verification
ENV SQLX_OFFLINE=true

# build the project binary
RUN cargo build --release --bin ztp

# ============ R U N T I M E ==============

# runtime stage (the final image)
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# copy the compiled binary
COPY --from=builder /app/target/release/ztp ztp

# config is needed for runtime
COPY configuration configuration

# set the environment
ENV APP_ENVIRONMENT=production

# launch the binary
ENTRYPOINT ["./ztp"]
