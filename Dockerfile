FROM node:22-alpine AS frontend-builder
WORKDIR /build/app/nehex-admin

COPY app/nehex-admin/package.json app/nehex-admin/package-lock.json ./
RUN npm ci

COPY app/nehex-admin ./
RUN npm run build


FROM python:3.12-slim AS runtime
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV ADMIN_MANAGER_BUILD_ON_STARTUP=false

WORKDIR /app

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY app ./app
COPY --from=frontend-builder /build/app/nehex-admin/dist ./app/nehex-admin/dist

EXPOSE 7878

CMD ["sh", "-c", "uvicorn app.main:app --host 0.0.0.0 --port ${APP_PORT:-7878}"]
