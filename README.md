# NeHex Core (FastAPI)

NeHex 的后端服务，使用 `Python + FastAPI + MySQL`。

## 技术栈

- `FastAPI`
- `SQLAlchemy 2.x`
- `PyMySQL`（MySQL 驱动，连接池复用）
- `Pydantic v2`

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

复制 `.env.example` 到 `.env`，按需修改。

4. 启动服务（端口 `7878`）

```powershell
uvicorn app.main:app --host 0.0.0.0 --port 7878 --reload
```

启动后请用以下地址访问（不要用 `0.0.0.0` 作为浏览器地址）：

- 本机访问：`http://127.0.0.1:7878/setting`
- 局域网访问：`http://你的局域网IP:7878/setting`

## 已实现 API

### `GET /setting`

返回 `settings` 表所有配置。

响应示例：

```json
{
  "data": [
    {
      "setting_key": "site_title",
      "setting_type": "string",
      "setting_content": "NeHex",
      "description": "站点标题",
      "created_at": "2026-04-02T14:24:22",
      "updated_at": "2026-04-02T14:24:22"
    }
  ]
}
```
