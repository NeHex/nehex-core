ARG DOCKERHUB_MIRROR=docker.io

FROM ${DOCKERHUB_MIRROR}/library/node:22-alpine AS frontend-builder
WORKDIR /app/app/nehex-admin

RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.aliyun.com/g' /etc/apk/repositories

COPY app/nehex-admin/package.json app/nehex-admin/package-lock.json ./
RUN npm ci --registry=https://registry.npmmirror.com

COPY app/nehex-admin ./
RUN npm run build

FROM ${DOCKERHUB_MIRROR}/library/rust:1-bookworm AS backend-builder
WORKDIR /app

RUN mkdir -p /usr/local/cargo/conf && \
    echo '[source.crates-io]' > /usr/local/cargo/config.toml && \
    echo 'replace-with = "aliyun-sparse"' >> /usr/local/cargo/config.toml && \
    echo '[source.aliyun-sparse]' >> /usr/local/cargo/config.toml && \
    echo 'registry = "sparse+https://mirrors.aliyun.com/crates.io-index/"' >> /usr/local/cargo/config.toml && \
    echo '[net]' >> /usr/local/cargo/config.toml && \
    echo 'retry = 10' >> /usr/local/cargo/config.toml && \
    echo 'git-fetch-with-cli = true' >> /usr/local/cargo/config.toml && \
    echo '[http]' >> /usr/local/cargo/config.toml && \
    echo 'timeout = 120' >> /usr/local/cargo/config.toml && \
    echo 'multiplexing = false' >> /usr/local/cargo/config.toml

COPY backend-rust ./backend-rust
RUN cd backend-rust && cargo build --release

FROM ${DOCKERHUB_MIRROR}/library/debian:bookworm-slim AS runtime
ENV ADMIN_MANAGER_BUILD_ON_STARTUP=false
ENV APP_ENV=prod

RUN sed -i "s@deb.debian.org@mirrors.aliyun.com@g" /etc/apt/sources.list.d/debian.sources && \
    apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/backend-rust/target/release/backend-rust ./backend-rust/backend-rust
COPY --from=frontend-builder /app/app/nehex-admin/dist ./app/nehex-admin/dist

RUN mkdir -p /app/storage /app/backups

EXPOSE 7878

CMD ["/app/backend-rust/backend-rust"]
