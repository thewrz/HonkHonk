"""Unit tests for the KDE real-pixel smoke harness."""

import tempfile
import unittest
from pathlib import Path
from unittest import mock

from PIL import Image

from gui_smoke_kde import (
    app_environment,
    capture_active_window_dbus,
    frame_is_non_black,
    is_non_black_pixels,
)


class PixelClassificationTests(unittest.TestCase):
    def test_rejects_an_all_black_frame(self) -> None:
        """An entirely black capture cannot satisfy the smoke check."""
        self.assertFalse(is_non_black_pixels([(0, 0, 0)] * 1_000))

    def test_accepts_a_frame_with_visible_content(self) -> None:
        """Enough visibly bright pixels satisfy the smoke check."""
        pixels = [(0, 0, 0)] * 989 + [(40, 40, 40)] * 11

        self.assertTrue(is_non_black_pixels(pixels))

    def test_rejects_isolated_bright_noise(self) -> None:
        """Sparse bright noise cannot hide an otherwise black frame."""
        pixels = [(0, 0, 0)] * 991 + [(255, 255, 255)] * 9

        self.assertFalse(is_non_black_pixels(pixels))

    def test_checks_the_entire_active_window_capture(self) -> None:
        """Visible content outside the screen center still counts."""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "active-window.png"
            image = Image.new("RGB", (100, 100))
            image.paste((40, 40, 40), (0, 0, 20, 20))
            image.save(path)

            self.assertTrue(frame_is_non_black(path))


class LaunchEnvironmentTests(unittest.TestCase):
    def test_forces_wayland_zero_and_wgpu(self) -> None:
        """Real-pixel launches target isolated Wayland with wgpu."""
        environment = app_environment()

        self.assertEqual(environment["WAYLAND_DISPLAY"], "wayland-0")
        self.assertEqual(environment["HONKHONK_RENDERER"], "wgpu")


class CaptureCleanupTests(unittest.TestCase):
    @mock.patch("os.close")
    @mock.patch("os.pipe", return_value=(17, 18))
    @mock.patch("dbus.types.UnixFd", side_effect=lambda descriptor: descriptor)
    @mock.patch("dbus.Interface")
    @mock.patch("dbus.bus.BusConnection")
    def test_closes_both_pipe_ends_when_dbus_capture_fails(
        self,
        connection: mock.Mock,
        interface: mock.Mock,
        _unix_fd: mock.Mock,
        _pipe: mock.Mock,
        close: mock.Mock,
    ) -> None:
        """A failed ScreenShot2 request cannot leak either descriptor."""
        connection.return_value.get_object.return_value = mock.Mock()
        interface.return_value.CaptureActiveWindow.side_effect = RuntimeError("failed")

        with self.assertRaisesRegex(RuntimeError, "failed"):
            capture_active_window_dbus("unix:path=/tmp/test-bus", Path("unused.png"))

        self.assertCountEqual(close.call_args_list, [mock.call(17), mock.call(18)])


if __name__ == "__main__":
    unittest.main()
