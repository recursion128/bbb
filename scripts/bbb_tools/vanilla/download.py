"""The only Vanilla module allowed to perform network I/O."""

from __future__ import annotations

import os
import time
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

from .errors import DownloadFailure
from .integrity import verify_file

USER_AGENT = "bbb-vanilla-tooling/1"


def fetch_bytes(url: str, *, timeout: float = 60.0, retries: int = 3) -> bytes:
    if not url.startswith("https://"):
        raise DownloadFailure(f"refusing non-HTTPS Vanilla URL: {url}")
    last_error: Exception | None = None
    for attempt in range(1, retries + 1):
        try:
            request = Request(url, headers={"User-Agent": USER_AGENT})
            with urlopen(request, timeout=timeout) as response:
                return response.read()
        except (HTTPError, URLError, TimeoutError, OSError) as error:
            last_error = error
            if attempt != retries:
                time.sleep(min(2 ** (attempt - 1), 4))
    raise DownloadFailure(
        f"download failed after {retries} attempts: {url}: {last_error}"
    ) from last_error


def download_artifact(
    url: str,
    destination: Path,
    *,
    size: int,
    sha1: str,
    sha256: str | None = None,
    timeout: float = 60.0,
    retries: int = 3,
) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    part = destination.with_name(f".{destination.name}.{os.getpid()}.part")
    try:
        payload = fetch_bytes(url, timeout=timeout, retries=retries)
        with part.open("wb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        verify_file(
            part,
            size=size,
            sha1=sha1,
            sha256=sha256,
            label=url,
        )
        os.replace(part, destination)
    finally:
        part.unlink(missing_ok=True)
