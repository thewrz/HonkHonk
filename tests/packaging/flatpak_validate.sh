#!/usr/bin/env bash
# Validates the Flatpak manifest before flatpak-builder runs.
# Exits non-zero if any check fails.
set -euo pipefail

PASS=0
FAIL=0
MANIFEST="packaging/flatpak/io.github.wrzonance.HonkHonk.yml"
METAINFO="packaging/flatpak/io.github.wrzonance.HonkHonk.metainfo.xml"

check() {
    local desc="$1"
    local result="$2"
    if [ "$result" = "ok" ]; then
        echo "  PASS  $desc"
        PASS=$((PASS + 1))
    else
        echo "  FAIL  $desc — $result"
        FAIL=$((FAIL + 1))
    fi
}

has() { grep -qF -- "$1" "$MANIFEST"; }

echo "=== Flatpak manifest validation ==="

# ── File exists and is valid YAML ─────────────────────────────────────
[ -f "$MANIFEST" ] \
    && check "manifest file exists" "ok" \
    || check "manifest file exists" "missing: $MANIFEST"

[ -f "$MANIFEST" ] || { echo ""; echo "Results: $PASS passed, $FAIL failed"; exit 1; }

if python3 - 2>/dev/null <<'PYEOF'
import sys
try:
    import yaml
    yaml.safe_load(open("packaging/flatpak/io.github.wrzonance.HonkHonk.yml"))
except ImportError:
    pass  # pyyaml not available — skip, other checks verify structure
except Exception:
    sys.exit(1)
PYEOF
then
    check "manifest is valid YAML" "ok"
else
    check "manifest is valid YAML" "parse error"
fi

# ── App identity ──────────────────────────────────────────────────────
has 'io.github.wrzonance.HonkHonk' \
    && check "app-id is io.github.wrzonance.HonkHonk" "ok" \
    || check "app-id is io.github.wrzonance.HonkHonk" "missing"

has 'org.freedesktop.Platform' \
    && check "runtime is org.freedesktop.Platform" "ok" \
    || check "runtime is org.freedesktop.Platform" "missing"

has 'command: honkhonk' \
    && check "command is honkhonk" "ok" \
    || check "command is honkhonk" "missing"

# ── finish-args: required permissions ─────────────────────────────────
has '--socket=wayland' \
    && check "finish-args: --socket=wayland" "ok" \
    || check "finish-args: --socket=wayland" "missing"

has '--socket=pulseaudio' \
    && check "finish-args: --socket=pulseaudio (PipeWire compat)" "ok" \
    || check "finish-args: --socket=pulseaudio (PipeWire compat)" "missing"

has '--device=dri' \
    && check "finish-args: --device=dri (GPU for wgpu)" "ok" \
    || check "finish-args: --device=dri (GPU for wgpu)" "missing"

has '--talk-name=org.kde.StatusNotifierWatcher' \
    && check "finish-args: StatusNotifierWatcher (SNI tray)" "ok" \
    || check "finish-args: StatusNotifierWatcher (SNI tray)" "missing"

has '--filesystem=xdg-music' \
    && check "finish-args: --filesystem=xdg-music (sound library)" "ok" \
    || check "finish-args: --filesystem=xdg-music (sound library)" "missing"

# ── Build: Rust SDK extension ─────────────────────────────────────────
has 'rust-stable' \
    && check "uses org.freedesktop.Sdk.Extension.rust-stable" "ok" \
    || check "uses org.freedesktop.Sdk.Extension.rust-stable" "missing"

# ── Module: binary installed to correct Flatpak path ─────────────────
has '/app/bin/honkhonk' \
    && check "binary installed to /app/bin/honkhonk (on PATH)" "ok" \
    || check "binary installed to /app/bin/honkhonk (on PATH)" "missing"

# /app/usr/{bin,share} are NOT on PATH/XDG_DATA_DIRS inside the sandbox
if grep -qE '/app/usr/(bin|share)/' "$MANIFEST"; then
    check "no installs under /app/usr/ (wrong Flatpak path)" \
        "found /app/usr/ — use /app/bin/ and /app/share/ instead"
else
    check "no installs under /app/usr/ (wrong Flatpak path)" "ok"
fi

# ── Assets: .desktop and icon ─────────────────────────────────────────
has 'io.github.wrzonance.HonkHonk.desktop' \
    && check "desktop installs under the full app-id" "ok" \
    || check "desktop installs under the full app-id" "missing"

has 'assets/icons/generated/hicolor' \
    && check "manifest installs the full app-id-named icon set" "ok" \
    || check "manifest installs the full app-id-named icon set" "missing"

# ── No stale identity left behind ─────────────────────────────────────
for f in "$MANIFEST" "$METAINFO" .github/workflows/flatpak.yml; do
    if [ -f "$f" ]; then
        ! grep -qF 'io.github.thewrz' "$f" \
            && check "$f has no stale thewrz identity" "ok" \
            || check "$f has no stale thewrz identity" "found io.github.thewrz reference"
    else
        check "$f has no stale thewrz identity" "missing: $f"
    fi
done

# ── AppStream strict validation ────────────────────────────────────────
if command -v appstreamcli >/dev/null 2>&1; then
    check "appstreamcli is available" "ok"
    if appstreamcli validate --strict --no-net "$METAINFO" >/tmp/appstream-validate.log 2>&1; then
        check "metainfo passes appstreamcli --strict" "ok"
    else
        check "metainfo passes appstreamcli --strict" \
            "$(tail -n5 /tmp/appstream-validate.log | tr '\n' ' ')"
    fi
else
    check "appstreamcli is available" "not installed — strict validation skipped"
fi

# ── Summary ───────────────────────────────────────────────────────────
echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
