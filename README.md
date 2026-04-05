# NeHex Core (FastAPI)

NeHex 后端服务，使用 `Python + FastAPI + MySQL`，并内置管理后台前端静态托管。

## 技术栈

- `FastAPI`
- `SQLAlchemy 2.x`
- `PyMySQL`
- `Redis`（分布式缓存）
- `Pydantic v2`
- `Vue 3 + Vuetify`（后台管理端）

## 本地运行

1. 创建并激活虚拟环境

```powershell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
```

2. 安装依赖

```powershell
pip install -r requirements.txt
```

依赖管理采用 `pip-tools`：

- `requirements.in`：人工维护的直接依赖范围
- `requirements.txt`：锁定后的可复现安装清单（由 `pip-compile` 生成）

更新依赖时：

```powershell
pip-compile requirements.in --output-file requirements.txt
```

3. 配置环境变量

复制 `.env.example` 为 `.env` 后修改数据库参数等配置。

4. 启动服务

```powershell
uvicorn app.main:app --host 0.0.0.0 --port 7878 --reload
```

默认会在启动时自动构建管理端前端（`npm install && npm run build`）。
可通过 `ADMIN_MANAGER_BUILD_ON_STARTUP=false` 关闭自动构建（适合 Docker 使用预构建产物）。

缓存默认优先使用 Redis（`REDIS_ENABLED=true`），连接由 `REDIS_URL` 控制。
当 Redis 不可用时会自动回退到进程内 TTL 缓存，服务可继续运行。

## Docker 部署（推荐）

### 1. 准备 Docker 环境变量

```powershell
Copy-Item .env.example .env
```

然后编辑 `.env`，至少修改：

- `DB_PASSWORD`
- `MYSQL_ROOT_PASSWORD`
- `ADMIN_API_SECRET`
- `CORS_ALLOW_ORIGINS`（生产环境请改成你的前端域名白名单）
- `DB_AUTO_CREATE_TABLES`（生产建议 `false`，通过迁移初始化数据库）

说明：Docker 镜像会在构建阶段预编译管理端前端，运行时建议保持
`ADMIN_MANAGER_BUILD_ON_STARTUP=false`（默认即为 `false`）。

### 2. 构建并启动

```powershell
docker compose up -d --build
```

### 3. 访问后台安装向导

- 默认后台路径：`http://127.0.0.1:7878/nehex-admin/install`

首次安装包含 3 步：

1. 设置管理员账号密码 + 后台路径（默认 `/nehex-admin`）
2. 设置 NeHex 配置
3. 设置站点配置

安装完成后会自动跳转登录页。
同时会自动创建默认示例内容（示例文章、`about` 示例页面、示例主题配置）。

### 网络说明

- `mysql` 和 `redis` 在 `docker-compose.yml` 中均未暴露宿主机端口，仅供 Docker 内部网络访问。
- 后端通过内部服务名连接：`mysql`、`redis`。

若你在 `docker compose up -d` 后看到数据库连接拒绝，可调大：

- `DB_STARTUP_MAX_RETRIES`
- `DB_STARTUP_RETRY_INTERVAL_SECONDS`

## 安装状态接口

- `GET /admin-api/install/status`：获取是否已安装
- `POST /admin-api/install`：执行首次安装

## 常用接口

### `GET /setting`

获取站点设置（未安装时返回兼容默认配置，不会 500）。

## 更新日志

### 2026-04-05

- 安全：将后端 CORS 从默认全开放改为可配置白名单（`CORS_ALLOW_ORIGINS`）。
- 安全：管理员登录 Cookie 改为按请求协议动态设置 `Secure`（支持反向代理 `X-Forwarded-Proto`）。
- 安全：新增管理员 Cookie 鉴权的 CSRF 同源校验（`Origin/Referer`）。
- 数据：为 `friends.url` 增加唯一约束，并在启动索引检查中补充唯一索引创建。
- 前端：路由鉴权由本地标记改为基于 `/admin-api/auth/me` 的服务端会话校验，避免伪造本地登录状态。
- 运维：运行时自动建表/建索引改为显式开关（`DB_AUTO_CREATE_TABLES`，默认关闭）。
- 性能：`GET /article` 与 `GET /admin-api/comments` 增加分页参数（`page`、`size`）与分页元信息响应。
- 安全：公共 `GET /setting` 改为白名单字段输出，避免黑名单遗漏导致敏感配置外泄。
- 稳定性：内存 TTL 缓存增加容量上限（`SIMPLE_CACHE_MAX_ENTRIES`，默认 `1024`），并在写入时清理过期/淘汰旧键。
- 前端：管理端文章/评论列表接入分页请求与分页控件，降低大数据量场景加载压力。
- 前端：管理端日常/独立页编辑详情改为直连详情接口（不再“先拉全量再筛选”）。
- 工程：移除部分 Python 文件 UTF-8 BOM，减少跨工具链编码兼容问题。
- 工程：将超大模块拆分为分层子模块（`admin_service`、`admin_manager`、`schemas/admin`、`settings.vue`），降低单文件复杂度。
- 工程：引入 `requirements.in + pip-compile` 锁定产物 `requirements.txt`，提升构建可重复性。
- 性能：缓存层升级为 Redis 优先（自动回退内存缓存），支持多实例共享缓存并降低数据库重复查询压力。
