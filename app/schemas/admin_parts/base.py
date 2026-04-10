from __future__ import annotations

from datetime import datetime
from typing import Any
from typing import Optional

from pydantic import BaseModel, Field, field_validator, model_validator

from app.models.setting import SettingType


class AdminLoginRequest(BaseModel):
    account: str = Field(min_length=1, max_length=100)
    password: str = Field(min_length=1, max_length=300)

    @field_validator("account", "password")
    @classmethod
    def normalize_required_text(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized


class AdminLoginData(BaseModel):
    token: Optional[str] = None
    account: str
    expires_at: datetime


class AdminLoginResponse(BaseModel):
    data: AdminLoginData


class AdminPublicMarkerData(BaseModel):
    marker: str
    account: str
    expires_at: datetime


class AdminPublicMarkerResponse(BaseModel):
    data: AdminPublicMarkerData


class AdminActionResponse(BaseModel):
    success: bool = True
    message: str


class AdminBackupRestoreRequest(BaseModel):
    confirm_overwrite: bool = Field(default=False)


class AdminMailSmtpTestRequest(BaseModel):
    smtp_host: str = Field(min_length=1, max_length=255)
    smtp_port: int = Field(default=465, ge=1, le=65535)
    smtp_security: str = Field(default="ssl", min_length=1, max_length=20)
    smtp_username: Optional[str] = Field(default=None, max_length=255)
    smtp_password: Optional[str] = Field(default=None, max_length=255)
    smtp_from_email: Optional[str] = Field(default=None, max_length=255)
    smtp_from_name: Optional[str] = Field(default=None, max_length=120)
    smtp_timeout_seconds: int = Field(default=12, ge=3, le=120)
    test_email: str = Field(min_length=1, max_length=255)

    @field_validator(
        "smtp_host",
        "smtp_security",
        "test_email",
    )
    @classmethod
    def normalize_required_text(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized

    @field_validator("smtp_username", "smtp_password", "smtp_from_email", "smtp_from_name")
    @classmethod
    def normalize_optional_text(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None

    @field_validator("smtp_security")
    @classmethod
    def normalize_smtp_security(cls, value: str) -> str:
        normalized = value.strip().lower()
        if normalized not in {"none", "starttls", "ssl"}:
            raise ValueError("smtp_security must be one of: none/starttls/ssl")
        return normalized


class AdminSettingUpdateItem(BaseModel):
    setting_key: str = Field(min_length=1, max_length=100)
    setting_content: Any = None
    setting_type: Optional[SettingType] = None
    description: Optional[str] = Field(default=None, max_length=255)

    @field_validator("setting_key")
    @classmethod
    def normalize_setting_key(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("setting_key is required")
        return normalized

    @field_validator("description")
    @classmethod
    def normalize_description(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminSettingsUpdateRequest(BaseModel):
    items: list[AdminSettingUpdateItem] = Field(min_length=1, max_length=200)


class AdminAccountSettingsUpdateRequest(BaseModel):
    account: Optional[str] = Field(default=None, min_length=1, max_length=100)
    new_password: Optional[str] = Field(default=None, min_length=1, max_length=300)
    confirm_password: Optional[str] = Field(default=None, min_length=1, max_length=300)

    @field_validator("account", "new_password", "confirm_password")
    @classmethod
    def normalize_optional_text_fields(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None

    @model_validator(mode="after")
    def validate_password_update(self) -> "AdminAccountSettingsUpdateRequest":
        if self.new_password is None:
            return self

        if self.confirm_password is None:
            raise ValueError("confirm_password is required when new_password is provided")

        if self.new_password != self.confirm_password:
            raise ValueError("new_password and confirm_password do not match")
        return self


class AdminInstallStatusData(BaseModel):
    installed: bool
    schema_ready: bool
    table_count: int = Field(ge=0)
    admin_manager_web: str


class AdminInstallStatusResponse(BaseModel):
    data: AdminInstallStatusData


class AdminInstallArticleClassItem(BaseModel):
    value: str = Field(min_length=1, max_length=100)
    label: Optional[str] = Field(default=None, max_length=120)

    @field_validator("value", "label")
    @classmethod
    def normalize_text(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None

    @model_validator(mode="after")
    def validate_value(self) -> "AdminInstallArticleClassItem":
        if not self.value:
            raise ValueError("value is required")
        if self.label is None:
            self.label = self.value
        return self


class AdminInstallStepAdmin(BaseModel):
    account: str = Field(min_length=1, max_length=100)
    password: str = Field(min_length=1, max_length=300)
    confirm_password: str = Field(min_length=1, max_length=300)
    admin_manager_web: str = Field(default="/nehex-admin", min_length=1, max_length=80)

    @field_validator("account", "password", "confirm_password", "admin_manager_web")
    @classmethod
    def normalize_required_text(cls, value: str) -> str:
        normalized = value.strip()
        if not normalized:
            raise ValueError("Field is required")
        return normalized

    @model_validator(mode="after")
    def validate_password(self) -> "AdminInstallStepAdmin":
        if self.password != self.confirm_password:
            raise ValueError("password and confirm_password do not match")
        return self


class AdminInstallStepNehex(BaseModel):
    site_title: Optional[str] = Field(default="NeHex", max_length=255)
    site_sub_title: Optional[str] = Field(default="", max_length=255)
    site_api_base: Optional[str] = Field(default="", max_length=255)
    article_classes: list[AdminInstallArticleClassItem] = Field(default_factory=list, max_length=50)

    @field_validator("site_title", "site_sub_title", "site_api_base")
    @classmethod
    def normalize_optional_text(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None

    @model_validator(mode="after")
    def ensure_article_classes(self) -> "AdminInstallStepNehex":
        if not self.article_classes:
            self.article_classes = [AdminInstallArticleClassItem(value="default", label="默认分类")]
            return self

        unique: dict[str, AdminInstallArticleClassItem] = {}
        for item in self.article_classes:
            unique[item.value] = item
        self.article_classes = list(unique.values())
        return self


class AdminInstallStepSite(BaseModel):
    site_url: Optional[str] = Field(default="", max_length=500)
    site_description: Optional[str] = Field(default="", max_length=2000)
    site_keywords: Optional[str] = Field(default="", max_length=500)
    site_icp: Optional[str] = Field(default="", max_length=255)
    site_notice: Optional[str] = Field(default="", max_length=4000)

    @field_validator("site_url", "site_description", "site_keywords", "site_icp", "site_notice")
    @classmethod
    def normalize_optional_text(cls, value: Optional[str]) -> Optional[str]:
        if value is None:
            return None
        normalized = value.strip()
        return normalized or None


class AdminInstallRequest(BaseModel):
    admin: AdminInstallStepAdmin
    nehex: AdminInstallStepNehex
    site: AdminInstallStepSite


class AdminInstallResponse(BaseModel):
    data: AdminInstallStatusData
    message: str = "Installation completed"
