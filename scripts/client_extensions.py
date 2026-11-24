"""Parse the bounded ledger of client extensions outside Vanilla parity."""

import re
from dataclasses import dataclass
from pathlib import Path

_ID = re.compile(r"client-extension/[a-z0-9]+(?:-[a-z0-9]+)*")
_FIELDS = {
    "Status": "status",
    "Owner": "owner",
    "Goal": "goal",
    "Non-goals": "non_goals",
    "Dependencies": "dependencies",
    "Acceptance": "acceptance",
    "Enablement": "enablement",
    "Server capability": "server_capability",
    "Reopen when": "reopen_when",
}
_REQUIRED = set(_FIELDS) - {"Reopen when"}


@dataclass(frozen=True)
class Extension:
    key: str
    status: str
    owner: str
    goal: str
    non_goals: str
    dependencies: tuple[str, ...]
    acceptance: str
    enablement: str
    server_capability: str
    reopen_when: str


def load_extensions(repo: Path) -> dict[str, Extension]:
    path = repo / "docs/client-extensions.md"
    if not path.exists():
        return {}
    entries: dict[str, Extension] = {}
    key = ""
    fields: dict[str, str] = {}
    heading_line = 0

    def error(message: str, line: int) -> ValueError:
        return ValueError(f"{path}:{line}: {key or 'client extensions'}: {message}")

    def finish() -> None:
        if not key:
            return
        missing = _REQUIRED - fields.keys()
        if missing:
            raise error(f"missing fields: {', '.join(sorted(missing))}", heading_line)
        if fields["Status"] not in {"active", "deferred"}:
            raise error(f"invalid Status: {fields['Status']!r}", heading_line)
        if fields["Status"] == "deferred" and "Reopen when" not in fields:
            raise error("deferred entry requires Reopen when", heading_line)
        raw = fields["Dependencies"]
        deps = () if raw == "-" else tuple(part.strip() for part in raw.split(","))
        for dep in deps:
            if not _ID.fullmatch(dep):
                raise error(f"invalid dependency ID: {dep!r}", heading_line)
            if dep == key:
                raise error(f"self dependency: {dep}", heading_line)
        if len(set(deps)) != len(deps):
            raise error("duplicate dependency ID", heading_line)
        values = {_FIELDS[name]: value for name, value in fields.items()}
        values["dependencies"] = deps
        values.setdefault("reopen_when", "")
        entries[key] = Extension(key=key, **values)

    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        heading = re.match(r"^\s*(#+)\s*(.+?)\s*$", line)
        if heading:
            finish()
            key, fields = "", {}
            title = heading[2]
            if not title.startswith("client-extension"):
                continue
            if heading[1] != "##" or not _ID.fullmatch(title) or line != f"## {title}":
                raise error(f"malformed extension heading: {line!r}", number)
            if title in entries:
                raise error(f"duplicate extension ID: {title}", number)
            key, heading_line = title, number
            continue
        if not key:
            continue
        if not line.strip() and number != heading_line + 1:
            continue
        field = re.fullmatch(r"- ([^:]+):\s*(.*?)\s*", line)
        if not field:
            raise error("expected '- Field: value'", number)
        name, value = field.groups()
        if name not in _FIELDS:
            raise error(f"unknown field: {name}", number)
        if name in fields:
            raise error(f"duplicate field: {name}", number)
        if not value:
            raise error(f"empty field: {name}", number)
        fields[name] = value
    finish()

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(current: str, trail: tuple[str, ...]) -> None:
        if current in visiting:
            raise ValueError(f"{path}: dependency cycle: {' -> '.join((*trail, current))}")
        if current in visited:
            return
        visiting.add(current)
        for dependency in entries[current].dependencies:
            if dependency not in entries:
                raise ValueError(f"{path}: {current}: missing dependency: {dependency}")
            visit(dependency, (*trail, current))
        visiting.remove(current)
        visited.add(current)

    for current in entries:
        visit(current, ())
    return entries


def validate_owner(repo: Path, stream: str, capability: str, coverage_key: str) -> None:
    entries = load_extensions(repo)
    if capability not in entries:
        raise ValueError(f"unknown client extension capability: {capability}")
    entry = entries[capability]
    if entry.status != "active":
        raise ValueError(f"{capability}: Status must be active, found {entry.status}")
    if entry.owner != stream:
        raise ValueError(f"{capability}: Owner is {entry.owner}, not {stream}")
    expected = f"extension/{capability.split('/', 1)[1]}"
    if coverage_key != expected:
        raise ValueError(f"{capability}: coverage key must be {expected}, found {coverage_key!r}")
    if entry.dependencies:
        raise ValueError(f"{capability}: remaining dependencies block admission: {', '.join(entry.dependencies)}")
