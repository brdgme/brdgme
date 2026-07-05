# 20: external-dns

**Status:** Superseded 2026-07-05 — folded into Phase 16 and Phase 21

**Original plan (retired):** run the external-dns controller in-cluster
(DigitalOcean provider, `gateway-httproute` source) to reconcile DNS
records from `HTTPRoute` hostnames automatically.

**Why retired:** external-dns removed its in-tree DigitalOcean provider in
v0.21.0 ("no new in-tree providers accepted, use the webhook system"). The
only DigitalOcean option is a third-party, externaldns-team-unreviewed
webhook (`amoniacou/external-dns-digitalocean-webhook`, 1 GitHub star,
"use at your own risk" per the official README) — not something to run in
the production DNS-write path. Moving the zone off DigitalOcean to a
provider with official in-tree support (Cloudflare, Route53, etc.) was
considered and rejected: the zone only just moved from Route53 to DO
nameservers this week (Phase 21), and a second nameserver cutover for this
alone isn't worth it.

Separately: every DNS record in this project (the Linode-migration legacy
records, the Resend records) is already managed directly as
`digitalocean_record` tofu resources in `infra/dns.tf` (Phase 21). The
automation external-dns would have added — no manual DO console edits at
cutover — is achieved just as well by adding the cutover hostname records
to that same file, since the cutover DNS flip is a rare, one-time,
human-supervised event, not a continuous reconciliation need. This also
avoids an always-on controller and a DNS-write-scoped token living in the
cluster.

**Where this work now lives:**
- `infra/dns.tf` (Phase 21): add A records for `brdg.me`, `legacy.brdg.me`,
  `api.brdg.me`, `ws.brdg.me` pointed at the Gateway's DO Load Balancer IP.
  The IP doesn't exist until the Gateway (Phase 14) is actually applied in
  prod, so this can't be written until cutover.
- Phase 16 (cutover): the "point `brdg.me` at the new system" cutover step
  becomes "get the Gateway's LB IP, add/update the records in
  `infra/dns.tf`, `tofu apply`" instead of a git-only HTTPRoute change.
