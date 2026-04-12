# NeHex Core 

## 技术栈

- `Rust + Axum`
- `SQLx`
- `PostgreSQL`
- `Redis`（分布式缓存）
- `Vue 3 + Vuetify`（后台管理端）

## 中国大陆网络加速

项目已内置以下国内镜像优化：

- `Cargo`：`nehex-core/.cargo/config.toml` 使用 `rsproxy.cn`（sparse index）
- `npm`：`app/nehex-admin/.npmrc` 使用 `registry.npmmirror.com`
- `Docker`：`Dockerfile` 与 `docker-compose.yml` 默认使用 `docker.m.daocloud.io`

如需切换回官方源，可覆盖环境变量：

```bash
DOCKERHUB_MIRROR=docker.io
POSTGRES_IMAGE=postgres:16-alpine
REDIS_IMAGE=redis:7.2-alpine
```
