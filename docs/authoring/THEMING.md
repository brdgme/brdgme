# Theming and the Abstract Colour Palette

The palette contract between games, the markup pipeline, and themes.
Colours in game markup are abstract names; a theme is the thing that
resolves each name to a concrete value at the final render boundary.

Decided 2026-07-11 from a full colour audit of `rust/`, `brdgme-go/`, and
the legacy `brdg.me` repo (see Appendix B). Supersedes the palette details
in `docs/superpowers/specs/2026-07-05-26-theming-design.md` (which assumed
all 21 Material colours survive); the architectural direction there
(semantic colours end-to-end, `Palette` as data, CSS custom properties on
the web) is unchanged.

## The palette

A theme defines exactly 14 colour slots.

### Hues (9)

| Slot   | Default (light) value | Primary uses found in games                  |
| ------ | --------------------- | -------------------------------------------- |
| RED    | `#d32f2f` (211,47,47)   | dice, suits, corps, errors; player colour   |
| GREEN  | `#388e3c` (56,142,60)   | dice, suits, money; player colour           |
| BLUE   | `#1976d2` (25,118,210)  | dice, suits, corps; player colour           |
| YELLOW | `#fbc02d` (251,192,45)  | dice, suits, corps                          |
| PURPLE | `#7b1fa2` (123,31,162)  | dice, suits, corps; player colour           |
| CYAN   | `#0097a7` (0,151,167)   | dice, wilds, water; player colour           |
| PINK   | `#c2185b` (194,24,91)   | highlights, rainbow suits; player colour    |
| ORANGE | `#f57c00` (245,124,0)   | corps, rainbow suits; player colour         |
| BROWN  | `#5d4037` (93,64,55)    | goods, casino tiles; player colour          |

### Neutrals (4)

| Slot               | Default (light) value | Meaning                                      |
| ------------------ | --------------------- | -------------------------------------------- |
| FOREGROUND         | `#000000`             | default text; "black" game identities        |
| BACKGROUND         | `#ffffff`             | page/board background; default text bg       |
| BACKGROUND_SHADE_1 | `#dcdcdc` (220)       | subtle surface step (checkerboards, outlines)|
| BACKGROUND_SHADE_2 | `#bebebe` (190)       | stronger surface step                        |

GREY is a tenth hue-like slot for de-emphasised text and neutral game
identities (empty cells, grid lines, folded/out states). Default (light):
`#616161` (97,97,97). It is the single most used colour across all games
and must remain readable everywhere text appears (see contrast rules).

The shades are an ordered ramp stepping from BACKGROUND toward
FOREGROUND: SHADE_1 is subtle, SHADE_2 is stronger. The naming is
deliberately direction-neutral - on light themes the ramp darkens, on
dark themes it lightens. As a starting point, mix BACKGROUND toward
FOREGROUND by roughly 10-15% for SHADE_1 and 20-30% for SHADE_2, then
adjust to taste and validate contrast.

### The `contrast` transform

`contrast` replaces the old `inv()`/`mono()` pair (and the Go
`Inv`/`Mono` markup flags). It resolves to whichever of FOREGROUND or
BACKGROUND is more readable against the already-theme-resolved colour it
is applied to. Every historical use of `inv`/`mono` was the composed
"readable text on this background" idiom, so `contrast` is the only
transform themes need to support. Bare `inv` has no users and is not part
of the contract.

Because `contrast` is evaluated after theme resolution, it stays correct
per theme with no extra authoring work.

### Player palette (8)

Player colours are drawn from the hues, in this order:

    GREEN, RED, BLUE, ORANGE, PURPLE, BROWN, CYAN, PINK

Games never reference player colours directly - they emit `Player(n)`
markup nodes and the pipeline resolves them through the theme. GREY is
deliberately excluded (too heavily used for de-emphasised text);
overlap with the other game hues is accepted, as it always has been.

### Parse aliases (backward compatibility)

The markup parser accepts these legacy names and maps them to slots:

| Alias     | Resolves to |
| --------- | ----------- |
| `magenta` | PURPLE      |
| `amber`   | ORANGE      |
| `black`   | FOREGROUND  |
| `white`   | BACKGROUND  |

Aliases exist for old stored markup and the legacy `brdg.me` 8-colour
set (`black, red, green, yellow, blue, magenta, cyan, gray`), all of
which map onto the palette with no loss.

Separately, `brdgme-go` games emit concrete `{{fg rgb(r,g,b)}}` triples
even for named colours (`brdgme-go/render/color.go:43`). Because those
values are byte-identical to the old Rust constants, the parser
reverse-maps the known legacy triples to palette slots at parse time -
this themes Go game output and historical markup stored in the DB
without any Go-side change. The reverse map covers the 21 old constants
plus the 11 bespoke acquire-1/lords-of-vegas-1 values (mapped to their
Appendix A slots); unknown triples fall back to FOREGROUND with a
warning. `rgb(...)` is purely a parse-time compatibility form - it never
survives into the AST, and games cannot emit it (see the rules below).

## Contrast requirements (themes MUST validate these)

Games freely place any hue, GREY, or FOREGROUND text on BACKGROUND and
on both shades - Acquire, for example, renders GREY text inside
SHADE_1/SHADE_2 checkerboard cells. A theme is only valid if:

- FOREGROUND, GREY, and every hue used as text reach at least 3:1
  contrast against BACKGROUND, BACKGROUND_SHADE_1, and
  BACKGROUND_SHADE_2. Aim for 4.5:1 (WCAG AA body text) where the
  palette allows it; the light default's weakest pair (GREY on
  SHADE_2) sits at roughly 3.3:1, so 3:1 is the hard floor.
- `contrast` output (FOREGROUND or BACKGROUND) reaches at least 4.5:1
  against every hue and both shades, since it exists specifically to
  carry text on coloured backgrounds.
- All 8 player colours are pairwise distinguishable, and each is
  distinguishable from GREY and FOREGROUND. Colourblind-friendly themes
  should verify this under deuteranopia/protanopia simulation - the
  palette was kept small precisely to make this achievable.
- Hue pairs that games rely on being distinct stay distinct: notably
  RED/ORANGE/YELLOW (Acquire corps), BLUE/CYAN (rainbow suits, water vs
  dice), GREY/FOREGROUND (stone vs ore in Seven Wonders, onyx vs
  diamond in Splendor), and GREY/BROWN.

A contrast-check test over these pairs should gate adding any theme
(per the theming design spec).

## Worked example: Dracula

Dracula's published palette covers 11 of the 14 slots natively; BLUE and
BROWN are derived tones (Dracula has no blue or brown accent) and should
be tuned/validated rather than taken as gospel.

| Slot               | Value     | Source                        |
| ------------------ | --------- | ----------------------------- |
| RED                | `#ff5555` | Red                           |
| GREEN              | `#50fa7b` | Green                         |
| BLUE               | `#6987f5` | derived (between cyan/purple) |
| YELLOW             | `#f1fa8c` | Yellow                        |
| PURPLE             | `#bd93f9` | Purple                        |
| CYAN               | `#8be9fd` | Cyan                          |
| PINK               | `#ff79c6` | Pink                          |
| ORANGE             | `#ffb86c` | Orange                        |
| BROWN              | `#b8825e` | derived (desaturated orange)  |
| GREY               | `#6272a4` | Comment                       |
| FOREGROUND         | `#f8f8f2` | Foreground                    |
| BACKGROUND         | `#282a36` | Background                    |
| BACKGROUND_SHADE_1 | `#343746` | derived (bg -> fg step)       |
| BACKGROUND_SHADE_2 | `#44475a` | Current Line/Selection        |

Note GREY takes Comment, so BLUE cannot also use it - the two must stay
distinct (see contrast requirements). GREY at Comment likely fails 3:1
against SHADE_2 here; a Dracula theme may need to lighten GREY or keep
SHADE_2 closer to BACKGROUND. This is exactly the kind of trade-off the
contrast test exists to catch.

## Rules for game authors

- Concrete colours are not expressible from game code, by construction:
  the markup AST has no RGB variant and no conversion from
  `brdgme_color::Color`. Games only name palette slots and transforms.
  The `Color` struct exists solely as the value a theme `Palette`
  resolves slots to; game crates have no reason to import it.
- The `rgb(r,g,b)` markup syntax is parse-time legacy compatibility
  only (Go output, stored logs). It cannot be emitted by games and must
  not appear in new code, prompts, or documentation examples
  (`bot/system_prompt.md` needs updating to the named syntax).
- Use `contrast` for text on coloured backgrounds. Never hand-pick
  black/white text.
- Use `Player(n)` nodes for anything player-owned. Never hardcode
  per-player hues (the legacy `hive` port must fix this).
- Board surfaces and checkerboards use BACKGROUND and the two shades,
  not literal greys.
- "Black" and "white" game identities (black dice, black corp, white
  suit) use FOREGROUND and GREY respectively - see Appendix A for the
  reasoning and precedents.

## Appendix A: disposition of the old 21-constant palette

| Old constant | Disposition                                              |
| ------------ | -------------------------------------------------------- |
| RED          | kept                                                     |
| PINK         | kept (previously unused; now also a player colour)       |
| PURPLE       | kept                                                     |
| DEEP_PURPLE  | removed (never used)                                     |
| INDIGO       | removed (never used)                                     |
| BLUE         | kept                                                     |
| LIGHT_BLUE   | removed (never used)                                     |
| CYAN         | kept (replaces BLUE_GREY as a player colour)             |
| TEAL         | removed (never used)                                     |
| GREEN        | kept                                                     |
| LIGHT_GREEN  | removed (never used)                                     |
| LIME         | removed (never used)                                     |
| YELLOW       | kept                                                     |
| AMBER        | renamed to ORANGE (`#f57c00`-style value; alias kept)    |
| ORANGE       | name reused by the AMBER rename (old value was unused)   |
| DEEP_ORANGE  | removed; its one user (Acquire corps) moves to ORANGE    |
| BROWN        | kept                                                     |
| GREY         | kept                                                     |
| BLUE_GREY    | removed (only ever a player colour; slot goes to CYAN)   |
| WHITE        | removed; semantic uses become BACKGROUND / `contrast`    |
| BLACK        | removed; semantic uses become FOREGROUND / `contrast`    |

BLACK-as-identity (Acquire's black corp, Greed/Liar's Dice dice, Jaipur
goods, Sushi Go cards, Splendor onyx, Seven Wonders ore) maps to
FOREGROUND: on light themes this is identical to today, and on dark
themes a "black" item must become light to be visible anyway - FOREGROUND
is the colour a hand-made dark theme would pick. Distinctness survives
because the pairs that matter (black vs grey) become FOREGROUND vs GREY,
which every valid theme keeps distinct. The residual quirk - prose like
"2 black dice" rendering in a light colour on dark themes - is inherent
to theming (every terminal colour scheme has it) and accepted.

Bespoke colour migrations (the only two games with out-of-palette
colours anywhere in rust/, brdgme-go/, or brdg.me):

- **acquire-1** (`src/render.rs`): checkerboard `220`/`190` greys ->
  BACKGROUND_SHADE_1 / BACKGROUND_SHADE_2; unincorporated `100` grey ->
  GREY bg; unavailable-tile `80` grey text -> GREY; available-tile pink
  bg `248,187,208` -> PINK (or a shade, at porting discretion); corp
  DEEP_ORANGE -> ORANGE; corp BLACK -> FOREGROUND; `.inv().mono()` ->
  `contrast`.
- **lords-of-vegas-1** (`src/casino.rs`, `src/render.rs`): Albion ->
  PURPLE, Sphinx -> ORANGE or BROWN, Vega -> GREEN, Tivoli -> GREY,
  Pioneer -> BROWN, unbuilt tile `200` grey -> BACKGROUND_SHADE_2;
  `.inv().mono()` -> `contrast`. The casino colours are tile backgrounds
  under player-coloured content; if plain hues clash in practice, the
  theme-level answer is bg-muted variants, not bespoke RGB.
- **cathedral_1** (brdgme-go, `render.go:149-154`): the one `Mono, Inv`
  flag use -> `contrast` at porting time.

Other migration touchpoints: `api/src/db/color.rs` mirrors the player
palette as a DB enum for signup colour preferences - the AMBER->ORANGE
rename, BLUE_GREY removal, and PINK addition need a data migration with
old names mapped via the aliases.

## Appendix B: colour audit summary (2026-07-11)

Scope: all 16 games in `rust/game/`, `rust/lib/` + `api` + `bot` +
`web`, all 17 games + libs in `brdgme-go/`, and all ~26 games plus
render/server in the legacy `~/Development/brdg.me` repo (including the
never-ported games: alhambra, chess, hive, red7, seven_wonders,
starship_catan).

Findings that shaped the palette:

- Of the 21 constants, only 13 were used in production anywhere: RED,
  GREEN, BLUE, YELLOW, PURPLE, CYAN, GREY, BLACK, WHITE, AMBER, BROWN,
  BLUE_GREY (player palette only), DEEP_ORANGE (Acquire corps only).
  PINK, DEEP_PURPLE, INDIGO, LIGHT_BLUE, TEAL, LIGHT_GREEN, LIME, and
  ORANGE appeared only in tests and dead CSS.
- Bespoke RGB exists in exactly two games (acquire-1, lords-of-vegas-1;
  11 values total, listed in Appendix A). brdgme-go and every legacy
  brdg.me game stay entirely within their named sets.
- `inv`/`mono` are only ever used composed as the contrast idiom
  (acquire-1, lords-of-vegas-1, cathedral_1). Bare `inv` has no users.
- Player colouring is already abstract in all game code: games emit
  `Player(n)` nodes; none call `player_color()` directly.
- The legacy brdg.me render layer supports only 8 colours; all legacy
  games fit the new palette via the aliases. red7 is the stress test
  (7 rainbow suits, currently lossily collapsed) and is fully served by
  the 9-hue palette: red, ORANGE, YELLOW, GREEN, CYAN (blue suit), BLUE
  (indigo suit), PINK or PURPLE (violet suit).
- Concrete RGB currently gets baked in at `lib/markup`'s `transform()`
  and again in `html.rs`/`ansi.rs`, plus two out-of-band paths: the web
  sidebar player names (`web/src/game/server_fns.rs`,
  `web/src/components/game.rs`) and the bot's LLM prompt (hex strings,
  cosmetic only). The `.brdgme-*` classes in `web/style/main.scss` are
  dead code duplicating the constants, and `main.scss` carries ~20
  hardcoded chrome colours - both are the web-side theming surface per
  the design spec.
