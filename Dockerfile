FROM node:22-alpine AS frontend-builder
WORKDIR /app/app/nehex-admin

COPY app/nehex-admin/package.json app/nehex-admin/package-lock.json ./
RUN npm ci

COPY app/nehex-admin ./
RUN npm run build


FROM rust:1-bookworm AS backend-builder
WORKDIR /app

COPY backend-rust ./backend-rust
RUN cd backend-rust && cargo build --release


FROM debian:bookworm-slim AS runtime
ENV ADMIN_MANAGER_BUILD_ON_STARTUP=false
ENV APP_ENV=prod

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/backend-rust/target/release/backend-rust ./backend-rust/backend-rust
COPY --from=frontend-builder /app/app/nehex-admin/dist ./app/nehex-admin/dist

RUN mkdir -p /app/storage /app/backups

EXPOSE 7878

CMD ["/app/backend-rust/backend-rust"]
