# Verification: lib/color + lib/cost findings (F12-F18, F38, F39)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust (commit f8763a5). All paths relative to that dir.

## F12 (major, dependencies, lib/color/src/lib.rs:51) - CONFIRMED

Sub-claim (a): regex used only for #rrggbb validation, and brdgme_color is the sole regex dependent.

- lib/color/src/lib.rs:51-66: `from_hex` builds `Regex::new(r"^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$")` inside `lazy_static!` - the crate's only regex use (only `use regex::Regex` at lib.rs:5, no other `Regex` in crate).
- lib/color/Cargo.toml:9-10: `lazy_static = "1.5.0"`, `regex = "1.12.4"`.
- Workspace grep for `regex` across all Cargo.toml files: only `lib/color/Cargo.toml`. Dropping it removes the regex tree from the workspace. (Note: `lazy_static` would remain - game/lords-of-vegas-1/Cargo.toml:15 also depends on it - but the finding only claims the regex/aho-corasick tree.)

Sub-claim (b): parse API effectively dead outside tests.

- Workspace grep for `from_hex`: lib/color internal (lib.rs:51,75; own tests lib.rs:165-167; palette.rs:3355 which is inside `#[cfg(test)]`), plus lib/markup/src/transform.rs:455,478. transform.rs's `#[cfg(test)] mod tests` begins at line 410-411, so both markup calls are test-only.
- Workspace grep for `parse::<Color>` / `Color::from_str`: only lib/color's own tests (lib.rs:174-178). The `FromStr for Color` impl (lib.rs:69-85) has no runtime caller; it is the only consumer of the private `named()` table (lib.rs:127-158).
- Markup runtime path uses `NamedColor::from_str` (lib/markup/src/parser.rs:156), not `Color::from_str`.
- Web maps player color names via its own `PLAYER_COLOR_NAMES` (web/src/theme.rs:65, used at web/src/theme.rs:73,458 and web/src/auth/server.rs:692).
- Four `.unwrap()`s confirmed: lib.rs:54 (Regex::new), lib.rs:58,59,60 (from_str_radix x3).

Both sub-claims hold. CONFIRMED at major/dependencies.

## F13 (minor, correctness, lib/color/src/lib.rs:28) - CONFIRMED

- lib.rs:28: `if self.r / 3 + self.g / 3 + self.b / 3 >= 128`. u8 integer division floors each channel independently.
- rgb(128,128,128): 128/3 = 42 (floor), 42*3 = 126 < 128 -> black branch (lib.rs:35), though true mean is 128, which per the >= 128 rule should be white. Worst case loses 2 per channel (up to 6 total), biasing the boundary dark. CONFIRMED.

## F14 (minor, consistency, lib/color/src/lib.rs:127) - CONFIRMED

Three divergent name tables verified:

- lib/color/src/lib.rs:127-158 `named()`: normalises to lowercase alpha-only; aliases include `deeppurple`->purple, `indigo`->blue, `lightblue`->blue, `teal`->cyan, `lightgreen`/`lime`->green, `amber`/`deeporange`->orange, `bluegrey`->cyan, `magenta`->purple, `white`->background, `black`->foreground.
- lib/color/src/palette.rs:61-83 `NamedColor::from_str`: exact lowercase match on the 12 canonical names only; no aliases.
- lib/markup/src/parser.rs:150-158 `resolve_named`: aliases `magenta`->Purple, `amber`->Orange, `black`->Foreground, `white`->Background, then falls through to `NamedColor::from_str`.

Same conceptual mapping implemented three inconsistent ways (e.g. "teal" resolves via `named()` but not via markup; markup accepts "magenta" but `NamedColor::from_str` does not). CONFIRMED.

## F15 (minor, simplicity, lib/color/src/palette.rs:138) - ADJUSTED

- palette.rs is 3814 lines; 379 `Color {` struct literals, each spanning 5 lines (e.g. palette.rs:139-143), giving ~1,900 lines of literals plus ~32 `Palette` wrappers - roughly 2,000 lines, not the claimed ~3,000 (remainder is doc comments, functions, tests).
- 34 palettes, all reachable: themes() (palette.rs:3192-3239) registers exactly 34 `(&str, ThemeCategory, &Palette)` entries.
- LIGHT_PROTANOPIA (palette.rs:2707-2720) and DARK_PROTANOPIA (palette.rs:2806-2819) are field-by-field copies of LIGHT_DEUTERANOPIA / DARK_DEUTERANOPIA (all 12 fields aliased individually), each documented as "Byte-identical" (palette.rs:2702, 2804).
- A `const fn rgb(r,g,b)` would collapse each 5-line literal to one line; realistic saving is ~1,500 lines (file to ~2,300), not to ~400.

Verdict: ADJUSTED - substance correct (verbose literals, duplicate protanopia palettes, const-fn fix applies), but the line-count estimates are overstated (~2,000 literal lines, not ~3,000; post-fix file would be far larger than ~400 lines because docs/functions/tests dominate the remainder). Severity minor stands.

## F16 (nit, quality, lib/color/src/palette.rs:3190) - CONFIRMED

- palette.rs:3190-3191 doc: "Light/Dark is assigned by each palette's actual `background` lightness, not by theme name."
- palette.rs:3194-3237: THEMES is a static array with the category hardcoded per entry (e.g. `("dracula", Dark, &DRACULA)`). No code computes lightness to assign categories; `relative_luminance` (palette.rs:3273) is used only for contrast. The doc describes an authoring convention as if it were mechanism. CONFIRMED.

## F17 (nit, consistency, lib/color/src/palette.rs:3266) - CONFIRMED

- Runtime: palette.rs:3264-3271 `srgb_channel_to_linear` uses `if c <= 0.03928`.
- Test-only: palette.rs:3436-3443 `lin` inside `rgb_to_lab` uses `if c > 0.04045`; palette.rs:3653-3660 `srgb_to_linear` uses `if c <= 0.04045`.
- Three near-identical linearisation functions with two different breakpoints (0.03928 is the legacy WCAG constant, 0.04045 the sRGB spec value; numerically near-equivalent but inconsistent). CONFIRMED.

## F18 (nit, quality, lib/color/src/lib.rs:47) - CONFIRMED

- lib.rs:48: `format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)` in `hex()`.
- lib.rs:121: identical format string in `impl fmt::Display for Color`. Either could delegate to the other. CONFIRMED.

## F38 (nit, quality, lib/cost/src/lib.rs:15) - CONFIRMED

- lib/cost/src/lib.rs:9-13: `impl<K: Hash + Eq> Default for Cost<K>` - no Clone bound.
- lib.rs:15-19: `impl<K: Hash + Eq + Clone> Cost<K>` block opens with `pub fn new() { Self::default() }`. `new()` therefore demands `K: Clone` though its body needs only Hash+Eq. Callers with non-Clone keys can use `Cost::default()` but not `Cost::new()`. CONFIRMED.

## F39 (minor, consistency, game/splendor-2/src/cost.rs:7) - CONFIRMED

Only-consumer claim: workspace grep for `brdgme_cost`/`brdgme-cost` hits only lib/cost itself plus game/seven-wonders-1 (Cargo.toml, src/lib.rs, src/card.rs). game/seven-wonders-1/src/card.rs:83 `resources: Cost<MultiResource>` and card.rs:99 `pub cost: Cost<Good>`. Confirmed sole consumer.

Semantic equivalence (splendor-2 cost.rs vs lib/cost):
- Origin: cost.rs:7-8 header says "Ported from `brdgme-go/libcost/cost.go`", i.e. same Go source as lib/cost.
- `from_resources` (cost.rs:21-27) vs `from_keys` (lib/cost lib.rs:22-28): both `*entry(k).or_insert(0) += 1` per input item. Equivalent (from_resources takes `&[Resource]` with Copy keys; from_keys takes `IntoIterator<Item=K>` and clones - same result).
- `add` (cost.rs:38-46 vs lib.rs:31-37): both clone self and add other's entries via or_insert(0). Identical.
- `inv` (cost.rs:49-55 vs lib.rs:40-42): negate every value. Identical.
- `sub` (cost.rs:58-60 vs lib.rs:45-47): both `self.add(&other.inv())`. Identical.
- `sum` (cost.rs:71-73 vs lib.rs:112-114): both `self.0.values().sum()`. Identical.
- `Cost::can_afford`: splendor (cost.rs:65-68) `self.sub(other).0.values().all(|&v| v >= 0)`; lib/cost (lib.rs:64-67) `self.sub(other).pos_neg()` then `neg.0.is_empty()`. pos_neg's neg map (lib.rs:50-61) collects exactly entries with v < 0, so neg empty iff all diff values >= 0. Equivalent.
- Splendor-specific parts, correctly identified: `get`/`set` accessors (cost.rs:30-36) and the free-fn gold-shortfall `can_afford(a, c)` (cost.rs:79-87), which folds Gold into per-resource shortfall - no lib/cost analogue.

All claims verified. CONFIRMED at minor/consistency.
