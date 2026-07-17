# Pre-Go-Live Polish

This is a running collection of jank noticed before go-live - mostly
minor UI/UX issues, plus the occasional dev-process item.
Each entry records observed behavior and expected behavior. These are not
individually actioned as found - the list will be turned into a proper
superpowers spec/plan and fixed as one batch when scheduled.

## Entries

### 2026-07-10: Login email form has no loading state before enter-code form

- **Observed:** After submitting the email address on the login form, the
  form sits inert for about a second before jumping to the enter-code
  form - no pending/loading indication during that gap.
- **Expected:** An immediate loading state on submit (the legacy brdg.me
  site shows a spinner after submitting the email address) until the
  enter-code form renders.

### 2026-07-10: Sidebar reloads on every link click

- **Observed:** Each navigation causes the sidebar to invalidate and
  re-fetch, so the "Logout" link flashes to "Login" for a moment before
  flashing back, and the active game list briefly shows "Loading
  games...".
- **Expected:** The sidebar keeps its state across client-side navigation
  and does not invalidate/reload on every link click (no auth-state
  flash, no games-list loading flash).

### 2026-07-10: Favicon is the Leptos default

- **Observed:** The site still serves the default Leptos favicon.
- **Expected:** A brdg.me favicon: a simple flat dice in a material
  design style, showing the 6 side. Two colours only, taken from the
  brdg.me backgrounds - #ffffff for the dice body, #e0e0e0 for the pips
  and the dice outline. No gradients. Start with an SVG so it can be
  reused wherever needed.

### 2026-07-10: Login email doesn't match brdg.me style

- **Observed:** The login confirmation email doesn't use the brdg.me
  monospace style, and the branding is written "brdgme" in places.
- **Expected:** Monospace styling matching the legacy brdg.me email
  (white background, black text, Source Code Pro / Lucida Console
  monospace `<pre>` block), and the branding always written "brdg.me",
  never "brdgme". Legacy wording for reference: subject "brdg.me login
  confirmation", body "Your brdg.me confirmation is **NNNNNN** / This
  confirmation will expire in 30 minutes if not used." Legacy HTML:

  ```html
  <link
      href="https://fonts.googleapis.com/css?family=Source+Code+Pro:400,700"
      rel="stylesheet"
  >
  <pre
      style="
          background-color: white;
          color: black;
          font-family: 'Source Code Pro', 'Lucida Console', monospace;
      "
  >Your brdg.me confirmation is <b>643856</b>

  This confirmation will expire in 30 minutes if not used.</pre>
  ```

- **Note:** The legacy system sent login emails from play@brdg.me (the
  address used for game plays). Using login@brdg.me for login emails is
  fine, but game emails later on must come from play@brdg.me.

### 2026-07-10: CI runs every job on every change (dev process jank)

- **Observed:** All CI jobs (Rust test/build, Go test/build, e2e,
  kubeconform, legacy builds) run on every push, including docs-only
  commits - long, heavy builds for no benefit. Rust builds in
  particular are often really long even when they do need to run.
- **Expected:** Jobs only run when relevant paths change. Preferred
  mechanism: a `dorny/paths-filter` gate job with `if:` on its outputs,
  keeping a single `ci.yml` so the existing `needs:` chains still work
  and skipped jobs still report a status (unlike workflow-level
  `paths:` filters, which break required checks). Rough gating:
  - test-rust, cargo-deny, build-rust, e2e: `rust/**`,
    `docker-bake.hcl`, `.github/workflows/ci.yml`
  - test-go, build-go-games: `brdgme-go/**`, `.github/workflows/ci.yml`
  - kubeconform: `k8s/**`
  - build-legacy: `web/**`, `websocket/**`, `rust/api/**`
- **Caveat:** Docker builds use context `.` - verify what the
  Dockerfiles actually `COPY` and make sure the filters cover it.
- **Also investigate:** whether Rust build caching is as good as it can
  be - both the Swatinem/rust-cache CI jobs and the docker-bake
  registry-backed layer cache (cargo-chef stages) - since Rust builds
  are still often really long.
- **Related (post-go-live deploy direction):** Nothing tags images on
  git tags today - CI only triggers on master push and PRs, and images
  are only tagged `sha-<short>` and `latest`. When moving to
  tag-driven deploys, don't rebuild on tag push: retag the existing
  image with `docker buildx imagetools create -t ...:v1.2.3
  ...:sha-XXXXXXX`. Then move image-tag source of truth per the
  comment in `k8s/argocd/brdgme-app.yaml` (deploy repo/overlay or Argo
  CD Image Updater) to replace the manual bump. Edge case with path
  filtering: a tag on a docs-only commit has no `sha-` image - retag
  from the newest ancestor that built, or only tag commits that built.

### 2026-07-11: Sidebar "Menu" button does nothing on narrow viewports

- **Observed:** When the browser is narrow enough that the sidebar
  collapses (under the 80em breakpoint), the header's "Menu" button does
  nothing - the sidebar is permanently inaccessible on small screens.
- **Expected:** The Menu button opens the sidebar as an overlay;
  clicking outside it (or navigating) closes it.
- **Note:** The button is rendered with no `on:click` at all
  (`rust/web/src/components/layout.rs` line ~18). The mobile collapse
  CSS already exists (`rust/web/style/main.scss`, the
  `@media (max-width: 80em)` block: `.menu.open`, `.menu-close-underlay`)
  but was carried over from the legacy app and never wired up in the
  Leptos port - `layout.rs` never toggles an `open` class or renders the
  underlay.

### 2026-07-11: Input fields are not auto-focused

- **Observed:** Nothing ever receives focus automatically - the login
  email field, the code field after submitting the email, and the game
  command field (on opening a game and after making a play) all require
  a click before typing.
- **Expected:**
  - Login page: email field focused on load.
  - After submitting the email: the code field is focused.
  - Opening a game: the command field is focused.
  - After a play is submitted: the command field is focused again.
  - While viewing a game, starting to type focuses the command field if
    no other input field currently has focus.
- **Caveat (accessibility):** the type-anywhere behaviour must not steal
  keyboard navigation - if a link or other interactive element has
  focus, keys (especially Enter) must keep their normal behaviour; only
  divert printable-character keystrokes when nothing focusable is
  active.

### 2026-07-11: Main content flashes white when submitting a command

- **Observed:** Entering a command in a game blanks the main content to
  white, then the updated game appears.
- **Expected:** The current game stays visible until the updated state
  arrives; the update swaps in without any blank frame.
- **Note:** `GamePage`'s game `Resource` is wrapped in `<Suspense>` with
  an empty-`<div>` fallback (`rust/web/src/app.rs` ~line 563), which
  blanks on every refetch. `docs/superpowers/plans/2026-07-05-bugs.md`
  (line ~92) records this exact bug as already fixed by switching the
  outer `Suspense` to `Transition` - but no `Transition` exists anywhere
  in `rust/web/src` today, so the fix regressed or was reverted. Check
  the sibling `GameLogs`/`GameMeta` suspense boundaries for the same
  pattern while fixing.

### 2026-07-11: Page title should show how many games are waiting on you

- **Observed:** The `<title>` is the static "brdg.me"
  (`<Title text="brdg.me"/>` in `rust/web/src/app.rs`), so a
  backgrounded tab gives no cue when it becomes your turn.
- **Expected:** "brdg.me (N)" where N is the number of active games
  where it is your turn; plain "brdg.me" when N is 0. Updates live on
  websocket game updates.
- **Note:** The sidebar already fetches the active-games list (with
  whose-turn info) and re-keys it on the websocket signal
  (`rust/web/src/components/layout.rs`) - the title can derive from the
  same data rather than a new query.

### 2026-07-11: Favicon grey is too light against the tab background

- **Observed:** Beta testing the deployed batch (deploy sha-48686c8) - the
  dice favicon shape is correct, but the `#e0e0e0` grey used for the pips
  and outline is too light to see against browser tab backgrounds.
- **Expected:** A darker grey with enough contrast to read clearly as a
  dice in the tab bar.
- **Note:** Michael has already edited `rust/web/public/favicon.svg` in his
  working tree to use `#606060` for the grey. The work here is to land
  that edit and verify visibility after the next deploy.
- **Resolved:** Confirmed fixed by Michael 2026-07-17.

### 2026-07-11: Game log sections still flash on command submit

- **Observed:** Beta testing the deployed batch (deploy sha-48686c8) - the
  entry above (Suspense -> Transition in `GamePage`) stopped the board
  from white-flashing on command submit, but the "Recent logs" section
  above the command input and the logs in the right sidebar still flash
  after submitting a command.
- **Expected:** Same fix class as the board: those log resources/
  components keep showing stale data while refetching instead of
  remounting or dropping to a loading state.
- **Resolved:** Confirmed fixed by Michael 2026-07-17.

### 2026-07-11: No loading indicator on initial game page load

- **Observed:** Beta testing the deployed batch (deploy sha-48686c8) -
  when a game page is first opened, there is no loading indication while
  the game data loads.
- **Expected:** Show the same spinner used on the login page, vertically
  and horizontally centered in the game area, while the game page loads.
- **Note:** Make the spinner a reusable component so other pages that
  load data from the server can show it too.
- **Resolved:** Consolidated 2026-07-17 into the "No loading indicator
  when navigating to a game" entry below - same requirement, refined
  there (spinner only when routing to a game for the first time, never
  on websocket/command rerenders).

### 2026-07-11: Command input stays enabled while a command is submitting

- **Observed:** Beta testing the deployed batch (deploy sha-48686c8) - the
  command input and send button remain enabled while a game command is
  being submitted, allowing re-submission before the first one completes.
- **Expected:** Disable the command input and the send button while
  submitting. On success, clear the input (current behavior). On error,
  re-enable both but keep the submitted text in the input so the user can
  correct and resubmit.

### 2026-07-11: Autocomplete click doesn't focus the command input

- **Observed:** Beta testing the deployed batch (deploy sha-48686c8) -
  clicking an autocomplete word above the game command input correctly
  inserts the word into the command input, but focus does not move.
- **Expected:** The command input should also be focused after the click
  so the user can keep typing.
- **Note:** Related to the input-auto-focus entry above (Task 9 work) -
  the click lands on the suggestion element, so the type-anywhere keydown
  handler's "nothing focused" guard does not help here. The click handler
  itself should refocus the input.

### 2026-07-11: Header "Sub menu" button does nothing

- **Observed:** On narrow viewports the header's "Sub menu" button is
  visible but clicking it does not open the game meta panel - same class
  of bug as the earlier inert "Menu" button (no `on:click` at all; the
  `.game-meta.open` / `.game-meta-close-underlay` CSS in the
  `@media (max-width: 60em)` block was never wired up).
- **Expected:** The button opens the game meta panel as an overlay;
  clicking the underlay or navigating closes it, like the sidebar menu.
- **Resolved:** Fixed same day - `MainLayout` provides a `SubMenuOpen`
  context that `GameMeta` consumes to toggle `.open` and mount the
  underlay.

### 2026-07-11: "Sub menu" button visible at widths where the panel is already shown

- **Observed:** Between 60em and 80em the main menu is collapsed (so the
  header is visible) but the game meta panel is still on screen - yet the
  "Sub menu" button shows anyway. Only the "Menu" button should show in
  that range.
- **Expected:** The "Sub menu" button only appears below 60em, where the
  game meta panel actually collapses.
- **Resolved:** Fixed same day - `.header-sub-menu` is `display: none` by
  default and only shown inside the 60em media block.

### 2026-07-11: Header buttons should be icons, sub menu right-aligned

- **Observed:** "Menu" and "Sub menu" are plain form buttons with text
  labels, and the sub menu button sits directly right of the heading.
- **Expected:** Unicode icon buttons - U+2630 (the trigram hamburger, the
  standard choice over U+2261 which is the maths "identical to" operator)
  for the menu, U+22EE (vertical ellipsis, the conventional
  secondary/overflow "kebab" menu glyph, broadly available) for the sub
  menu - with the sub menu button aligned to the right edge of the screen.
- **Resolved:** Fixed same day - borderless `.header-icon-button` styling,
  `aria-label`s kept for accessibility, title given `flex: 1` so trailing
  buttons align right.

### 2026-07-11: Titlebar colour only reflects the current game's turn

- **Observed:** The header's my-turn highlight was driven by an
  `is_my_turn` prop only the game page passed, so on the index/dashboard
  (or a game where it is not your turn) the bar never showed the active
  colour even with other games waiting.
- **Expected:** The bar shows the active colour whenever ANY active game
  is awaiting the player's turn, on every page the bar is visible.
- **Resolved:** Fixed same day - `MainLayout` derives it from the shared
  active-games resource (same data as the sidebar and title badge).

### 2026-07-11: "Next game" button broken

- **Observed:** The header's "Next game" button had no click handler at
  all, and its visibility was just "is it my turn in the current game".
- **Expected:** Links to the active game that has been awaiting the
  player's turn the longest (oldest `game_players.is_turn_at`); hidden
  when there is no such game or the player is already viewing it.
- **Resolved:** Fixed same day - `is_turn_at` added to `GameSummary`, the
  target computed in `MainLayout` from the shared active-games resource.

### 2026-07-17: Settings page scrolls the whole page

- **Observed:** On the settings page the entire page scrolls, sidebar
  included.
- **Expected:** Only the main content area scrolls; the sidebar stays
  static. Research current best practice before implementing - the
  standard app-shell pattern is a full-viewport (`100dvh`) flex/grid
  container with the sidebar and main pane as children, `overflow-y:
  auto` on the main pane only, and the `body` itself never scrolling.
  Verify how the game pages handle this today and align the approach so
  all pages share one layout convention.
- **Resolved:** Fixed same day - app-shell pattern: `.layout` bounded to
  `100dvh` and `.content` given `overflow-y: auto`, so tall pages scroll
  inside the content pane; game pages already fit the viewport and are
  unaffected.

### 2026-07-17: Content pages too narrow (settings needs 3 theme columns)

- **Observed:** The settings page main content is too narrow - the theme
  picker can't fit 3 columns.
- **Expected:** Content-based (non-game) pages get a wider centered
  max-width, around 1200-1220px (Wikipedia uses 1220px; Bootstrap xxl
  container is 1320px, MUI's lg is 1200px - ~1220px is squarely within
  best practice). Wide enough for 3 theme columns on the settings page.
  Game pages are unaffected.
- **Resolved:** Fixed same day - new shared `.content-page` wrapper
  (max-width 1220px, centered) applied to settings, home, dashboard and
  new-game pages, replacing the settings page's 40em cap. Game pages
  unaffected.

### 2026-07-17: Selected theme should be indicated by border, not name highlight

- **Observed:** The selected theme is indicated by highlighting the
  theme's name.
- **Expected:** The selected theme's tile instead gets a thicker border
  in the highlight colour; the name is not highlighted.
- **Resolved:** Fixed same day - the `selected` class moved from the tile
  label to the tile itself, styled as a 3px highlight-colour border with
  padding compensation; the name highlight is gone.

### 2026-07-17: Theme colour preview should be solid swatch blocks

- **Observed:** Theme colours preview as coloured name blocks, e.g.
  `<bg green>Green </bg><bg red>Red </bg>`.
- **Expected:** No text at all - each colour is a swatch of 5 spaces
  rendered with the background colour, packed with no gaps between
  swatches, in 2 rows of 5. The 10 swatches are the accent colours in
  `NamedColor::ALL` order (`rust/lib/color/src/palette.rs`): row 1 Red,
  Green, Blue, Yellow, Purple; row 2 Cyan, Pink, Orange, Brown, Grey.
  Foreground/Background are excluded (the tile itself already shows
  them).
- **Resolved:** Fixed same day - `SAMPLE_MARKUP` replaced with 10
  text-free 5-space background swatches in `NamedColor::ALL` accent
  order, packed in 2 rows of 5 via `white-space: pre` and
  inline-block spans.

### 2026-07-17: Username change shows stale value when returning to settings

- **Observed:** After updating the username it applies immediately
  (game pages show the new name), but navigating back to the settings
  page shows the pre-save username.
- **Expected:** The name is not cached anywhere after a change - the
  settings form always shows the current value. Likely a
  resource/context holding the old user record that isn't invalidated
  on save; find and invalidate (or refetch) it.
- **Resolved:** Fixed same day - `get_settings` now reads the name from
  the `users` table (source of truth) instead of the session-cached
  value, and `set_username` also refreshes the session copy so
  `get_current_user` is immediately correct too.

### 2026-07-17: ELO rating change not shown when a game finishes

- **Observed:** rust/web renders the ELO rating at game end, but not the
  rating change.
- **Expected:** Match live brdg.me, which renders the ELO rating change
  next to the ELO rating when a game finishes. Check the legacy
  implementation (live brdg.me, github.com/beefsack/brdg.me) for the
  exact presentation and make rust/web consistent with it.
- **Resolved:** Fixed same day - `PlayerViewData.rating_change` plumbed
  from `game_players.rating_change` (already stored and queried) and
  rendered next to the rating in the legacy brdg.me format
  `Rating: <n> (<icon><abs change>)` with the existing
  rating-change-up/down/none colour classes, shown only once a change
  exists.

### 2026-07-17: Command input sometimes clears itself while typing

- **Observed:** While typing into the game command input, the input
  sometimes clears itself spontaneously. Gut feeling (Michael): it
  happens when an update arrives for the currently open game or for one
  of the other active games in the sidebar.
- **Expected:** Typed input is never lost to background updates.
- **Note:** Needs investigation + a reproduction first (e.g. trigger a
  websocket game update while typing - a bot game or second account).
  Likely cause class: a reactive re-render/remount of the input (or a
  value-controlled reset) on the websocket-driven refetch - same family
  as the earlier flash-on-update entries. Fix so the input value
  survives updates.
- **Resolved:** Fixed same day - two confirmed causes. (1) `GamePage`'s
  per-game WS memo (`seq_for_this_game`) collapsed to `None` whenever an
  update arrived for a DIFFERENT game, re-keying `game_data` and
  remounting the whole game view mid-typing - it now tracks
  `(game_id, last seq)` via a pure `track_game_seq` helper (unit-tested)
  so other games' updates are no-ops. (2) `GameCommandInput`'s text lived
  in a local signal inside the `<Transition>` closure, so even legitimate
  refetches of the open game reset it - hoisted to `GamePage` via a new
  `CommandInputText` context (same pattern as the `logs` resource),
  cleared on game-to-game navigation.

### 2026-07-17: Sub menu button still not showing on mobile game pages

- **Observed:** When viewing a game on a mobile-width viewport, the sub
  menu button does not appear at all - despite the 2026-07-11 fixes
  above ("Sub menu button does nothing" / "visible at widths where the
  panel is already shown" / "Header buttons should be icons"), which
  were meant to leave it working below 60em.
- **Expected:** On game pages below the 60em breakpoint the sub menu
  button shows as the vertical-ellipsis character (U+22EE), aligned to
  the right-hand side of the title bar, and opens the game meta panel.
- **Note:** Check whether the earlier `.header-sub-menu` display rules /
  `SubMenuOpen` wiring regressed or never actually shipped to the
  deployed build.
- **Resolved:** Fixed same day - the 2026-07-11 `SubMenuOpen` wiring and
  icon button shipped, but the CSS rule re-showing `.header-sub-menu`
  inside the 60em media block was never written; added it.

### 2026-07-17: Invalid command errors surface as raw server function errors (and HTTP 500)

- **Observed:** Submitting an invalid game command (e.g. in Acquire's
  buy phase) shows the user
  `error running server function: expected buy or done`, and the
  `submit_command` server fn responds `HTTP 500` with body
  `ServerError|expected buy or done` (observed 2026-07-16 on
  beta.brdg.me, cf-ray a1bd57e69883e7e2-SYD).
- **Expected:**
  - User-facing message: `Invalid command: expected buy or done` - no
    "error running server function" leakage.
  - An invalid command is user input error, not a server fault - it
    should be a 4xx (and ideally a typed/expected server fn error the
    client renders directly), not a 500.
- **Note:** Leptos server fns default unhandled errors to
  `ServerFnError::ServerError` (hence the 500 + generic prefix). The
  fix class is returning a dedicated user-input error variant from
  `submit_command` for game-rejected commands and rendering it in the
  command input's error area, rather than letting it bubble as a
  generic server error.
- **Resolved:** `execute_command` now returns a new
  `ExecuteCommandError::UserError(String)` variant for game-rejected
  commands (`Response::UserError` and non-empty `remaining_input`),
  instead of folding them into `anyhow::Error`. `submit_command` changed
  signature to `Result<Option<String>, ServerFnError>` -
  `Ok(None)` = success, `Ok(Some(message))` = the game rejected the
  command (same expected-rejection pattern as `set_username`), `Err`
  only for real transport/server faults. `GameCommandInput` renders
  `Ok(Some(msg))` as `Invalid command: {msg}` and `Err` as a generic
  "Failed to submit command. Please try again." (never the raw
  `ServerFnError` text). Responds `200` rather than a literal `4xx`;
  a typed 4xx would need a custom Leptos server-fn error type and is
  left as a follow-up if ever required.

### 2026-07-17: No loading indicator when navigating to a game

- **Observed:** Clicking a link to a game (e.g. from the sidebar or
  dashboard) gives no loading indication while the game loads.
- **Expected:** Show the same loading indicator the login form uses,
  centered in the game area, while the game is loading - but ONLY when
  navigating to a game the user is not already viewing (initial load or
  clicking through to a different game). It must NOT appear when the
  currently visible game refreshes from a websocket update or a command
  submission - those keep showing the current state until the update
  swaps in (per the earlier flash-on-update fixes).
- **Note:** Consolidates/supersedes the 2026-07-11 "No loading indicator
  on initial game page load" entry (reusable spinner component,
  login-page spinner) - confirmed the same requirement by Michael
  2026-07-17: a loading icon when routing to a game the first time, not
  on rerenders after websocket updates or commands. The
  2026-07-17 command-input fix keys `game_data` on
  `(game_id, last seq)` - the spinner condition should distinguish a
  game_id change (show spinner) from a seq-only refetch (don't).
- **Resolved:** Reusable `Spinner` component extracted from the login form
  (`rust/web/src/components/spinner.rs`); `GamePage`'s `<Transition>` is
  now remounted via a deduped `current_game` memo (game id only), so its
  centered-spinner fallback shows on initial load and game-to-game
  navigation, while seq-only WS/command refetches keep the stale board.
