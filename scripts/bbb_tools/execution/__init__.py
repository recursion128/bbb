"""Portable subprocess lifecycle primitives."""

from .process import ManagedProcess, ProcessFailure

__all__ = ["ManagedProcess", "ProcessFailure"]
