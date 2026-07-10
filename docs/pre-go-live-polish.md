# Pre-Go-Live UI/UX Polish

This is a running collection of minor UI/UX jank noticed before go-live.
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
