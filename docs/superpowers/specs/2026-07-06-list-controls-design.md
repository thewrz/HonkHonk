# List-Controls Framework — Type-to-Filter + Sort for All List Views

Date: 2026-07-06 · Status: approved design, filed as issues (epic + children)

## Why

Two foundational UX behaviors users expect from modern list interfaces, built once and
wired into any current or future view:

1. **Type-to-filter** — on views that enable it, simply typing immediately filters the
   list (by tag/folder, file name, and customized tile name) without clicking into the
   search box first. Click-to-focus always keeps working.
2. **Sorting** — a chip showing the active sort key with an ascending/descending
   chevron, plus an options menu (by name, date, length, …), persisted per view.

Grouping is out of scope for v1: favorites (#14) and folder-derived categories already
cover it. The sort menu keeps a **group-by seam** for when user-assignable sound tags
exist (future enabler issue).

## Modules — `src/ui/list_controls/`

Approach: shared controller structs + pure functions + view factories, state owned by
`HonkHonk` per view, message-mapper closures like the existing `search_bar.rs` pattern.
No iced internal-state components; no per-view duplication. Each file stays under the
400-line cap.

### `filter.rs`

- `FilterState { query: String, had_focus: bool }` — carries the Escape staging proven
  in #87: first Escape blurs, second clears.
- `filter_items<T>(items: &[T], query: &str, haystacks: impl Fn(&T) -> …) -> Vec<&T>` —
  pure, case-insensitive substring match over each item's haystack fields (customized
  display name, file name, tag/category).
- `Activation` per view: `TypeToFilter` or `ClickOnly`.
- The existing `search_bar.rs` becomes the shared input widget; the main grid's
  `search_query` / `filtered_sounds()` migrate onto `FilterState` (one implementation).

### `sort.rs`

- `SortState<K: SortKey> { key: K, direction: Direction }`.
- `SortKey` trait: `label()`, `compare(&T, &T)`; implemented by per-view enums:
  - `SoundSortKey { Name, Length, Tag, Modified, Added }` (tiles)
  - `SlotSortKey { SlotNumber, Name, Length, Tag, Modified, Added }` (slots, shortcuts)
  - `MacroSortKey { Name, Created, Length }` (macros view, #168)
- View factories: `view_sort_chip()` (label opens the options menu; chevron toggles
  direction) and the options menu.

### Type-to-activate plumbing (app level)

One keyboard subscription routes printable `KeyPressed` events into the active view's
`FilterState` (focus the search bar + insert the character) only when **all** hold:

- the active view's activation is `TypeToFilter`;
- no overlay is open (sound editor, context menu, macro editor);
- a new `text_entry_active: bool` app flag is false — set/cleared by focus events of
  other text inputs, which keeps #168's inline editors safe from hijacking.

Only Escape is currently intercepted app-wide; slot hotkeys arrive via the portal, not
in-app keypresses, so there are no key conflicts.

### Persistence

- Config gains `sort_prefs: BTreeMap<String, SortPref>` keyed by view id (`"tiles"`,
  `"slots"`, `"shortcuts"`, `"macros"`), written through `persist_config()`.
- Filter queries are never persisted — every view starts unfiltered on launch.

## Data enablers

- **File modified time** — `SoundEntry.modified_ms: Option<u64>` read from fs metadata
  during the existing library scan (no extra I/O pass).
- **Date added** — `sound_meta` gains a dedicated `added: BTreeMap<sound_id, epoch_ms>`
  map stamped on first sight of a sound id and pruned with the existing stale-sound
  cleanup. Deliberately not a per-entry field: the store prunes all-default entries to
  keep `meta.json` small, and a per-entry timestamp would defeat that.
- **Settings registry** — each entry gains `label` + `keywords` + section, making the
  registry the search index for staged settings search.

## Wiring matrix

| View | Filter | Type-to-activate | Sort (default) |
|---|---|---|---|
| Main tiles | migrated onto `FilterState` | yes (+ click stays) | `SoundSortKey` (Name ↑) |
| Slots | yes | yes | `SlotSortKey` (SlotNumber ↑) |
| Shortcut assignment | yes | yes | `SlotSortKey` (SlotNumber ↑) |
| Macros (#168) | yes | yes | `MacroSortKey` (Name ↑) |
| Settings | staged search (below) | no — click-only | none |

Sorting the slots view reorders the **list rendering only** — slot assignments and
numbers stay attached to their rows; it never remaps which sound lives in which slot.

## Staged settings search (Discord-style)

1. Search activates by click only (no type-to-activate in settings).
2. Typing narrows the **category sidebar** to categories containing matches.
3. Clicking a filtered category shows **only the matching setting(s), highlighted**.
4. Clearing the query restores all categories and all settings, with the scrollable
   **anchored on the setting the user was manipulating**.

The scroll-anchor restore (iced 0.14 `scrollable::Id` + offset bookkeeping) is the
fiddliest piece and is isolated in its own issue.

## Testing

Pure-function tests: `filter_items` matching, per-key `compare` ordering, Escape
staging, activation guard, sort-pref round-trip through config. App-state boundary
tests only — no iced view-rendering tests (repo doctrine).

## Issue map

| # | Issue | Depends on |
|---|---|---|
| epic | list-controls framework (tracking, carries this matrix) | — |
| 1 | feat(state): capture file modified-time + first-seen timestamps | — |
| 2 | feat(ui): filter module + type-to-filter activation (+ main-grid migration) | — |
| 3 | feat(ui): sort module — chip + options menu + persisted prefs (tiles first) | 1 |
| 4 | feat(ui): wire filter + sort into the slots view | 2, 3 |
| 5 | feat(ui): wire filter + sort into the shortcut-assignment view | 2, 3 |
| 6 | feat(ui): staged settings search (Discord-style) | 2 |
| 7 | feat(state,ui): user-assignable sound tags + group-by (backlog enabler) | — |

`#168` (macros view) is amended by comment: its list consumes the framework.

Related prior art: #8 (original search bar), #87 (Escape/focus staging), #14
(favorites), #16 (bulk-import review screen — future consumer).
