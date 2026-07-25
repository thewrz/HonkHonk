#!/usr/bin/env python3
"""Launch HonkHonk in virtual KWin and reject a black screenshot."""

from __future__ import annotations

import argparse
import contextlib
import importlib.metadata
import os
import shutil
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
    """Classify the central client region, excluding KWin decoration."""
    from PIL import Image

    with Image.open(path) as image:
        width, height = image.size
        center = image.crop((width // 4, height // 4, width * 3 // 4, height * 3 // 4))
        return is_non_black_pixels(center.convert("RGB").get_flattened_data())


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


def capture_frame(info: object, attempt: int) -> Path:
    """Capture with ScreenShot2 and fall back to Spectacle."""
    import dbus
    from kwin_mcp.screenshot import capture_screenshot_dbus, capture_screenshot_to_file

    output_dir = info.screenshot_dir
    path = output_dir / f"probe_{attempt:03d}.png"
    try:
        return capture_screenshot_dbus(info.dbus_address, path)
    except (dbus.DBusException, OSError, RuntimeError, ValueError) as error:
        print(f"ScreenShot2 unavailable ({error}); using Spectacle", file=sys.stderr)
        return capture_screenshot_to_file(
            dbus_address=info.dbus_address,
            wayland_socket=info.wayland_socket,
            output_dir=output_dir,
        )


def wait_for_frame(session: object, app: object, info: object, timeout: int) -> Path:
    """Wait until the app paints a non-black frame or exits."""
    deadline = time.monotonic() + timeout
    attempt = 0
    while time.monotonic() < deadline:
        exit_code = app.process.poll()
        if exit_code is not None:
            log = session.read_app_log(app.pid, last_n_lines=80)
            raise RuntimeError(f"HonkHonk exited with {exit_code}\n{log}")
        attempt += 1
        frame = capture_frame(info, attempt)
        if frame_is_non_black(frame):
            return frame
        time.sleep(0.25)
    raise RuntimeError(f"no non-black HonkHonk frame within {timeout}s")


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
