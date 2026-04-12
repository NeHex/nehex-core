# NeHex Core Rust Backend (WIP)

This is the in-progress Rust replacement backend for `nehex-core`.

## Current status

Implemented runtime foundation:
- startup config/env loading
- PostgreSQL connectivity and startup readiness check
- optional startup schema bootstrap for core/system tables + compatibility indexes (`DB_AUTO_CREATE_TABLES=true`)
- TTL cache with optional Redis backend + in-memory fallback (`REDIS_ENABLED=true`)
  - honors `REDIS_SOCKET_CONNECT_TIMEOUT` / `REDIS_SOCKET_TIMEOUT`
- install-time schema bootstrap for core tables (`POST /admin-api/install`)
- comment mail notification flow (new comment + reply) with `mail_log` persistence
- CORS, logging, health/version
- admin frontend static fallback (`/nehex-admin/...` style)
- online presence websocket (`/ws/online`)

Migrated public endpoints:
- `GET /article`
- `GET /article/{article_id}`
- `POST /article/{article_id}/read`
- `POST /article/{article_id}/like`
- `GET /comment`
- `POST /comment`
- `POST /comment/{comment_id}/like`
- `GET /project`
- `GET /project/{project_id}`
- `GET /daily`
- `GET /album`
- `GET /album/{album_id}`
- `GET /page`
- `GET /page/{page_key}`
- `GET /friend`
- `POST /friend/apply`
- `POST /friend-apply`
- `GET /setting`
- `GET /setting/theme`
- `GET /setting/site-owner`
- `GET /storage/{file_path}`

Notes:
- object storage upload supports `local`, `r2`, `s3`, `aliyun_oss`, `hi168_s3`
- legacy `object_storage_oss_*` settings are mapped to `s3` config keys
- `object_storage_public_base_url` is honored for URL generation

Migrated admin endpoints:
- `GET /admin-api/install/status`
- `POST /admin-api/install`
- `POST /admin-api/auth/login`
- `GET /admin-api/auth/me`
- `GET /admin-api/auth/public-marker`
- `POST /admin-api/auth/logout`
- `GET /admin-api/dashboard`
- `GET /admin-api/articles`
- `GET /admin-api/articles/{article_id}`
- `POST /admin-api/articles`
- `PUT /admin-api/articles/{article_id}`
- `DELETE /admin-api/articles/{article_id}`
- `POST /admin-api/dailies`
- `GET /admin-api/dailies/{daily_id}`
- `PUT /admin-api/dailies/{daily_id}`
- `DELETE /admin-api/dailies/{daily_id}`
- `POST /admin-api/albums`
- `PUT /admin-api/albums/{album_id}`
- `DELETE /admin-api/albums/{album_id}`
- `GET /admin-api/pages`
- `GET /admin-api/pages/{page_id}`
- `POST /admin-api/pages`
- `PUT /admin-api/pages/{page_id}`
- `DELETE /admin-api/pages/{page_id}`
- `GET /admin-api/projects`
- `POST /admin-api/projects`
- `PUT /admin-api/projects/{project_id}`
- `DELETE /admin-api/projects/{project_id}`
- `GET /admin-api/comments`
- `POST /admin-api/comments`
- `PUT /admin-api/comments/{comment_id}`
- `DELETE /admin-api/comments/{comment_id}`
- `GET /admin-api/friends`
- `POST /admin-api/friends`
- `PUT /admin-api/friends/{friend_id}`
- `DELETE /admin-api/friends/{friend_id}`
- `GET /admin-api/friend-applies`
- `PUT /admin-api/friend-applies/{apply_id}/status`
- `GET /admin-api/settings`
- `PUT /admin-api/settings`
- `PUT /admin-api/settings/account`
- `GET /admin-api/media/library`
- `POST /admin-api/media/folders`
- `PUT /admin-api/media/folders/{folder_id}`
- `DELETE /admin-api/media/folders/{folder_id}`
- `GET /admin-api/media/folders/{folder_id}/images`
- `POST /admin-api/media/images/upload`
- `POST /admin-api/media/images/move`
- `DELETE /admin-api/media/images/{image_id}`
- `POST /admin-api/storage/upload`
- `GET /admin-api/backups`
- `POST /admin-api/backups`
- `POST /admin-api/backups/upload-restore`
- `GET /admin-api/backups/{filename}/download`
- `DELETE /admin-api/backups/{filename}`
- `POST /admin-api/backups/{filename}/restore`
- `POST /admin-api/settings/mail/test`
- `GET /admin-api/mail-logs`

## Run

```bash
source "$HOME/.cargo/env"
cd backend-rust
cargo run
```

The server uses `.env` in repository root and expects PostgreSQL to be reachable.

## Mirror (CN)

For users in Mainland China:

- workspace-level Cargo mirror is preconfigured at `../.cargo/config.toml` (`rsproxy.cn`)
- if `rustup` install/update is still slow, set:

```bash
export RUSTUP_DIST_SERVER="https://rsproxy.cn"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
```
