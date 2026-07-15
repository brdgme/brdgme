# Asset Caching (content-addressed /pkg assets)

**IMPLEMENTED 2026-07-15: content-addressed (hashed) filenames for Leptos
pkg assets, plus a matching Cloudflare cache rule** - pending review, not
yet accepted by Michael. Code changes are in the working tree; the
Cloudflare rule has not been applied via `tofu apply` yet.

## The bug and root cause

beta.brdg.me is proxied through Cloudflare (orange-cloud). Leptos pkg
assets were served at stable URLs (`/pkg/web.js`, `/pkg/web.wasm`) with no
`Cache-Control` header. Cloudflare's default cached-extension list includes
`.js` and `.css` but not `.wasm` or HTML.

After a deploy, browsers could fetch the new `web.wasm` from origin while
the edge kept serving the old `web.js` glue for up to Cloudflare's default
4h edge TTL. wasm-bindgen glue and its wasm are built together and must
match; a stale/new mismatch fails hydration with a `LinkError` in
production.

## Design (four coordinated pieces)

1. **`hash-files = true`** in `[package.metadata.leptos]`
   (`rust/web/Cargo.toml`). cargo-leptos 0.3.7 MD5-hashes each file under
   `site/pkg` after the build, renames it to `{stem}.{hash}.{ext}` (hash =
   unpadded base64url of the MD5 digest, 22 chars, per
   `cargo-leptos src/compile/hash.rs`), and writes `hash.txt` next to the
   server binary (not in site-root), with lines keyed by extension:
   `js: <hash>`, `wasm: <hash>`, `css: <hash>`. (Locally verified on
   cargo-leptos 0.3.6, the toolchain installed on this machine: identical
   filenames, hash.txt format, and md5sum+basenc-recomputed hashes.)

2. **Cache-Control middleware** (`rust/web/src/router.rs`,
   `set_cache_control`): `/pkg/*` responses get
   `public, max-age=31536000, immutable` (content-addressed, safe forever);
   `text/html` responses get `no-cache` (browsers revalidate, so a deploy
   atomically switches which hashed URLs a page references).

3. **Cloudflare cache rule** (`infra/cloudflare.tf`,
   `cloudflare_ruleset.cache_rules`, phase
   `http_request_cache_settings`): makes `/pkg/*` cache-eligible at the
   edge (covers `.wasm`, which the default extension list misses), with
   `edge_ttl` mode `respect_origin`, so the ~5.8MB wasm is served from the
   edge instead of origin on every request.

4. **`LEPTOS_HASH_FILES=true`** set as `ENV` in the final web image
   (`rust/Dockerfile`). leptos 0.8.20's `HydrationScripts` only reads
   `hash.txt` when this env var is set at runtime (cargo-leptos issue
   #347); it reads the file from `current_exe().parent()/hash.txt` on each
   request. If the env var or the file is missing, it silently falls back
   to unhashed names (logs an error for the script names, nothing fails
   loudly). `hash.txt` is `COPY`ed into the image beside the binary at
   `/app/hash.txt`.

## The wasm re-hash step in rust/Dockerfile

The Dockerfile runs a manual Sentry pipeline after
`cargo leptos build --release`: `wasm-bindgen --keep-debug`, `wasm-split`
to extract DWARF into `web.debug` and inject a `build_id` custom section,
then `wasm-opt` to size-optimize. That pipeline produces different wasm
bytes than the ones cargo-leptos already hashed, so shipping the
post-Sentry wasm under the cargo-leptos-assigned hashed name would be a
content/name mismatch, and `hash.txt` would point at the pre-Sentry wasm.

The Dockerfile therefore re-hashes the *final* wasm bytes with the same
MD5/base64url-unpadded scheme (coreutils `md5sum` + `basenc`), removes the
stale hashed wasm file, installs the final wasm under its true hashed
name, and rewrites the `wasm:` line of `hash.txt` with `sed`. Sentry
symbolication is unaffected because matching is by the injected `build_id`
custom section, not the filename.

## Deploy-ordering constraint

The app deploy that ships hashed filenames must be live before
`tofu apply` of the Cloudflare cache rule. Applying the rule while
`/pkg/web.wasm` is still a stable URL would edge-cache that wasm
long-term and reproduce the exact bug. The Terraform resource carries the
same warning comment.

With hashing live, the JS glue and wasm are fetched as a matched,
content-addressed pair - partial edge eviction can no longer mix versions
from different deploys.

## Alternatives considered

- **CI cache-purge step on deploy.** Deliberately excluded: purging is a
  mitigation, not a fix. Content-addressing removes the class of bug
  entirely rather than racing to invalidate stale entries.
- **ETag/conditional-request machinery for HTML.** Not built: `no-cache`
  plus SSR is sufficient, and the app had no existing ETag support to
  extend.
