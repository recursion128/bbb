"""Platform input backends used by live development and acceptance gates."""

from .contract import InputBackend, InputError

__all__ = ["InputBackend", "InputError"]
