#!/usr/bin/env bash
# Repo-wide regression guard for the io.github.thewrz -> io.github.wrzonance
# Flatpak identity rename (#207).
#
# Pins two invariants at the boundary of the migration:
#   1. No stale 'io.github.thewrz' substring remains anywhere under the
#      Flatpak packaging surface, the Flatpak workflow, or the Flatpak test
#      scripts — except the two intentional canary literals inside
#      flatpak_validate.sh's own regression check, which exist specifically
#      to assert the string's absence elsewhere and must not be miscounted
#      as a leftover.
#   2. Packaging surfaces explicitly out of scope for this migration
#      (packaging/aur/**) are byte-identical to the pre-migration base —
#      this rename must not leak edits into unrelated packaging.
# Exits non-zero if any check fails.
set -euo pipefail

PASS=0
FAIL=0

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

echo "=== Flatpak identity sweep (#207) ==="

# ── Invariant 1: no stale identity outside the known canary lines ─────
STALE="io.github.thewrz"
CANARY_FILE="tests/packaging/flatpak_validate.sh"
CANARY_COUNT=2
SWEEP_TARGETS=(
    packaging/flatpak/io.github.wrzonance.HonkHonk.yml
    packaging/flatpak/io.github.wrzonance.HonkHonk.metainfo.xml
    packaging/flatpak/cargo-sources.json
    packaging/flatpak/flatpak-cargo-generator.py
    .github/workflows/flatpak.yml
    tests/packaging/flatpak_cargo_sources_fresh.sh
)

hits=""
for f in "${SWEEP_TARGETS[@]}"; do
    if [ -f "$f" ] && grep -qF "$STALE" "$f"; then
        hits="$hits $f"
    fi
done
if [ -z "$hits" ]; then
    check "no stale '$STALE' outside the canary file" "ok"
else
    check "no stale '$STALE' outside the canary file" "found in:$hits"
fi

# flatpak_validate.sh is allowed exactly CANARY_COUNT occurrences: the
# grep pattern of its own regression guard and that guard's failure
# message, both asserting the string's absence — never a real app-id use.
if [ -f "$CANARY_FILE" ]; then
    canary_count=$(grep -oF "$STALE" "$CANARY_FILE" | wc -l | tr -d ' ')
    if [ "$canary_count" -eq "$CANARY_COUNT" ]; then
        check "$CANARY_FILE has exactly the $CANARY_COUNT known canary literals" "ok"
    else
        check "$CANARY_FILE has exactly the $CANARY_COUNT known canary literals" \
            "found $canary_count occurrences — new stale reference or guard removed?"
    fi
else
    check "$CANARY_FILE exists" "missing"
fi

# ── Invariant 2: out-of-scope packaging is untouched by this branch ───
BASE_REF="${FLATPAK_SWEEP_BASE_REF:-origin/main}"
OUT_OF_SCOPE=(packaging/aur)

if git rev-parse --verify --quiet "$BASE_REF" >/dev/null; then
    merge_base=$(git merge-base HEAD "$BASE_REF" 2>/dev/null || echo "$BASE_REF")
    changed=$(git diff --name-only "$merge_base" HEAD -- "${OUT_OF_SCOPE[@]}")
    if [ -z "$changed" ]; then
        check "packaging/aur/** byte-identical to $BASE_REF" "ok"
    else
        check "packaging/aur/** byte-identical to $BASE_REF" \
            "unexpected changes: $(echo "$changed" | tr '\n' ' ')"
    fi
else
    check "packaging/aur/** byte-identical to $BASE_REF" \
        "cannot resolve $BASE_REF — set FLATPAK_SWEEP_BASE_REF to a reachable ref"
fi

# ── Summary ───────────────────────────────────────────────────────────
echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
