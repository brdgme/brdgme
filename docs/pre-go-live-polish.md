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
