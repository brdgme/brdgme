# 26: Theming - Semantic Colours Implementation Plan

**Created:** 2026-07-13
**Spec:** `docs/superpowers/specs/2026-07-05-26-theming-design.md`
**Palette contract:** `docs/authoring/THEMING.md` (12 slots + `soften` +
`contrast`; revised 2026-07-13)

Execution model: serial orchestrator subagents, one per phase below,
one at a time. Each orchestrator spawns Sonnet 5 implementation
subagents, reviews the result, and verifies (build + tests) before
finishing. The top-level session only reads files, updates this plan,
and dispatches orchestrators.

## Migration strategy: additive then removal

To keep the workspace green throughout the serial migration:

1. Phase 1 ADDS `ColType::Named(...)` (with optional soften) alongside
   the existing `ColType::RGB`, adds `NamedColor`/`Palette`/`soften` to
   `brdgme-color`, teaches parser/transform/renderers both forms, and
   resolves Named against the built-in LIGHT palette at the existing
   render boundaries. Nothing else changes; all games still compile.
2. Phases 2-24 migrate one game per orchestrator to Named markup.
3. Phase 25 removes `ColType::RGB` from the public AST (keeping the
   parse-time `rgb(...)` reverse-map), deletes `From<brdgme_color::Color>
   for Col`, deletes the dead colour constants, renames AMBER->ORANGE,
   and fixes remaining lib/api/bot/web consumers.
4. Phase 26 does DB + bot-prompt migration; Phase 27 web theming.

## Key contracts (orchestrators must follow, not re-decide)

- `NamedColor`: Red, Green, Blue, Yellow, Purple, Cyan, Pink, Orange,
  Brown, Grey, Foreground, Background (12).
- Player palette order: GREEN, RED, BLUE, ORANGE, PURPLE, BROWN, CYAN,
  PINK.
- `soften(color, pct)`: theme-resolve `color`, then in HSL set
  `l' = l + (l_bg - l) * pct/100` (hue/sat kept), pct integer 1-99.
  Markup syntax: `{{bg soften(pink, 75)}}` (also valid in `fg` grammar
  positions but games must only use it for backgrounds).
- `contrast`: resolves to FOREGROUND or BACKGROUND, whichever has the
  higher WCAG contrast ratio vs the resolved colour it is applied to.
  Parser normalises legacy composed `inv`+`mono` to `contrast`.
- Parse aliases: magenta->PURPLE, amber->ORANGE, black->FOREGROUND,
  white->BACKGROUND. `rgb(r,g,b)` accepted at parse time only,
  reverse-mapped: 21 legacy constants -> slots; acquire/lords-of-vegas
  bespoke values -> per THEMING.md Appendix A (220/190 greys ->
  soften(FOREGROUND, 86)/soften(FOREGROUND, 75), pink 248,187,208 ->
  soften(PINK, 75), 200 grey -> soften(FOREGROUND, 78), 100 grey ->
  GREY, 80 grey -> GREY); unknown triples -> FOREGROUND + warning.
- LIGHT palette values: the current Material constants per THEMING.md
  (ORANGE = old AMBER-slot value `#f57c00`... note: THEMING.md defines
  ORANGE as 245,124,0 which is the existing `ORANGE` constant; old
  AMBER 255,160,0 maps to the ORANGE slot as an alias).

## Phases (serial)

| # | Orchestrator scope | Verify |
|---|---|---|
| 1 | Core: brdgme-color (NamedColor, Palette, LIGHT+DARK, soften, contrast) + brdgme-markup (Named AST variant, parser incl. aliases/rgb reverse-map/soften/contrast, transform takes palette, html/ansi resolve via palette) - additive, RGB kept | workspace `cargo check` + lib tests incl. new unit tests for soften exactness (#dbdbdb/#bfbfbf/#f7bed4) |
| 2 | tic-tac-toe-2 (pilot, simplest) | `cargo test -p`, review |
| 3-22 | one orchestrator per remaining named-colour game: age-of-war-2, battleship-2, category-5-2, cathedral-2, farkle-2, for-sale-2, greed-2, jaipur-2, liars-dice-2, lost-cities-1, lost-cities-2, love-letter-2, modern-art-2, no-thanks-2, roll-through-the-ages-2, splendor-2, sushi-go-2, sushizock-2, texas-holdem-2, zombie-dice-2 | per-game `cargo test -p`, review |
| 23 | acquire-1 (bespoke: checkerboard/pink/corp remaps per THEMING.md Appendix A) | tests + eyes-on render diff |
| 24 | lords-of-vegas-1 (bespoke casino remaps per Appendix A) | tests + eyes-on render diff |
| 25 | Removal: drop ColType::RGB from AST, drop From<Color> for Col, delete dead constants, AMBER->ORANGE rename, fix lib/cmd, lib/game, bot, web/db.rs consumers to palette-based rendering (LIGHT hardcoded where theme not yet plumbed) | workspace build + all tests |
| 26 | api/bot: db color enum + data migration (Amber->Orange, BlueGrey removed, Cyan/Pink added), bot/system_prompt.md named syntax | api tests, migration review |
| 27 | web theming: Palette -> CSS custom properties at SSR, semantic classes renderer for web output, theme switcher with cookie + profile sync, contrast gate test. Themes (user-confirmed 2026-07-13): "brdgme light" (current LIGHT), "brdgme dark" (dark counterpart of LIGHT, tuned to pass the contrast gate), Dracula (per THEMING.md worked example, BLUE/BROWN derived, GREY tuned to pass contrast). Default selection is "system theme": prefers-color-scheme picks brdgme light/dark when the user has no explicit choice; Dracula is opt-in. Monokai deferred. | web build + tests, contrast test green |
| 28 | Final verification: full workspace build, tests, clippy, grep for stray RGB emission, golden-output sanity | all green |

Email theming (mrml) is intentionally deferred: it depends on the #22
email template work; not blocked by anything here.

## Decisions made without user input (for post-completion review)

- D1: `soften` uses HSL-lightness mixing (not RGB mix) - near-exact
  reproduction of legacy values; slightly more resolver complexity.
- D2: The two BACKGROUND_SHADE slots are REMOVED from the palette
  contract in favour of `soften(FOREGROUND, pct)` (12-slot contract).
  Themes lose direct hand-tuning of surfaces; contrast test enumerates
  in-use soften expressions instead.
- D3: Acquire available-tile pink = `soften(PINK, 75)` (#f7bed4), a
  1-3/255 drift from the historical Material Pink 100 #f8bbd0.
- D4: Additive-then-removal sequencing (RGB variant survives until
  phase 25) to keep the workspace compiling during serial per-game
  migration.
- D5: `soften` pct is a free integer 1-99 with documented preferred
  steps, rather than a hard-quantised enum.
- D6: Email theming deferred (depends on #22 email templates).
- D7: Web renderer gains a semantic-class mode (option (b) in the
  spec); inline-style mode kept for email/legacy paths.
- D8: `.mono().inv()` sites replaced with `.contrast()`, which flips
  some text colours vs legacy output by WCAG ratio: cathedral-2 and
  lords-of-vegas-1 text on GREEN #388e3c is now black (was white);
  lords-of-vegas Sphinx label on ORANGE #f57c00 is black. Accepted as
  intended `contrast` semantics rather than special-casing.
- D9: splendor-2 Diamond/Onyx colours deliberately swapped (Diamond
  BLACK -> Grey, Onyx GREY -> Foreground) per THEMING.md onyx/diamond
  guidance - a semantic remap, not 1:1.
- D10: Theme tuning to pass the contrast gate: DARK brown #a1887f ->
  #8f533d; DRACULA foreground #f8f8f2 -> #f8f8f8, blue #6987f5 ->
  #708cf5, brown #b8825e -> #bc8967, grey #6272a4 -> #b8bfd6. LIGHT
  untouched. Gate scoping: text-on-background checked only for the 7
  hues actually used as bare text (YELLOW/ORANGE excluded - they would
  fail even on LIGHT).
- D11: web main.scss's ~20 hardcoded chrome colours NOT rethemed; only
  body bg/fg wired to palette CSS vars. Follow-up work.
- D12: DB migrations 006_color_palette.sql and 007_user_theme.sql
  written but NOT run against any database (none available in env);
  sqlx::test DB tests likewise unrun. brdgme-color FromStr legacy
  colour aliases kept because stored text data may predate 006.
- D13: legacy `{{c name}}` parser and rgb() reverse-map fold removed Go
  constants onto slots (deeppurple/indigo/lightblue->Purple/Blue/Blue,
  teal/bluegrey->Cyan, lightgreen/lime->Green, deeporange->Orange);
  unknown names/triples -> Foreground (triples also warn via eprintln).
- D14: bare Mono/Inv transform chains fold into `contrast` in the web
  semantic-class mode (no real users found in audit).
- D15: Bug found (pre-existing, NOT fixed here): lords-of-vegas-1
  card.rs shuffled_deck iterates HashMap TILES.keys() before shuffling,
  so Game::start is not deterministic across processes for a fixed
  seed. Needs a separate fix.

## Progress log

- [x] Phase 1 core libs (2026-07-13): NamedColor/Palette/LIGHT/DARK/
  soften/contrast in brdgme-color; ColType::Named + soften/contrast
  grammar + rgb reverse-map in brdgme-markup; transform_with_palette
  added, transform() wraps LIGHT. Tests 6+24 pass; workspace check
  clean except pre-existing web failure (E0433 brdgme_game, unrelated).
  Extra decisions: resolution at transform boundary (TNode stays
  concrete); removed-constant rgb mappings (deep_purple/indigo->Purple/
  Blue, light_blue->Blue, teal->Cyan, light_green/lime->Green,
  blue_grey->Cyan); unknown rgb triples warn via eprintln; unknown bare
  names fail parse; DARK palette placeholder pending phase 27 contrast
  gate; legacy `{{c name}}` parser still emits RGB constants (phase 25
  must address). Migration surface for games: Col::from(NamedColor),
  ColType::Named { color, soften }, .contrast().
- [x] Phase 2 tic-tac-toe-2 (2026-07-13): render.rs BLUE/GREY ->
  NamedColor::Blue/Grey via Col::from(NamedColor); exact-render markup
  test in lib.rs updated ({{fg rgb(...)}} -> {{fg grey}}/{{fg blue}}).
  26 tests pass; workspace check clean (excl. web); LIGHT-palette HTML
  render byte-identical before/after. Recipe for phases 3-22: (1) grep
  game crate for `use brdgme_color::` constants; (2) replace each
  CONST.into() with NamedColor::X.into() (black->Foreground,
  white->Background, amber->Orange, magenta->Purple); (3) update any
  tests asserting `{{fg rgb(...)}}` strings to named syntax; (4) crates
  keep the brdgme_color dep (NamedColor lives there); (5) verify with
  cargo test -p + a before/after html(transform(render)) dump on LIGHT.
- [x] Phase 3 age-of-war-2 (2026-07-13): castle.rs BLUE/PURPLE/GREEN/
  RED/YELLOW/GREY/BLACK -> NamedColor (BLACK->Foreground);
  Die::colour/Clan::colour signatures changed Color -> NamedColor
  (callers unchanged, all go through .into()); render.rs GREY/GREEN ->
  NamedColor::Grey/Green. No rgb() test assertions existed. 26+1 tests
  pass; workspace check clean (excl. web); LIGHT-palette HTML render
  byte-identical before/after (seed 42, 2-6 players). Gotcha: crates
  may expose `pub fn colour() -> Color` helpers - retype them to
  NamedColor rather than converting at each call site.
- [x] Phase 4 battleship-2 (2026-07-13): render.rs CYAN/BLUE/GREY/RED/
  YELLOW -> NamedColor 1:1; only colour-bearing file, no rgb() test
  assertions, no Color-returning helpers, no inv/mono composition.
  37+1 tests pass; workspace check clean (excl. web); LIGHT-palette
  HTML render byte-identical before/after (temp dump test on pub +
  both player states with placed ships, hit and miss cells; deleted
  after). No new gotchas.
- [x] Phase 5 category-5-2 (2026-07-13): lib.rs Card::color() retyped
  Color -> NamedColor (Purple/Red/Yellow/Cyan/Grey); render.rs legend
  `order` array retyped to NamedColor, `col.into()` -> `(*col).into()`
  (Col: From<NamedColor>, not From<&NamedColor>); test_card_colors
  assertions updated. 21+1 tests pass; workspace check clean (excl.
  web); LIGHT-palette HTML render byte-identical before/after
  (Game::start seed 42, pub + 4 player states; temp dump deleted).
  Gotcha: iterating a NamedColor array by reference needs an explicit
  deref before .into().
- [x] Phase 6 cathedral-2 (2026-07-13): render.rs GREY (3 uses) ->
  NamedColor::Grey; `player_col(tile.player).mono().inv()` ->
  `.contrast()`. Only render.rs changed; no rgb() test assertions, no
  Color-returning helpers (player_col returns Col from player index).
  25+1 tests pass; workspace check clean (excl. web). LIGHT-palette
  HTML render NOT byte-identical: text on the green player background
  (#388e3c) flipped white -> black, because WCAG contrast favours
  black there (~5.0 vs ~4.2 for white) while the old mono().inv()
  picked white. Accepted as intended `.contrast()` semantics per
  THEMING.md rather than special-casing; flagged for user review.
  All other output (incl. grey spans) byte-identical (seed 42, pub +
  2 player states; temp dump deleted).
- [x] Phase 7 farkle-2 (2026-07-13): lib.rs `die_color(Die) -> Color`
  retyped to `-> NamedColor` (CYAN/GREEN/RED/BLUE/YELLOW/PURPLE/GREY ->
  NamedColor 1:1); sole caller render.rs `die_color(d).into()` compiled
  unchanged. No inv/mono usage, no rgb() test assertions. 13 lib tests
  + suite pass; workspace check clean (excl. web). LIGHT-palette HTML
  render byte-identical before/after (seed 42, pub + 2 player states;
  temp dump deleted). No new gotchas.
- [x] Phase 8 for-sale-2 (2026-07-13): render.rs GREEN/BLUE/GREY ->
  NamedColor 1:1; only colour-bearing file (grep incl. bin/command.rs
  clean), no rgb() test assertions, no Color-returning helpers, no
  inv/mono usage. 16+1 tests pass; workspace check clean (excl. web);
  LIGHT-palette HTML render byte-identical before/after (Game::start
  3 players fixed seed, pub + 3 player states; temp dump deleted).
  No new gotchas.
- [x] Phase 9 greed-2 (2026-07-13): lib.rs `Die::color` retyped
  `Color -> NamedColor` (GREY/YELLOW/RED/GREEN/CYAN 1:1, BLACK ->
  Foreground for the black E die per THEMING.md); sole caller
  render.rs `d.color().into()` compiled unchanged. No inv/mono usage,
  no rgb() test assertions. 14 lib tests + contract test pass;
  workspace check clean (excl. web); LIGHT-palette HTML render
  byte-identical before/after (Game::start 2 players seed 42, pub
  state; temp dump deleted). No new gotchas.
- [x] Phase 10 jaipur-2 (2026-07-13): lib.rs `Good::color` retyped
  `Color -> NamedColor` (RED/YELLOW/GREY/PURPLE/GREEN/BLUE 1:1,
  BLACK -> Foreground for Camel per THEMING.md); render.rs two
  `N::Fg(GREY.into(), ...)` -> `NamedColor::Grey.into()`. No
  inv/mono usage, no rgb() test assertions. 60 lib tests + contract
  test pass; workspace check clean (excl. web); LIGHT-palette HTML
  render byte-identical before/after (Game::start 2 players seed 42,
  pub + both player states; temp dump deleted). No new gotchas.
- [x] Phase 11 liars-dice-2 (2026-07-13): render.rs only:
  GREY/CYAN/RED -> NamedColor 1:1, BLACK -> Foreground for non-1 dice
  per THEMING.md; import replaced with `use brdgme_color::NamedColor`.
  No Color-returning helpers, no inv/mono usage, no rgb() test
  assertions (grep clean). 4 unit + 1 contract test pass; workspace
  check clean (excl. web); LIGHT-palette HTML render byte-identical
  before/after (Game::start(4, 42) pub + player states, plus targeted
  render_die/render_bid/reveal_table samples covering cyan/black/red/
  grey paths; temp dumps deleted). No new gotchas.
- [x] Phase 12 lost-cities-1 (2026-07-13): card.rs `Expedition::color`
  retyped `Color -> NamedColor` (RED/GREEN/BLUE 1:1, White -> Grey
  matching prior GREY constant, Yellow AMBER -> Orange per THEMING.md);
  render.rs `use brdgme_color::GREY` -> `NamedColor`, five `GREY.into()`
  -> `NamedColor::Grey.into()`. No inv/mono usage, no rgb() test
  assertions. 7 unit + contract tests pass; workspace check clean
  (excl. web). LIGHT-palette HTML render before/after (Game::start(2, 42)
  pub + both player states; temp dump deleted): identical except amber
  spans #ffa000 -> #f57c00 (LIGHT orange), the expected AMBER -> Orange
  remap; all grey/red/green/blue spans byte-identical. No new gotchas.
- [x] Phase 13 lost-cities-2 (2026-07-13): card.rs `Expedition::color`
  retyped `Color -> NamedColor` (RED/GREEN/BLUE 1:1, White -> Grey
  matching prior GREY, Yellow AMBER -> Orange per THEMING.md);
  render.rs `use brdgme_color::GREY` -> `NamedColor`, six `GREY.into()`
  -> `NamedColor::Grey.into()`. No inv/mono usage, no rgb() test
  assertions. 8 lib tests + contract test pass; workspace check clean
  (excl. web). LIGHT-palette HTML render before/after (Game::start(2, 42)
  pub + both player states; temp dump deleted): identical except amber
  spans #ffa000 -> #f57c00, the expected AMBER -> Orange remap; all
  other spans byte-identical. Mirrors lost-cities-1. No new gotchas.
- [x] Phase 14 love-letter-2 (2026-07-13): card.rs `Card::color`
  retyped `Color -> NamedColor` (GREY/CYAN/GREEN/PURPLE/BLUE/RED/
  YELLOW 1:1, BLACK -> Foreground for Handmaid per THEMING.md);
  render.rs GREY.into() x2 -> NamedColor::Grey.into(), BLACK ->
  Foreground, GREEN -> Green; imports updated. No inv/mono usage,
  no rgb() test assertions (grep clean). 20 lib tests + contract
  test pass; workspace check clean (excl. web); LIGHT-palette HTML
  render byte-identical before/after (Game::start(4, 42) pub + 4
  player states; temp dump deleted). No new gotchas.
- [x] Phase 15 modern-art-2 (2026-07-13): card.rs `Suit::color`
  retyped `Color -> NamedColor` (YELLOW/GREEN/RED/BLUE/BROWN 1:1);
  render.rs `GREEN as MONEY_GREEN`/`GREY` imports -> `NamedColor`,
  `NamedColor::Green.into()` for money, `NamedColor::Grey.into()` for
  empty purchases. No inv/mono usage, no rgb() test assertions (grep
  incl. tests clean). 12 unit + 1 contract test pass; workspace check
  clean (excl. web); LIGHT-palette HTML render byte-identical
  before/after (Game::start(4, 42) pub + 4 player states; temp dump
  deleted). No new gotchas.
- [x] Phase 16 no-thanks-2 (2026-07-13): render.rs only:
  BLUE/GREEN/PURPLE/GREY -> NamedColor 1:1 (card/chips/points/
  "no cards"); import replaced with `use brdgme_color::NamedColor`.
  No Color-returning helpers, no inv/mono usage, no rgb() test
  assertions (grep incl. bin/tests clean). 15 lib tests + contract
  test pass; workspace check clean (excl. web); LIGHT-palette HTML
  render byte-identical before/after (Game::start(3, 42) pub + player
  states via temp render_dump test; deleted after). No new gotchas.
- [x] Phase 17 roll-through-the-ages-2 (2026-07-13): good.rs
  `Good::colour` retyped `Color -> NamedColor` (PURPLE/GREY/RED/BLUE/
  YELLOW 1:1); dice.rs `dice_value_colour` retyped `Option<Color> ->
  Option<NamedColor>` (GREEN/PURPLE/RED/CYAN/YELLOW 1:1); render.rs
  GREY/GREEN/BLUE/RED -> NamedColor 1:1, and render_dice's rgb-tuple
  run comparison `(c.r, c.g, c.b)` collapsed to plain
  `colour != run_colour` (NamedColor is PartialEq/Eq). No BLACK/AMBER
  cases, no inv/mono usage, no rgb() test assertions. 114 lib tests +
  contract test pass; workspace check clean (excl. web); LIGHT-palette
  HTML render byte-identical before/after (Game::start(3, 42) pub + 3
  player states via temp render_dump test; deleted after). No new
  gotchas.
- [x] Phase 18 splendor-2 (2026-07-13): lib.rs `resource_color` retyped
  `Color -> NamedColor` (BLUE/GREEN/RED/YELLOW/PURPLE 1:1; Diamond
  BLACK -> Grey and Onyx GREY -> Foreground per THEMING.md's onyx/
  diamond guidance - a deliberate semantic swap, not 1:1); callers
  drop `(&...)` reference before `.into()`. render.rs GREY/GREEN/
  YELLOW/CYAN -> NamedColor 1:1. No inv/mono usage, no rgb() test
  assertions. 52 lib tests + contract test pass; workspace check
  clean (excl. web). LIGHT-palette HTML render before/after
  (Game::start(3, 42) pub + 3 player states; temp dump deleted):
  identical except the expected Diamond #000000 -> #616161 and Onyx
  #616161 -> #000000 swaps; all other spans byte-identical.
  No new gotchas.
- [x] Phase 19 sushi-go-2 (2026-07-13): lib.rs `Card::color` retyped
  `Color -> NamedColor` (GREY/PURPLE/YELLOW/RED/CYAN/BLUE/GREEN 1:1;
  Chopsticks BLACK -> Foreground per THEMING.md Appendix A); other
  lib.rs GREY/RED/BLUE `.into()` sites and render.rs GREY sites
  -> NamedColor 1:1. No inv/mono usage, no rgb() test assertions
  (grep incl. bin/tests clean). 39 lib tests + contract test pass;
  workspace check clean (excl. web); LIGHT-palette HTML render
  byte-identical before/after (Game::start(3, 42) pub + 3 player
  states via temp render_dump test; deleted after; BLACK ==
  LIGHT Foreground #000000, so no diff expected or seen). No new
  gotchas.
- [x] Phase 20 sushizock-2 (2026-07-13): render.rs only:
  BLUE/RED/GREY -> NamedColor::Blue/Red/Grey 1:1 at all 7 `.into()`
  sites; no Color-returning helpers, no BLACK/AMBER, no inv/mono, no
  rgb() test assertions. 37 lib tests + contract test pass; workspace
  check clean (excl. web); LIGHT-palette HTML render byte-identical
  before/after (Game::start(3), seed 42, pub + 3 player states via
  temp render_dump test; deleted after). Gotcha reminder (not new):
  run `cargo fmt` after edits - a Grey substitution pushed one line
  over width and needed reformatting.
- [x] Phase 21 texas-holdem-2 (2026-07-13): card.rs `Suit::color`
  retyped `Color -> NamedColor` (Clubs/Spades BLACK -> Foreground per
  THEMING.md card-suit guidance, Diamonds/Hearts RED -> Red 1:1);
  render.rs GREEN-as-MONEY_GREEN -> NamedColor::Green, two GREY sites
  -> NamedColor::Grey; imports collapsed to `use
  brdgme_color::NamedColor`. No inv/mono usage, no rgb() test
  assertions. 43 lib tests + contract test pass; workspace check
  clean (excl. web); cargo fmt clean (one Grey line rewrapped);
  LIGHT-palette HTML render byte-identical before/after
  (Game::start(3, 42) pub + 3 player states via temp render_dump
  test; deleted after; BLACK == LIGHT Foreground so no diff
  expected). No new gotchas.
- [x] Phase 22 zombie-dice-2 (2026-07-13): lib.rs `Colour::to_color`
  retyped `Color -> NamedColor` (GREEN/YELLOW/RED -> Green/Yellow/Red
  1:1; callers unchanged via `.into()`); render.rs one GREY site ->
  NamedColor::Grey. No BLACK/AMBER, no inv/mono, no rgb() test
  assertions. 24 lib tests + contract test pass; workspace check
  clean (excl. web); cargo fmt --check clean; LIGHT-palette HTML
  render byte-identical before/after (Game::start(3, 42) pub + 3
  player states via temp render_dump test; before via git stash of
  the crate only; deleted after). No new gotchas. Named-colour game
  migrations complete; next up Phase 23 acquire-1.
- [x] Phase 23 acquire-1 (2026-07-13): first bespoke-colour + `soften`
  game. corp.rs `Corp::color` retyped `Color -> NamedColor` (PURPLE/
  GREEN/YELLOW/BLUE/RED 1:1, DEEP_ORANGE -> Orange, BLACK ->
  Foreground). render.rs: 5 bespoke statics deleted; checkerboard ->
  Named Foreground soften 86/75 (Col built inline in `empty_color`,
  now returning Col); unincorporated bg + unavailable-tile text ->
  NamedColor::Grey; available-tile bg -> Named Pink soften 75 via
  `available_loc_bg()` helper; all `.inv().mono()` sites ->
  `.contrast()`; `tile_background` generalised to `impl Into<Col>`;
  GREY.into() x2 -> NamedColor::Grey.into(). 10 lib tests + contract
  pass; workspace check clean (excl. web); fmt --check clean.
  LIGHT-palette HTML dump (Game::start(3, 42), pub + 3 player states,
  temp render_dump test, deleted after; crate had no prior
  uncommitted changes so no stash needed): after normalising the six
  expected remaps (#dcdcdc->#dbdbdb, #bebebe->#bfbfbf,
  #646464->#616161, #505050->#616161, #f8bbd0->#f7bed4,
  #e64a19->#f57c00) dumps are byte-identical; contrast text counts
  (#000000) unchanged, so no contrast flips. Decision: kept a small
  `available_loc_bg()` fn (Col isn't const-constructible with Vec).
  No new gotchas.
- [x] Phase 24 lords-of-vegas-1 (2026-07-13): casino.rs 5 bespoke
  statics deleted; `Casino::color` retyped `-> NamedColor` (Albion ->
  Purple, Sphinx -> Orange per Appendix A ordering - no collision in
  practice, Vega -> Green, Tivoli -> Grey, Pioneer -> Brown);
  `Casino::render` -> `.contrast()`. render.rs: UNBUILT_TILE_BG 200
  grey -> Named Foreground soften 78 (#c7c7c7 on light, vs old
  #c8c8c8); WHITE unowned player_color + inlay_bg ->
  NamedColor::Background; all `.inv().mono()` -> `.contrast()`;
  GREEN -> NamedColor::Green in render_cash. Tests + contract pass;
  workspace check clean (excl. web); fmt --check clean. LIGHT HTML
  dump: Game::start(3, seed) is NOT deterministic across processes
  (card.rs shuffled_deck iterates HashMap TILES.keys() before
  shuffling - new gotcha, pre-existing), so dump used a hand-built
  PubState fixture (owned tiles per player + one built tile per
  casino, temp render_dump test, deleted after; crate-only stash for
  before). After the 6 hue substitutions dumps are byte-identical
  except 72 fg flips white->black, all on #388e3c (Vega label +
  green-player tiles; same WCAG flip precedent as Phase 21) and
  #f57c00 (Sphinx label; black clearly correct on bright orange).
  No unexpected diffs.
- [x] Phase 25 RGB removal (2026-07-13): ColType::RGB and
  From<Color>/From<&Color> for Col deleted from brdgme-markup (TNode
  stays concrete Color); rgb(...) parse-time reverse-map kept; legacy
  `{{c name}}` parser now emits Named via resolve_named + Go-alias
  table (deeppurple/indigo/lightblue/teal/lightgreen/lime/deeporange/
  bluegrey per Phase 1 reverse-map), unknown -> Foreground. All 21
  colour constants + PLAYER_COLORS/player_colors()/player_color()
  deleted from brdgme-color; Palette::player_color(s) (8-entry) is the
  replacement. Style::default() -> &LIGHT.foreground/background;
  Color::mono()/inv() KEPT (transform still applies ColTrans::Mono/
  Inv). Decision: FromStr for Color / named() kept for web DB colour
  names, rewritten to resolve against LIGHT with legacy aliases
  (amber->orange, bluegrey->cyan, etc.); db.rs "Assign colors" string
  list untouched (Phase 26). Consumers fixed: lib/cmd repl, lib/game
  command doc, bot, tools/render_plain, web/db.rs (WHITE ->
  LIGHT.background). cargo check/test --workspace --exclude web green;
  fmt clean; zero grep hits for ColType::RGB or deleted constants;
  web still fails only with pre-existing E0433 brdgme_game (3 sites,
  none colour-related).
- [x] Phase 26 api/bot migration (2026-07-13): the real DB colour
  surface is rust/web (there is no api/ dir; plan's api/src/db/color.rs
  doesn't exist). New migration rust/web/migrations/006_color_palette.sql:
  drops/recreates the UNUSED public.color enum (no column uses it -
  users.pref_colors and game_players.color are both text) with the
  8-entry palette order Green/Red/Blue/Orange/Purple/Brown/Cyan/Pink,
  then remaps stored text data: users.pref_colors via nested
  array_replace and game_players.color via UPDATEs (Amber->Orange,
  BlueGrey->Cyan per THEMING.md slot inheritance). Not run against any
  DB. web/src/db.rs "Assign colors" list -> 8-entry palette order,
  fallback for >8 players BlueGrey->Pink. bot/system_prompt.md rgb()
  docs replaced with named-colour list + soften(color, pct) (bg only)
  + ` | contrast` transform (initial subagent wrote `{{fg contrast}}`,
  corrected to the real transform grammar `{{fg green | contrast}}`).
  brdgme-color FromStr/named() legacy aliases KEPT: not provably dead -
  game_players.color/pref_colors reads can still see old names on any
  DB where 006 hasn't run. Verify: cargo check --workspace --exclude
  web clean; bot tests 18/18; cargo fmt -p bot/-p web --check clean
  (fixed a Phase 25 overlong line in db.rs color()); web still fails
  only the pre-existing E0433 brdgme_game errors (server_fns.rs:44,
  game.rs:318, game.rs:384 - none colour-related). db.rs sqlx::test
  colour tests not run (need a database). Gotcha for 27/28:
  pre-existing fmt --check diffs remain in age-of-war-2/cathedral-2/
  for-sale-2 render.rs.
- [x] Phase 27 web theming (2026-07-13): CORRECTION to phases 1-26 notes:
  the "pre-existing web E0433 brdgme_game failure" was never a bug - plain
  `cargo check -p web` omits the feature-gated optional deps; the correct
  command is `SQLX_OFFLINE=true cargo check -p web --features ssr` (per
  docs/DEV.md), which passes. Library: TNode genericised to
  `TNode<C = Color>` (layout code colour-agnostic; existing API
  unchanged); new brdgme-markup semantic pipeline (`SemanticCol`/
  `SemanticColType`/`SemanticPlayer{name}`, `transform_semantic`,
  `html_class` emitting `mk-fg-{token}`/`mk-bg-{token}` classes: named,
  `soften-{name}-{pct}`, `player-{n}`, `c-` contrast prefix;
  `markup_class_css()` structural rules) and brdgme-color
  `palette_css_vars()`, `IN_USE_SOFTENS` = [(Foreground,86),
  (Foreground,75),(Foreground,78),(Pink,75)]. Decision: bare Mono/Inv
  chains fold into contrast in semantic mode (no real users per audit).
  Themes: `themes()` registry ("brdgme light"/"brdgme dark"/"dracula");
  contrast gate test in brdgme-color over all registered themes (3:1
  text floor, 4.5:1 contrast output, CIE76 deltaE >= 15.0 player/GREY/
  FOREGROUND distinguishability calibrated against LIGHT's minimum
  19.02 brown-vs-grey, plus the specific hue pairs). Gate scoping
  decision: text-on-background covers the 7 hues actually used as bare
  text (YELLOW/ORANGE excluded; they'd fail LIGHT itself). Tuned: DARK
  brown #a1887f -> #8f533d (deltaE vs grey); DRACULA foreground
  #f8f8f2 -> #f8f8f8 (soften blew up residual hue into muddy yellow),
  blue #6987f5 -> #708cf5, brown #b8825e -> #bc8967 (both for 4.5:1
  contrast output), grey #6272a4 -> #b8bfd6 (Comment fails on softened
  surfaces - the trade-off THEMING.md predicts). LIGHT untouched. Web:
  server_fns render via transform_semantic/html_class; per-game
  `--mk-player-{n}` vars as inline style on board/log containers
  (GameViewData.player_style); db.rs hex resolution removed (slot
  tokens via slot_from_color_name, Amber->orange BlueGrey->cyan
  unknown->grey); SSR shell emits static theme <style>
  (markup_class_css + :root light + prefers-color-scheme dark fallback
  + per-theme `[data-theme=...]` blocks scoped to any element) + tiny
  blocking boot script (cookie -> data-theme pre-paint, slug-validated,
  no flash); instant client-side switching (data-theme swap first,
  cookie + fire-and-forget profile sync after); /theme picker page with
  live preview grid (each tile data-theme-scoped, shared html_class
  sample - user-requested mid-phase); migration 007_user_theme.sql
  (users.theme text nullable); profile wins on login, local explicit
  choice pushed up when profile empty; new SQL as non-macro sqlx
  (offline cache untouched, no DB available). Chrome: only body bg/fg
  wired to vars; main.scss's ~20 hardcoded chrome colours NOT rethemed
  (Phase 28+ gotcha). Verify: contrast gate green x3 themes; web
  ssr+hydrate checks, theme unit tests (CSS content, slug/boot-script
  sync, sample html), cargo test --workspace --exclude web, fmt on
  web/brdgme_color/brdgme_markup all green. Not verified: sqlx::test
  DB tests and live SSR smoke (no database in env) - CSS emission
  verified by unit test instead.
- [x] Phase 28 final verification (2026-07-13): cargo check --workspace
  --exclude web, SQLX_OFFLINE=true cargo check -p web --features
  ssr/--features hydrate, cargo test --workspace --exclude web all
  green. web tests: 15 pass (incl. all 6 theme/boot-script unit
  tests); 52 sqlx::test DB tests fail solely on missing database
  (expected, unrun since Phase 26). Fixes this phase: 3 clippy
  warnings in lib/markup/src/parser.rs (manual_contains x2,
  unnecessary_lazy_evaluations) and cargo fmt on 9 migration-touched
  game crates (tic-tac-toe-2, jaipur-2, lost-cities-1/2,
  roll-through-the-ages-2, sushi-go-2, age-of-war-2, cathedral-2,
  for-sale-2 - the last three were the known pre-existing diffs; all
  were migration-touched files so reformatted). After fixes: clippy
  --workspace --exclude web and clippy -p web --features ssr zero
  warnings; cargo fmt --check clean workspace-wide. Greps clean:
  no ColType::RGB anywhere, no rgb( emission in game crates, game
  brdgme_color imports are NamedColor-only, player_color only via
  Palette (LIGHT.player_color). Golden sanity: tic-tac-toe-2/
  acquire-1/lords-of-vegas-1 tests pass; combined LIGHT HTML dump
  (Game::start seed 42, pub + player states, scratchpad harness)
  contains exactly {#000000,#ffffff,#616161,#d32f2f,#388e3c,#1976d2,
  #fbc02d,#7b1fa2,#f57c00,#5d4037} + softens #dbdbdb/#bfbfbf/#c7c7c7/
  #f7bed4 - all palette values, nothing stray. Contrast gate
  (gate_contrast_all_themes) green across the 3 themes. Series
  complete; still uncommitted; outstanding user-review items are D8,
  D10-D12, D15.
