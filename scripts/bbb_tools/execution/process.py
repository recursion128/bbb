"""Leak-free macOS/Linux process-group supervision."""

from __future__ import annotations

import os
import signal
import subprocess
import time
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import TracebackType
from typing import IO, Any

from .deadline import Deadline


class ProcessFailure(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class ProcessReceipt:
    argv: tuple[str, ...]
    pid: int
    returncode: int | None
    stdout: str | None
    stderr: str | None


class ManagedProcess:
    """Own one POSIX process group and reap it on every exit path."""

    def __init__(
        self,
        argv: Sequence[str | os.PathLike[str]],
        *,
        cwd: Path | None = None,
        env: Mapping[str, str] | None = None,
        stdin: int | IO[bytes] | None = subprocess.DEVNULL,
        stdout: int | IO[bytes] | None = subprocess.PIPE,
        stderr: int | IO[bytes] | None = subprocess.PIPE,
        **popen_options: Any,
    ) -> None:
        if not argv:
            raise ValueError("managed process argv must not be empty")
        self.argv = tuple(os.fspath(value) for value in argv)
        self.cwd = cwd
        self.env = None if env is None else dict(env)
        self.stdin = stdin
        self.stdout = stdout
        self.stderr = stderr
        if "start_new_session" in popen_options:
            raise ValueError("ManagedProcess always owns a new process session")
        self.popen_options = popen_options
        self.process: subprocess.Popen[Any] | None = None

    def start(self) -> ManagedProcess:
        if self.process is not None:
            raise ProcessFailure("managed process was already started")
        try:
            self.process = subprocess.Popen(
                self.argv,
                cwd=self.cwd,
                env=self.env,
                stdin=self.stdin,
                stdout=self.stdout,
                stderr=self.stderr,
                start_new_session=True,
                **self.popen_options,
            )
        except OSError as error:
            raise ProcessFailure(f"cannot start {self.argv[0]!r}: {error}") from error
        return self

    @property
    def pid(self) -> int:
        if self.process is None:
            raise ProcessFailure("managed process has not started")
        return self.process.pid

    def wait(self, timeout: float) -> int:
        if self.process is None:
            raise ProcessFailure("managed process has not started")
        try:
            return self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired as error:
            raise ProcessFailure(
                f"process {self.process.pid} exceeded {timeout:.3f}s: {self.argv!r}"
            ) from error

    def wait_for_exit(self, deadline: Deadline, *, poll_seconds: float = 0.05) -> int:
        if self.process is None:
            raise ProcessFailure("managed process has not started")
        while not deadline.expired():
            if (returncode := self.process.poll()) is not None:
                return returncode
            time.sleep(min(poll_seconds, deadline.remaining()))
        raise ProcessFailure(f"process {self.process.pid} exceeded its deadline")

    def terminate(self, *, grace_seconds: float = 5.0) -> None:
        self.stop(signal.SIGTERM, grace_seconds=grace_seconds)

    def stop(self, signal_number: int, *, grace_seconds: float = 5.0) -> None:
        process = self.process
        if process is None:
            return
        try:
            os.killpg(process.pid, signal_number)
        except ProcessLookupError:
            if process.poll() is None:
                process.wait(timeout=grace_seconds)
            return
        if process.poll() is None:
            try:
                process.wait(timeout=grace_seconds)
            except subprocess.TimeoutExpired:
                pass
        if self._wait_for_group_exit(process.pid, grace_seconds):
            return
        self.kill(grace_seconds=grace_seconds)

    def kill(self, *, grace_seconds: float = 5.0) -> None:
        process = self.process
        if process is None:
            return
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            return
        if process.poll() is None:
            process.wait(timeout=grace_seconds)
        if not self._wait_for_group_exit(process.pid, grace_seconds):
            raise ProcessFailure(f"process group {process.pid} survived SIGKILL")

    @staticmethod
    def _wait_for_group_exit(process_group: int, timeout: float) -> bool:
        deadline = Deadline.after(timeout)
        while not deadline.expired():
            try:
                os.killpg(process_group, 0)
            except ProcessLookupError:
                return True
            except PermissionError:
                # Darwin can report EPERM while a dying group still contains
                # a member the probe cannot signal. POSIX defines that as an
                # existing group, so keep waiting for ESRCH instead of
                # leaking the raw host error out of cleanup.
                pass
            time.sleep(min(0.01, deadline.remaining()))
        try:
            os.killpg(process_group, 0)
        except ProcessLookupError:
            return True
        except PermissionError:
            return False
        return False

    def communicate(self, timeout: float) -> tuple[bytes, bytes]:
        if self.process is None:
            raise ProcessFailure("managed process has not started")
        try:
            stdout, stderr = self.process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired as error:
            self.terminate()
            raise ProcessFailure(f"process {self.process.pid} exceeded {timeout:.3f}s") from error
        return stdout or b"", stderr or b""

    def receipt(self) -> ProcessReceipt:
        process = self.process
        if process is None:
            raise ProcessFailure("managed process has not started")
        return ProcessReceipt(
            argv=self.argv,
            pid=process.pid,
            returncode=process.poll(),
            stdout=None,
            stderr=None,
        )

    def __enter__(self) -> ManagedProcess:
        return self.start()

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        self.terminate()
