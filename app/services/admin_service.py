from app.services import admin_service_parts as _parts

__all__ = list(_parts.__all__)

for _name in __all__:
    globals()[_name] = getattr(_parts, _name)

del _name
del _parts
