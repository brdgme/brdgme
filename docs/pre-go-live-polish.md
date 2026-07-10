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
