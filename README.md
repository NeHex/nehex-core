# NeHex Core (FastAPI)

NeHex 后端服务，使用 `Python + FastAPI + MySQL`，并内置管理后台前端静态托管。

## 技术栈

- `FastAPI`
- `SQLAlchemy 2.x`
- `PyMySQL`
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

3. 配置环境变量

复制 `.env.example` 为 `.env` 后修改数据库参数等配置。

4. 启动服务

```powershell
uvicorn app.main:app --host 0.0.0.0 --port 7878 --reload
```

默认会在启动时自动构建管理端前端（`npm install && npm run build`）。
可通过 `ADMIN_MANAGER_BUILD_ON_STARTUP=false` 关闭自动构建（适合 Docker 使用预构建产物）。

## Docker 部署（推荐）

### 1. 准备 Docker 环境变量

```powershell
Copy-Item .env.example .env
```

然后编辑 `.env`，至少修改：

- `DB_PASSWORD`
- `MYSQL_ROOT_PASSWORD`
- `ADMIN_API_SECRET`

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
