# F-96 follow-up: `TURNSTILE_SECRET_KEY`

Out-of-band task, requested by the owner 2026-07-30. Read-and-report only; no
cluster commands were run and no source was modified. Uncommitted like the rest
of this directory.

Turnstile is **not in production yet**, so this is a pre-rollout blocker, not a
live outage.

---

## (a) What `TURNSTILE_SECRET_KEY` is, and every site that touches it

### It is Cloudflare Turnstile - confirmed, not assumed

Evidence, not inference:

- Verify endpoint hard-coded at `rust/web/src/auth/server.rs:262`:
  `https://challenges.cloudflare.com/turnstile/v0/siteverify`
- Widget script tag at `rust/web/src/app.rs:90`:
  `https://challenges.cloudflare.com/turnstile/v0/api.js`
- Widget markup uses the `cf-turnstile` class with `data-sitekey`
  (`rust/web/src/app.rs:663`)
- `docs/changes/archive/2026-07-23-email-restrictions-plan/plan.md:11` - "R3 Cloudflare
  Turnstile CAPTCHA widget on the login form"

Turnstile is Cloudflare's CAPTCHA alternative; here it is used as a bot gate on
the **login form only**. There is no dedicated crate - it is a plain `reqwest`
form POST.

### Every reference in the repo

Code:

| Path | Lines |
|---|---|
| `/home/beefsack/Development/brdgme/rust/web/src/main.rs` | 40, 41, 44 |
| `/home/beefsack/Development/brdgme/rust/web/src/auth/server.rs` | 257, 262, 273, 274, 280-282, 286, 289, 290, 1857-1866 |
| `/home/beefsack/Development/brdgme/rust/web/src/app.rs` | 11, 90, 339, 523, 527, 552, 591, 663 |
| `/home/beefsack/Development/brdgme/rust/web/src/components/game.rs` | 359 (comment only) |

Docs: `docs/CODING.md:532`;
`docs/changes/archive/2026-07-23-email-restrictions-plan/plan.md` (~50 hits);
`docs/changes/archive/2026-07-08-28-abuse-protection/spec.md:93,95,123`;
`docs/changes/archive/2026-07-10-28-wp4-cloudflare-pre-golive/spec.md:151`;
`docs/reviews/2026-07-23-rust-review/SUMMARY.md:28,93,120,156`.

**Zero hits in `k8s/`, in CI, in the Tiltfile, or in any `.env`.** Neither
`TURNSTILE_SECRET_KEY` nor `TURNSTILE_SITE_KEY` is provisioned by any manifest.
`k8s/base/web/deployment.yaml:33-39` mounts only `postgres-config`,
`email-config` and `database-encryption-key`; `k8s/prod/app/web-patch.yaml`
adds only the two Sentry DSNs.

### Where the panic happens

`/home/beefsack/Development/brdgme/rust/web/src/main.rs:40-45`, inside
`async fn main()` - **process startup**, not lazy on first request. It runs
after `crypto::load_key()` and before the DB pool is built:

```rust
let turnstile_secret = std::env::var("TURNSTILE_SECRET_KEY").unwrap_or_default();
if turnstile_secret.is_empty()
    && std::env::var("ALLOW_INSECURE_DEFAULT_KEY").as_deref() != Ok("true")
{
    panic!("TURNSTILE_SECRET_KEY not set - refusing to start without CAPTCHA verification");
}
```

A prod deploy without the var **crash-loops**; it does not degrade.

### How the value is threaded

It is not. `turnstile_secret` in `main.rs` is bound solely for the emptiness
check and never used again - it is not placed in `AppState` and not
`provide_context`'d. The request path re-reads the env var itself at
`rust/web/src/auth/server.rs:289`. Only the `reqwest::Client` is threaded
through context (`server.rs:288`).

### Consumers

Exactly one: `login()` at `rust/web/src/auth/server.rs:290`. `confirm_login()`
and `add_email_address()` have no Turnstile check.

### Behaviour with a present-but-wrong secret: FAIL CLOSED

`rust/web/src/auth/server.rs:256-277`. The deciding line is:

```rust
Ok(r) => r.json::<serde_json::Value>().await
    .map(|v| v["success"].as_bool().unwrap_or(false))
    .unwrap_or(false),
```

- Wrong secret: Cloudflare replies 200 with
  `{"success": false, "error-codes": ["invalid-input-secret"]}` -> `false` ->
  login rejected with "CAPTCHA verification failed."
- Transport error / timeout (5s connect, 10s total, `main.rs:48-51`): `Err`
  arm logs a warning, bumps `turnstile_verify_error_total`, returns `false`.
- Non-200: `send()` returns `Ok` for any status (no `error_for_status()`), so
  it falls through to the JSON parse; non-JSON body -> `unwrap_or(false)`.
- Malformed JSON, or `success: false` -> `false`.

There is no "treat verifier trouble as pass" branch anywhere. **Every error
path fails closed.**

### The one fail-open, and dev reachability

`rust/web/src/auth/server.rs:257-260`:

```rust
if secret.is_empty() || token.is_empty() {
    return secret.is_empty();
}
```

An **empty** secret returns `true` - allow. That is a real fail-open, and it is
the dev carve-out. It is unreachable in a real process because `main.rs:41-44`
panics first, unless `ALLOW_INSECURE_DEFAULT_KEY=true`.

So yes, a dev can run without the secret - via `ALLOW_INSECURE_DEFAULT_KEY=true`,
which is already set in `k8s/dev/web-patch.yaml:18-19` and
`scripts/rust-test.sh:64`. There is no feature flag or `cfg(test)` bypass beyond
`#[cfg(feature = "ssr")]` on the verifier itself.

### The site key is the second, quieter half of the problem

`TURNSTILE_SITE_KEY` (public) has **no** panic - `get_turnstile_site_key()`
(`auth/server.rs:280-283`) returns `unwrap_or_default()`. When it is empty the
widget div does not render at all (`app.rs:657-666`), so
`get_turnstile_response()` (`app.rs:523-533`) yields an empty string, which
`login()` rejects.

**Setting only the secret key produces a total login outage** - every login
fails CAPTCHA with no widget on screen to retry. Both vars must land together.
`hard_navigate()` (`app.rs:337-346`) exists because Turnstile's implicit
rendering only scans the DOM when `api.js` runs, so `/login` must be a full
page load - worth remembering when testing the rollout.

### Tests

`rust/web/src/auth/server.rs:1856-1868`, two `#[tokio::test]`s:

- `verify_turnstile_rejects_on_transport_error` - **makes a real network call
  to Cloudflare**. With network available Cloudflare answers `success: false`
  for the bogus secret, so it passes for the wrong reason; it does not actually
  exercise the transport-error arm.
- `verify_turnstile_empty_secret_allows_dev` - pins the empty-secret fail-open
  as intended.

No test covers non-200, malformed JSON, or a live `success: false`.

---

## (b) Should it use a dev default plus a log warning, like the encryption key?

### First: the owner's description of the encryption-key pattern is not accurate

Checked before recommending anything. **`rust/web` does not use "dev default plus
a log warning".** It uses *panic unless an explicit insecure opt-in flag is set*.

`rust/web/src/crypto.rs:56-75`:

```rust
pub fn load_key() -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let hex_str = match std::env::var("DATABASE_ENCRYPTION_KEY") {
        Ok(v) => v,
        Err(_) => {
            if std::env::var("ALLOW_INSECURE_DEFAULT_KEY").as_deref() == Ok("true") {
                return Ok(default_key());
            }
            return Err(CryptoError::MissingKey);
        }
    };
```

`load_key()` itself neither panics nor logs. The panic and the warning are at the
call site, `rust/web/src/main.rs:33-38`:

```rust
web::crypto::load_key().expect("DATABASE_ENCRYPTION_KEY missing or malformed");
if web::crypto::using_default_key() {
    tracing::warn!(
        "DATABASE_ENCRYPTION_KEY not set - using insecure default key (ALLOW_INSECURE_DEFAULT_KEY=true), DO NOT USE IN PRODUCTION"
    );
}
```

The **flag is the load-bearing part**, not the default. Also note the gate only
covers the *absent-variable* branch - a malformed or wrong-length key is a hard
error regardless of the flag.

And `docs/CODING.md:701-703` explicitly forbids the pattern the owner described:

> **Refuse startup on a missing `DATABASE_ENCRYPTION_KEY`.** Production fails
> closed: an unset encryption key must prevent the process from starting, not
> silently fall back to a hardcoded default key. (D-12, 2026-07.)

with the Turnstile half at `docs/CODING.md:532-534`:

> **Fail closed in production on auth failure.** Turnstile verifier errors reject
> rather than pass. (D-12, 2026-07; the matching `DATABASE_ENCRYPTION_KEY`
> startup rule is in the Database section.)

The owner's description **is** literally accurate about a different crate:
`rust/bot/src/crypto.rs:66-76` falls back to the hardcoded dev key with a
`tracing::warn!` and **no gate at all**, in every environment including prod.
That is a straight violation of `docs/CODING.md:701` and a live finding in its
own right (adjacent to F-90, the known web/bot crypto divergence). It is not what
`rust/web` does.

### The house pattern, for reference

Secrets in `rust/web` panic at startup: `DATABASE_URL` (`db/mod.rs:96`),
`DATABASE_ENCRYPTION_KEY` (`main.rs:33`), `TURNSTILE_SECRET_KEY` (`main.rs:40`).
Silent defaults are reserved for non-secret config (`NATS_URL`,
`PUBLIC_BASE_URL`, `METRICS_ADDR`, sweep intervals). The single exception is
`RESEND_WEBHOOK_SECRET`, which has no startup check and instead fails closed
per-request with a 500 (`email/inbound.rs:588-594`).

### Recommendation: no, and it already doesn't need to be

`TURNSTILE_SECRET_KEY` **already implements the real house pattern** - panic on
missing, with the same `ALLOW_INSECURE_DEFAULT_KEY=true` escape hatch, already
set in `k8s/dev/web-patch.yaml:18-19` and `scripts/rust-test.sh:64`. A dev clone
runs today without the secret. There is no ergonomic problem to solve.

Weighing the risk asymmetry the owner asked about: a bogus Turnstile secret
**fails closed** (see (a) - every error path returns `false`), so the danger of a
dev default is *not* silent bypass of the CAPTCHA by an attacker. It is the
opposite - a hardcoded fake secret would make every login fail with no
indication why. Meanwhile the one genuine fail-open, `secret.is_empty()` ->
`true` (`auth/server.rs:257-260`), is exactly what the startup panic exists to
keep out of production. **Removing the panic in favour of a bare dev default
would delete the only thing standing between a missing env var and a
CAPTCHA-disabled login endpoint.** Do not do it.

What should change is the *deployment* side, which is what F-96 is really about:

1. **Provision both vars in k8s.** See (c). `TURNSTILE_SECRET_KEY` alone is not
   enough.
2. **`TURNSTILE_SITE_KEY` needs a startup check too.** Today it silently defaults
   to empty (`auth/server.rs:280-283`), which renders no widget and rejects every
   login. Secret-set-but-site-key-unset is a total login outage that the current
   panic does nothing to prevent - and it is the most likely way to get the
   rollout wrong. Gate it on the same flag.
3. **Consider splitting `ALLOW_INSECURE_DEFAULT_KEY`.** One flag currently
   disables two unrelated production guards (`crypto.rs:60` and `main.rs:42`).
   Setting it to work around an encryption-key issue silently also disables the
   CAPTCHA check. Low priority, but it is a trap.
4. **Fix `rust/bot/src/crypto.rs`** to match web's gate. Route with F-90.

Optional nit for the remediation list: `verify_turnstile_rejects_on_transport_error`
(`auth/server.rs:1856-1862`) makes a real network call to Cloudflare and passes
for the wrong reason when the network is up. It does not test what it claims.

---

## (c) Sealed-secret commands

### Premise correction first

**`brdgme-config` is not a Kubernetes Secret. It is a separate GitOps
repository** - `git@github.com:brdgme/brdgme-config.git`, checked out locally at
`/home/beefsack/Development/brdgme-config`. All SealedSecret CRs live there, in
`sealed-secrets/secrets/`, not in the `brdgme` repo.

The web deployment consumes three secrets
(`k8s/base/web/deployment.yaml:33-40`):

```yaml
          envFrom:
            - secretRef:
                name: postgres-config
            - secretRef:
                name: email-config
            - secretRef:
                name: database-encryption-key
```

No overlay adds another. So "set it in `brdgme-config`" resolves to: create or
update a SealedSecret in the config repo, and make sure the web deployment has a
`secretRef` for it.

### The established facts these commands are derived from

| Fact | Source |
|---|---|
| Namespace | `brdgme` (`k8s/prod/app/kustomization.yaml:1`, both ArgoCD Applications' `destination.namespace`) |
| Controller | `sealed-secrets-controller` in `kube-system` - kubeseal's defaults | `brdgme-config/README.md` bootstrap step 1 |
| Scope | **Strict** - no `cluster-wide` or `namespace-wide` annotation on any of the 8 existing SealedSecrets. Name and namespace are both baked into the ciphertext. |
| Sealed files committed? | Yes, one file per secret, `<secret-name>.yaml`, in `brdgme-config/sealed-secrets/secrets/`, each listed in that dir's `kustomization.yaml` `resources:` |
| Rendering | `brdgme-config/prod/kustomization.yaml` pulls `brdgme//k8s/prod?ref=<sha>` plus `../sealed-secrets/secrets`; ArgoCD auto-syncs `path: prod` from `main` with `prune: true, selfHeal: true` |
| Tooling | `kubeseal` is in the **config repo's** devenv, not this repo's. Run from `/home/beefsack/Development/brdgme-config` inside `devenv shell`. |
| Public cert | **Not committed anywhere.** kubeseal must fetch it from the live controller, which is why every command needs cluster access. |

The documented pattern, verbatim from `brdgme-config/README.md`:

```
kubectl create secret generic email-config -n brdgme \
  --from-literal=RESEND_API_KEY=... --from-literal=EMAIL_FROM=login@brdg.me \
  --dry-run=client -o yaml | kubeseal --format yaml > sealed-secrets/secrets/email-config.yaml
```

> Add each new file to the `resources:` list in
> `sealed-secrets/secrets/kustomization.yaml`, then commit.

No documented command passes `--controller-name` or `--controller-namespace`;
all rely on kubeseal's defaults, which happen to match this cluster.

Precedent from git history: `6e4e879` ("seal(email-config): add
RESEND_WEBHOOK_SECRET...") shows that **adding a key to an existing secret means
re-sealing the whole file** - there is no partial-key merge. Commit bodies are
empty; no creation command is recorded in any commit message.

### DECISION REQUIRED before running anything

Nothing in either repo says where the Turnstile keys should live. Two options:

- **(A) New `turnstile-config` secret** - matches the one-file-per-concern
  convention, but requires a code-repo change too: a fourth `secretRef` in
  `k8s/base/web/deployment.yaml`, plus a new `ref=` sha in
  `brdgme-config/prod/kustomization.yaml`. Two repos, two PRs.
- **(B) Add both keys to the existing `email-config`** - no code-repo change at
  all, since `email-config` is already in `envFrom`. Requires re-sealing all five
  of its keys, so you need the current plaintext of `RESEND_API_KEY`,
  `RESEND_WEBHOOK_SECRET` and `EMAIL_FROM` in hand.

Commands for both are below. **(A) is the cleaner fit** but is not free; the
owner should pick.

Also note `TURNSTILE_SITE_KEY` is a **public** value (it ships to every browser
in the widget markup). It does not need sealing on confidentiality grounds - it
could equally be a plaintext `env` entry in `k8s/prod/app/web-patch.yaml`
alongside the Sentry DSNs. Sealing it anyway is harmless and keeps the two
together. Both are included below.

### Shell: this is fish

Every command below is written for fish. Differences that bite:

- **`export VAR=x` does not exist in fish** - use `set -x VAR x`.
- **`VAR=x some-command` prefix assignment does not exist in fish** - use
  `env VAR=x some-command`.
- `|`, `>`, `~` and backslash line-continuation all behave the same as bash.
- **fish has no `HISTCONTROL=ignorespace`.** A leading space does *not* keep a
  command out of history. Anything you type with the literal secret in it lands
  in `~/.local/share/fish/fish_history` in plaintext. Prefer the stdin variant
  below.

### Step 0 - environment

```fish
cd /home/beefsack/Development/brdgme-config
devenv shell
git pull
```

`devenv shell` provides `kubectl`, `kubeseal`, `kustomize`, `argocd`, `gh`.

Verify the controller is reachable and you are pointed at the right cluster:

```fish
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  -n kube-system rollout status deploy/sealed-secrets-controller
```

### Option A - new `turnstile-config` secret

Put the real values in shell variables first so they never appear in a command
line (`read -s` echoes nothing and does not enter history):

```fish
read -s -P "Turnstile SECRET key: " TURNSTILE_SECRET
read -P "Turnstile SITE key: " TURNSTILE_SITE
```

Seal:

```fish
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  create secret generic turnstile-config -n brdgme \
  --from-literal=TURNSTILE_SECRET_KEY=$TURNSTILE_SECRET \
  --from-literal=TURNSTILE_SITE_KEY=$TURNSTILE_SITE \
  --dry-run=client -o yaml \
  | kubeseal --kubeconfig ~/.kube/brdgme-kubeconfig.yaml --format yaml \
  > sealed-secrets/secrets/turnstile-config.yaml
```

Clear the variables from the session:

```fish
set -e TURNSTILE_SECRET
set -e TURNSTILE_SITE
```

Confirm the output looks like the others - `kind: SealedSecret`, `name:
turnstile-config`, `namespace: brdgme`, two entries under `encryptedData`, and a
`template.metadata` with the same name and namespace:

```fish
head -20 sealed-secrets/secrets/turnstile-config.yaml
```

Then, in `sealed-secrets/secrets/kustomization.yaml`, add
`- turnstile-config.yaml` to `resources:` in alphabetical position (after
`postgres-user.yaml`). Commit and push both changes to the config repo.

**Option A also needs a change in this repo**: add a fourth entry to
`envFrom` in `k8s/base/web/deployment.yaml`:

```yaml
            - secretRef:
                name: turnstile-config
```

then update the `ref=` sha in `brdgme-config/prod/kustomization.yaml` to the
merged commit. Until both land, the sealed secret exists but nothing reads it.

### Option B - fold into the existing `email-config`

Read the current plaintext from the live secret first (SealedSecret cannot merge
keys - you must re-seal all of them):

```fish
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  -n brdgme get secret email-config \
  -o 'go-template={{range $k, $v := .data}}{{$k}}={{$v | base64decode}}{{"\n"}}{{end}}'
```

That prints all three existing values. Then re-seal all five keys in one go:

```fish
read -s -P "RESEND_API_KEY: " RESEND_API_KEY
read -s -P "RESEND_WEBHOOK_SECRET: " RESEND_WEBHOOK_SECRET
read -s -P "Turnstile SECRET key: " TURNSTILE_SECRET
read -P "Turnstile SITE key: " TURNSTILE_SITE

kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  create secret generic email-config -n brdgme \
  --from-literal=EMAIL_FROM='brdg.me <mail@brdg.me>' \
  --from-literal=RESEND_API_KEY=$RESEND_API_KEY \
  --from-literal=RESEND_WEBHOOK_SECRET=$RESEND_WEBHOOK_SECRET \
  --from-literal=TURNSTILE_SECRET_KEY=$TURNSTILE_SECRET \
  --from-literal=TURNSTILE_SITE_KEY=$TURNSTILE_SITE \
  --dry-run=client -o yaml \
  | kubeseal --kubeconfig ~/.kube/brdgme-kubeconfig.yaml --format yaml \
  > sealed-secrets/secrets/email-config.yaml

set -e RESEND_API_KEY RESEND_WEBHOOK_SECRET TURNSTILE_SECRET TURNSTILE_SITE
```

Note `EMAIL_FROM` contains `<`, `>` and a space - **it must be single-quoted in
fish** exactly as shown, or the angle brackets are parsed as redirections.
Confirm the live value with the `go-template` command above before typing it;
`6e4e879` changed it and the value quoted here is from that commit message, not
from the cluster.

`email-config.yaml` is already in `kustomization.yaml`, so no kustomization edit
is needed. Commit and push. No change to this repo at all.

### Verifying the rollout

ArgoCD auto-syncs (`prune: true, selfHeal: true`), so pushing to `main` is
sufficient. To watch it:

```fish
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  -n brdgme get sealedsecret turnstile-config
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  -n brdgme get secret turnstile-config
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  -n brdgme rollout status deploy/web
```

If the SealedSecret exists but the Secret never appears, the seal failed to
decrypt - check the controller log:

```fish
kubectl --kubeconfig ~/.kube/brdgme-kubeconfig.yaml \
  -n kube-system logs deploy/sealed-secrets-controller --tail=50
```

The usual cause with strict scope is a name or namespace mismatch between the
`kubectl create secret` invocation and the SealedSecret's final location. There
is no `cluster-wide` annotation on any secret here, so `-n brdgme` and the exact
secret name are both load-bearing at seal time, not just at apply time.

### Things I could not determine from local files

- **`kubeseal`'s support for `--kubeconfig` in v0.36.0 is not verifiable from
  this repo.** kubeseal takes kubectl's standard client flags and should accept
  it, but no committed command in either repo passes it - every documented
  invocation is a bare `kubeseal --format yaml` relying on the ambient context.
  If `kubeseal --kubeconfig` errors with an unknown-flag message, the equivalent
  in fish is `set -x KUBECONFIG ~/.kube/brdgme-kubeconfig.yaml` before the
  pipeline. Flagging rather than guessing, since a kubeseal that silently
  contacts the wrong cluster produces a secret that never decrypts.
- **Live cluster state** - whether a manually-created unsealed Secret already
  exists in `brdgme`, and what the current `email-config` values actually are.
  Requires cluster reads, which are out of scope for this task.
- **Whether the local `brdgme-config` checkout matches remote `main`** - not
  fetched.
- **The controller's public cert is not committed**, so every seal requires live
  cluster access. There is no offline-seal path in this setup.

### Two incidental findings from this investigation

- `k8s/argocd/brdgme-app.yaml` in *this* repo is a stale duplicate pointing at
  `brdgme/brdgme` `master` `k8s/prod`. The live Application is
  `brdgme-config/argocd/brdgme-app.yaml`. `docs/BACKLOG.md:67` already lists
  deleting it.
- The config repo's README claims GitHub Actions in `brdgme/brdgme` pushes a
  commit to it on every `master` merge. **That is not implemented** - `ci.yml`
  has no deploy job and never mentions `brdgme-config`; `docs/DEPLOY.md`
  documents a manual edit of `prod/kustomization.yaml`. The `kubeconform` CI job
  only builds `k8s/prod` from this repo, so a new SealedSecret file gets no CI
  validation at all.
