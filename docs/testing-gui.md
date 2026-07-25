# GUI testing

HonkHonk uses two deliberately separate GUI test layers:

- Layer A uses `iced_test` in ordinary `cargo test` runs. It validates the real
  `view`/`update` loop and widget-tree interactions without a compositor.
- Layer B is this KDE-only real-pixel smoke harness. It launches the production
  wgpu window in virtual KWin, captures it, and rejects a black frame.

Layer B is a developer/agent verification tool, not a required CI job. It does
not validate audio, portals, tray integration, or accessibility. Track the
unproven lavapipe/llvmpipe CI experiment separately in issue #217.

## Vetted driver and pin

The sanctioned driver is
[`kwin-mcp`](https://github.com/isac322/kwin-mcp), installed as a user tool and
never added to HonkHonk's Cargo or runtime dependencies.

The reviewed pin is:

- Version/tag: `v0.7.0`
- Git commit: `a119f399d444cb402505ff1a380305eb13fcfcec`
- License: MIT
- PyPI wheel SHA-256:
  `f4175b36a2869a9c4dfaad7992c905a97b11bd5a5079e713ee20321227424559`

The audit compared every Python file in that wheel with the tagged source. The
direct dependency chain is MCP (MIT), Pillow (MIT-CMU), dbus-python (MIT/X11),
and PyGObject (LGPL; with pycairo under LGPL-2.1-only or MPL-1.1). The LGPL
components remain isolated user tooling and are not linked into or distributed
with HonkHonk.

Do not reproduce the upstream tag's March 2026 `uv.lock`: a July 2026
`pip-audit` found known vulnerabilities in that stale graph. Installing the
hash-pinned top-level wheel while resolving current compatible transitives
produced a 34-package environment with no known vulnerabilities. Re-audit every
fresh install.

`kwin-mcp` deliberately executes desktop commands and talks to KWin over D-Bus.
The reviewed source contains no network client, dynamic code evaluation, or
shell-enabled subprocesses. It uses KWin's private
`org.kde.KWin.EIS.RemoteDesktop` interface for libei input injection. KDE may
change that interface without compatibility guarantees; this is an accepted
test-tool risk, not an application API.

## Install on the KDE development box

The current Manjaro box needs KWin 6, Spectacle, AT-SPI's bus launcher,
`dbus-run-session`, Python 3.12 or newer, and `uv`. Keep the Python environment
outside the repository:

```bash
GUI_VENV="${XDG_DATA_HOME:-$HOME/.local/share}/honkhonk/gui-test"
uv venv --python 3.12 "$GUI_VENV"
uv pip install --python "$GUI_VENV/bin/python" \
  'https://files.pythonhosted.org/packages/d4/26/c57e82a8c17029b647ff2ba357e590c683e7f12db4aa631ba678ef5de6b1/kwin_mcp-0.7.0-py3-none-any.whl#sha256=f4175b36a2869a9c4dfaad7992c905a97b11bd5a5079e713ee20321227424559'
```

Confirm the installed graph against current advisory data:

```bash
GUI_SITE="$("$GUI_VENV/bin/python" -c \
  'import sysconfig; print(sysconfig.get_paths()["purelib"])')"
uvx pip-audit --path "$GUI_SITE"
```

Register the same executable as a user-level MCP server. Use the command for
the client that will drive it:

```bash
codex mcp add kwin-mcp -- "$GUI_VENV/bin/kwin-mcp"
claude mcp add --scope user kwin-mcp -- "$GUI_VENV/bin/kwin-mcp"
```

Restart the MCP client after changing its user configuration.

## Run the isolated pixel smoke

Build first, then run the checked-in driver through the pinned environment:

```bash
cargo build --bin honkhonk
"$GUI_VENV/bin/python" scripts/gui_smoke_kde.py
```

The script:

1. Creates a private runtime directory and starts
   `dbus-run-session + kwin_wayland --virtual`.
2. Names the private compositor socket `wayland-0`.
3. launches HonkHonk with `WAYLAND_DISPLAY=wayland-0` and
   `HONKHONK_RENDERER=wgpu`.
4. captures through KWin ScreenShot2, with Spectacle as the fallback.
5. writes `target/gui-smoke/honkhonk.png` only after at least 1% of the central
   client-region pixels are brighter than the near-black cutoff. Cropping out
   KWin's decoration prevents a painted title bar from masking a black client.

The isolated runtime has no real PipeWire or portal service. Corresponding
in-app error banners are expected and do not invalidate this renderer smoke.
The process group, temporary HOME, D-Bus, and compositor are removed on exit.

Useful overrides:

```bash
"$GUI_VENV/bin/python" scripts/gui_smoke_kde.py \
  --binary target/release/honkhonk \
  --output target/gui-smoke/release.png \
  --timeout 30
```

`--width` and `--height` resize the virtual output, not the winit window.
Programmatic window resize does not reliably reach winit here, so never use
those options as a window-sizing assertion.

Never set `HONKHONK_RENDERER=software` for a pixel check. Tiny-skia renders
black on this development box; it remains a functional fallback only.

Run the small script-boundary tests independently:

```bash
python -m unittest discover -s scripts -p 'test_*.py'
```

## Coordinate calibration before input

Iced 0.14 exposes no AccessKit/AT-SPI tree, so do not use kwin-mcp's
accessibility queries with HonkHonk. Drive the live `wayland-0` wgpu window by
screenshots and EIS input instead:

1. Capture a frame and record the HonkHonk window's top-left screen position.
2. Treat coordinates measured inside the Wayland surface as window-local.
   Translate them to the compositor's absolute coordinate space before sending
   an EIS click.
3. Click the visible **Settings** button as the calibration target.
4. Capture another frame and verify the settings view appeared.
5. Only after that state change should the same offset be trusted for sort-chip
   or search-field clicks. Recalibrate after moving the window or changing
   output scale.

Keyboard injection follows a US-QWERTY map. If Unicode entry becomes necessary,
prefer the separately licensed `wtype` or `wl-clipboard` path; do not add
`ydotool`, `dotool`, X11 tools, or an accessibility driver.

## Portable fallback

If KDE removes the private EIS interface, the sanctioned portable design is
ashpd's RemoteDesktop portal plus libei. Request `PersistMode::ExplicitlyRevoked`,
save the returned `restore_token`, and replace it with the new token after every
successful restoration because tokens are single-use. The first authorization
may show a portal dialog; restored sessions can then run silently while the
permission remains valid.

That fallback is documented only. Do not implement it in this harness until the
private KWin path actually becomes unusable.
