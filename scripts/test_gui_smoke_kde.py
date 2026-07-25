"""Unit tests for the KDE real-pixel smoke harness."""

import unittest

from gui_smoke_kde import app_environment, is_non_black_pixels


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


class LaunchEnvironmentTests(unittest.TestCase):
    def test_forces_wayland_zero_and_wgpu(self) -> None:
        """Real-pixel launches target isolated Wayland with wgpu."""
        environment = app_environment()

        self.assertEqual(environment["WAYLAND_DISPLAY"], "wayland-0")
        self.assertEqual(environment["HONKHONK_RENDERER"], "wgpu")


if __name__ == "__main__":
    unittest.main()
