FROM node:22-alpine AS frontend-builder
WORKDIR /app/app/nehex-admin

RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.ustc.edu.cn/g' /etc/apk/repositories

COPY app/nehex-admin/package.json app/nehex-admin/package-lock.json ./
RUN npm ci --registry=https://registry.npmmirror.com

COPY app/nehex-admin ./
RUN npm run build

FROM rust:1-bookworm AS backend-builder
WORKDIR /app

RUN mkdir -p /usr/local/cargo/conf && \
    echo '[source.crates-io]' > /usr/local/cargo/config.toml && \
    echo 'replace-with = "ustc"' >> /usr/local/cargo/config.toml && \
    echo '[source.ustc]' >> /usr/local/cargo/config.toml && \
    echo 'registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"' >> /usr/local/cargo/config.toml

COPY backend-rust ./backend-rust
RUN cd backend-rust && cargo build --release

FROM debian:bookworm-slim AS runtime
ENV ADMIN_MANAGER_BUILD_ON_STARTUP=false
ENV APP_ENV=prod

RUN sed -i "s@deb.debian.org@mirrors.ustc.edu.cn@g" /etc/apt/sources.list.d/debian.sources && \
    apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/backend-rust/target/release/backend-rust ./backend-rust/backend-rust
COPY --from=frontend-builder /app/app/nehex-admin/dist ./app/nehex-admin/dist

RUN mkdir -p /app/storage /app/backups

EXPOSE 7878

CMD ["/app/backend-rust/backend-rust"]
