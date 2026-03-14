# Current Work Status

## Phase 5.6: In Progress

All 13 blockers resolved. All 4 missing API endpoints implemented.
Frontend gaps and code quality items remain (see PLAN.md).

---

## Completed this session

### Dev environment fixes

- `k8s/kind-config.yaml`: pinned node image to `kindest/node:v1.34.0` to avoid
  kubelet bug in v1.35.0.
- `scripts/setup-kind-cluster.sh`: Kourier URL updated to `knative-extensions`
  org (repo moved); separate `KOURIER_VERSION="1.21.0"` variable added since
  Kourier does not publish patch releases matching Knative Serving.

### Auth fixes

- Login confirmation token changed from UUID to 6-digit numeric code
  (`format!("{:06}", rand::random::<u32>() % 1_000_000)`) to match frontend
  `type="tel"` / `pattern="[0-9]*"` validation.
- Logout now navigates to `/login` after `ServerAction` succeeds (`Effect` added
  in `SidebarMenu`).
- Email link on login code entry screen now resets back to email entry form on
  click.

---

## Immediate next tasks (Phase 5.6 frontend gaps)

From PLAN.md Phase 5.6 - remaining frontend/UI items:

1. **New-game UI** - page to create a new game (pick game type, invite players).
2. **Action buttons** - Undo, Concede, Restart wired to their endpoints in `GameMeta`.
3. **Dashboard active game list** - render `get_active_games` results in the
   sidebar/dashboard with turn indicators and navigation links.
4. **Game finished state** - show result/placing when `is_finished = true`.
5. **CSS for log classes** - `log-entry-new`, `log-window`, `log-window-heading`
   need styles in `style/main.scss`.
