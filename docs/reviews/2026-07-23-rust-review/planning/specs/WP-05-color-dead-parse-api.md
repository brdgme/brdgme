# WP-05: lib/color - delete the dead parse API

**Findings:** ls F12 (major), ls F13/F14/F15 (minor), ls F16/F17/F18 (nit).
**Decision:** D-39 answered **option A - delete `Color::from_hex` and
`impl FromStr for Color`**, which drops `regex` + `lazy_static` workspace-wide
and resolves F14's three-way alias divergence by deletion.

> **Read the named items before editing. If one does not match what this spec
> describes, STOP and report rather than improvising.** Code is under concurrent
> edit; deletions are specified by **item name only**, never by line range. Any
> line number below is an approximate hint - verify it.

## 1. Problem

All paths under `rust/lib/color/src/` unless stated.

- **ls F12** - `Color::from_hex` (`lib.rs`) compiles an anchored `Regex` in a
  `lazy_static!` to validate `#rrggbb`, with four `.unwrap()`s. `brdgme_color`
  is the workspace's only `regex`/`lazy_static` dependent; the API has no
  runtime caller.
- **ls F13** - `Color::mono` (`lib.rs`) floors each channel before summing.
- **ls F14** - three divergent alias tables: `named()` (`lib.rs`),
  `NamedColor::from_str` (`palette.rs`), `resolve_named`
  (`rust/lib/markup/src/parser.rs`).
- **ls F15** - `palette.rs` spells every colour as a multi-line
  `Color { r, g, b }`; `LIGHT_PROTANOPIA`/`DARK_PROTANOPIA` are field-by-field
  clones of the deuteranopia palettes.
- **ls F16** - `themes()` doc claims Light/Dark derives from background
  lightness; the `THEMES` static hardcodes each category.
- **ls F17** - sRGB linearization triplicated, runtime 0.03928 vs two test
  copies at 0.04045.
- **ls F18** - `Color::hex` and `impl fmt::Display for Color` duplicate
  `#{:02x}{:02x}{:02x}`.

## 2. Why it's wrong

- **ls F12 is correct as written.** Verified live by `rg` over `rust/` for
  `from_hex`, `parse::<Color>`, `Color::from_str`, `named(`, `regex`,
  `lazy_static`: the only `from_hex` callers outside lib/color sit in
  `rust/lib/markup/src/transform.rs`'s `#[cfg(test)] mod tests`; `Color::from_str`
  is used only by lib/color's own tests; `named()` is private and reached only
  from `impl FromStr for Color`; no other manifest names `regex`/`lazy_static`.
  **No live caller. Deletion is safe.**
- **ls F13 is correct as written.** `mono` reads
  `self.r / 3 + self.g / 3 + self.b / 3 >= 128`; `rgb(128,128,128)` gives
  `42*3 = 126` -> black although the true mean is 128.
- **ls F14 is correct as written**, and D-39 disposes of it by deleting
  `named()`. **`NamedColor::from_str` is live** - `resolve_named` delegates to
  it. Do not touch either.
- **ls F15 is correct in substance, wrong in its numbers.** Live: 379 `Color {`
  literals, ~2,000 literal lines (not ~3,000); a `const fn rgb` rewrite lands
  the file near **~2,300 lines, not ~400**. Use these numbers.
- **ls F16/F17/F18 are correct as written.**

## 3. Required end state

### 3a. `lib.rs` - delete the parse API (F12, F14)

Delete these items **by name**, nothing else in the file:

- `Color::from_hex` (the whole `pub fn` inside `impl Color`)
- `impl FromStr for Color` (whole block)
- `fn named` (private module-level fn, plus its `//` comment)
- `use lazy_static::lazy_static;`, `use regex::Regex;`, `use std::str::FromStr;`
- tests `color_from_hex_works` and `color_from_str_named_works`. **Delete, do
  not port.** If `mod tests` empties, delete it and its `use super::*;`.

Keep `use std::fmt;`, `Color`, `mono`, `inv`, `hex`, `Style`, `Display` and all
`pub use` re-exports. `ColorError` stays exported (`NamedColor::from_str` still
returns it), so `error.rs` **survives unchanged**.

### 3b. `rust/lib/color/Cargo.toml` (F12)

Remove the `lazy_static` and `regex` lines; `serde`/`thiserror` stay. There is
**no `[workspace.dependencies]` table** in `rust/Cargo.toml` (verified), so no
second manifest edit. Do not hand-edit `Cargo.lock`.

### 3c. `rust/lib/markup/src/transform.rs` (fallout of 3a)

Its `#[cfg(test)] mod tests` calls `Color::from_hex("#dbdbdb").unwrap()` and
`Color::from_hex("#765381").unwrap()` (near lines 462/485 - approximate,
verify). Replace each with the literal, e.g. `Color { r: 0xdb, g: 0xdb, b: 0xdb }`.

### 3d. `lib.rs::Color::mono` (F13)

Mean in `u16`, rounded to nearest, compared `>= 128` - keep that boundary and
the white/black outputs:
`let avg = (u16::from(self.r) + u16::from(self.g) + u16::from(self.b) + 1) / 3;`

### 3e. `palette.rs` - const constructor (F15)

Add `pub const fn rgb(r: u8, g: u8, b: u8) -> Color` to `palette.rs` (and to
lib.rs's existing `pub use crate::palette::{...}` list); mechanically rewrite
every `Color { r: _, g: _, b: _ }` literal in the palette `static`s to
`rgb(_, _, _)`. Rewrite `LIGHT_PROTANOPIA`/`DARK_PROTANOPIA` to struct-update
syntax over their deuteranopia sources, keeping their doc comments. Scripted
transform, reviewable diff, **zero value changes**.

## 4. Non-goals

- Do not touch `NamedColor`, `NamedColor::from_str`, `resolve_named` or
  `ColorError` - all live. Do not add a replacement hex parser.
- Do not change any palette colour value, theme name, theme order or
  `ThemeCategory` assignment.
- Do not touch `css.rs`, `mix`, `soften`, `contrast*`, or the CVD gate tests
  beyond the F17 helper share.

## 5. Regression test cases

`palette.rs`'s existing `#[cfg(test)] mod tests` (`soften_exactness`,
`mix_exactness`, `gate_contrast_all_themes`, `gate_cvd_simulation`) must pass
unchanged - that is the F15 safety net.

Add to `lib.rs` (recreate `#[cfg(test)] mod tests` if 3a emptied it):

- `mono` boundary: `(128,128,128)` -> white; `(127,127,127)` -> black;
  `(0,0,0)` -> black; `(255,255,255)` -> white.
- `hex` equals `to_string` for one non-trivial colour (F18 guard).

No test may reference `from_hex`, `from_str`, or `named`.

## 6. Riders

| finding | file | one-line fix | test needed |
| --- | --- | --- | --- |
| ls F16 | `palette.rs`, doc comment on `fn themes` | reword the "assigned by each palette's actual `background` lightness" sentence to "categorised by background lightness, assigned manually at registration" | n |
| ls F17 | `palette.rs`, `fn srgb_channel_to_linear` vs test-local `rgb_to_lab::lin` and `tests::srgb_to_linear` | make both test-local copies call `super::srgb_channel_to_linear` and delete them; note the 0.03928 IEC breakpoint in its doc comment | n (gate tests cover it) |
| ls F18 | `lib.rs`, `Color::hex` | `pub fn hex(self) -> String { self.to_string() }`; `Display` keeps the sole format string | y (see section 5) |
