# NeHex Core 

## 技术栈

- `Rust + Axum`
- `SQLx`
- `PostgreSQL`
- `Redis`（分布式缓存）
- `Vue 3 + Vuetify`（后台管理端）

## 中国大陆网络加速

项目已内置以下国内镜像优化：

- `Cargo`：`nehex-core/.cargo/config.toml` 使用阿里云 `mirrors.aliyun.com`（sparse index）
- `npm`：`app/nehex-admin/.npmrc` 使用 `registry.npmmirror.com`（阿里系镜像）
- `Docker`：`Dockerfile` 与 `docker-compose.yml` 默认使用官方仓库 `docker.io`

如需使用镜像加速，可覆盖环境变量（`DOCKERHUB_MIRROR` 仅填写 registry 域名，不要带 `http(s)://`）：

```bash
DOCKERHUB_MIRROR=docker.mirrors.ustc.edu.cn
POSTGRES_IMAGE=postgres:16-alpine
REDIS_IMAGE=redis:7.2-alpine
```
