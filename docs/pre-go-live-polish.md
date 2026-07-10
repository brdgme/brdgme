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
