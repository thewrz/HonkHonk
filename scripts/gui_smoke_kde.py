#!/usr/bin/env python3
"""Launch HonkHonk in virtual KWin and reject a black screenshot."""

from __future__ import annotations

import argparse
import contextlib
import importlib.metadata
import os
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Iterable, Iterator
from pathlib import Path

KWIN_MCP_VERSION = "0.7.0"
WAYLAND_SOCKET = "wayland-0"
BRIGHT_CHANNEL_THRESHOLD = 12
MIN_VISIBLE_FRACTION = 0.01


def app_environment() -> dict[str, str]:
    """Return the renderer/display contract for real-pixel checks."""
    return {
        "HONKHONK_RENDERER": "wgpu",
        "ICED_BACKEND": "wgpu",
        "WAYLAND_DISPLAY": WAYLAND_SOCKET,
    }


def is_non_black_pixels(pixels: Iterable[tuple[int, ...]]) -> bool:
    """Return whether enough pixels contain visible, non-black content."""
    visible = 0
    total = 0
    for pixel in pixels:
        total += 1
        if max(pixel[:3]) > BRIGHT_CHANNEL_THRESHOLD:
            visible += 1
    return total > 0 and visible / total >= MIN_VISIBLE_FRACTION


def frame_is_non_black(path: Path) -> bool:
    """Classify the captured active-window client image."""
    from PIL import Image

    with Image.open(path) as image:
        return is_non_black_pixels(image.convert("RGB").get_flattened_data())


def positive_number(value: str) -> int:
    """Parse a positive integer for argparse."""
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return parsed


def parse_args() -> argparse.Namespace:
    """Parse smoke-harness command-line arguments."""
    repo = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=repo / "target/debug/honkhonk")
    parser.add_argument(
        "--output",
        type=Path,
        default=repo / "target/gui-smoke/honkhonk.png",
    )
    parser.add_argument("--timeout", type=positive_number, default=20)
    parser.add_argument("--width", type=positive_number, default=1280)
    parser.add_argument("--height", type=positive_number, default=800)
    return parser.parse_args()


def require_runtime(binary: Path) -> None:
    """Fail early when the pinned tool or KDE runtime is unavailable."""
    try:
        installed = importlib.metadata.version("kwin-mcp")
    except importlib.metadata.PackageNotFoundError as error:
        raise RuntimeError("kwin-mcp is not installed; follow docs/testing-gui.md") from error
    if installed != KWIN_MCP_VERSION:
        raise RuntimeError(f"kwin-mcp {KWIN_MCP_VERSION} required; found {installed}")

    required = ("kwin_wayland", "dbus-run-session", "spectacle")
    missing = [name for name in required if shutil.which(name) is None]
    if missing:
        raise RuntimeError(f"missing runtime commands: {', '.join(missing)}")
    if not Path("/usr/lib/at-spi-bus-launcher").is_file():
        raise RuntimeError("missing /usr/lib/at-spi-bus-launcher")
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise RuntimeError(f"build the executable first: cargo build --bin honkhonk ({binary})")


@contextlib.contextmanager
def isolated_runtime(runtime_dir: Path) -> Iterator[None]:
    """Point the virtual compositor at a private wayland-0 socket."""
    previous = os.environ.get("XDG_RUNTIME_DIR")
    os.environ["XDG_RUNTIME_DIR"] = str(runtime_dir)
    try:
        yield
    finally:
        if previous is None:
            os.environ.pop("XDG_RUNTIME_DIR", None)
        else:
            os.environ["XDG_RUNTIME_DIR"] = previous


def capture_active_window_dbus(dbus_address: str, path: Path) -> Path:
    """Capture the active client window through KWin ScreenShot2."""
    import dbus
    import dbus.bus
    from PIL import Image

    bus = dbus.bus.BusConnection(dbus_address)
    screenshot = bus.get_object("org.kde.KWin", "/org/kde/KWin/ScreenShot2")
    interface = dbus.Interface(screenshot, "org.kde.KWin.ScreenShot2")
    read_fd, write_fd = os.pipe()
    try:
        options = {
            "include-cursor": dbus.Boolean(False),
            "include-decoration": dbus.Boolean(False),
        }
        results = interface.CaptureActiveWindow(options, dbus.types.UnixFd(write_fd))
    finally:
        os.close(write_fd)

    try:
        chunks = []
        while chunk := os.read(read_fd, 65_536):
            chunks.append(chunk)
    finally:
        os.close(read_fd)

    data = b"".join(chunks)
    if not data:
        raise RuntimeError("KWin ScreenShot2 returned no active-window data")
    image = Image.frombytes(
        "RGBA",
        (int(results["width"]), int(results["height"])),
        data,
        "raw",
        "BGRA",
        int(results["stride"]),
    )
    image.save(path, "PNG")
    return path


def capture_active_window_spectacle(info: object, path: Path) -> Path:
    """Capture the active client window through the Spectacle fallback."""
    environment = {
        **os.environ,
        "DBUS_SESSION_BUS_ADDRESS": info.dbus_address,
        "WAYLAND_DISPLAY": info.wayland_socket,
    }
    command = ["spectacle", "-b", "-a", "-e", "-n", "-o", str(path)]
    try:
        subprocess.run(
            command,
            env=environment,
            capture_output=True,
            check=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise RuntimeError(f"Spectacle active-window capture failed: {error}") from error
    if not path.is_file():
        raise RuntimeError("Spectacle active-window capture produced no file")
    return path


def capture_frame(info: object, attempt: int) -> Path:
    """Capture the active window with ScreenShot2, then Spectacle."""
    import dbus

    output_dir = info.screenshot_dir
    path = output_dir / f"probe_{attempt:03d}.png"
    try:
        return capture_active_window_dbus(info.dbus_address, path)
    except (dbus.DBusException, OSError, RuntimeError, ValueError) as error:
        print(f"ScreenShot2 unavailable ({error}); using Spectacle", file=sys.stderr)
        return capture_active_window_spectacle(info, path)


def wait_for_frame(session: object, app: object, info: object, timeout: int) -> Path:
    """Wait until the app paints a non-black frame or exits."""
    deadline = time.monotonic() + timeout
    attempt = 0
    capture_error = ""
    while time.monotonic() < deadline:
        exit_code = app.process.poll()
        if exit_code is not None:
            log = session.read_app_log(app.pid, last_n_lines=80)
            raise RuntimeError(f"HonkHonk exited with {exit_code}\n{log}")
        attempt += 1
        try:
            frame = capture_frame(info, attempt)
        except RuntimeError as error:
            capture_error = str(error)
            time.sleep(0.25)
            continue
        if frame_is_non_black(frame):
            return frame
        time.sleep(0.25)
    detail = f"; last capture error: {capture_error}" if capture_error else ""
    raise RuntimeError(f"no non-black HonkHonk frame within {timeout}s{detail}")


def run_smoke(args: argparse.Namespace) -> Path:
    """Run the isolated compositor, app, capture, and pixel assertion."""
    from kwin_mcp.session import Session, SessionConfig

    binary = args.binary.resolve()
    output = args.output.resolve()
    require_runtime(binary)
    output.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="honkhonk-gui-runtime-") as runtime_name:
        runtime_dir = Path(runtime_name)
        runtime_dir.chmod(0o700)
        with isolated_runtime(runtime_dir), Session() as session:
            info = session.start(
                SessionConfig(
                    socket_name=WAYLAND_SOCKET,
                    screen_width=args.width,
                    screen_height=args.height,
                    isolate_home=True,
                )
            )
            app = session.launch_app([str(binary)], extra_env=app_environment())
            frame = wait_for_frame(session, app, info, args.timeout)
            shutil.copy2(frame, output)
    return output


def main() -> int:
    """CLI entry point."""
    try:
        output = run_smoke(parse_args())
    except (ImportError, OSError, RuntimeError) as error:
        print(f"GUI smoke failed: {error}", file=sys.stderr)
        return 1
    print(f"GUI smoke passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
