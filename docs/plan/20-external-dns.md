# 20: external-dns

**Status:** Pending

**Decision (2026-07-03 tech review):** manage DO DNS records from the
cluster with external-dns (DigitalOcean provider, `gateway-httproute`
source). Records for `brdg.me`/`legacy.`/`api.`/`ws.` follow the `HTTPRoute`
hostnames created in Phase 14, so the Phase 16 cutover and its rollback
become pure git operations - no manual edits in the DO control panel.

**Sequencing:** after Phase 14 (needs the Gateway + HTTPRoutes), ideally
after Phase 15 (the DO API token lands as a SealedSecret), and before the
Phase 16 cutover to deliver its value. Manifests are delegable; adoption of
the live DNS records is human-operated.

- [ ] `k8s/prod/external-dns/`: Deployment + RBAC. Args:
      `--provider=digitalocean`, `--source=gateway-httproute`,
      `--domain-filter=brdg.me`, `--registry=txt`,
      `--txt-owner-id=brdgme-prod`, `--policy=upsert-only` initially.
- [ ] DO API token (DNS-scoped) as a SealedSecret.
- [ ] Adopt existing manually created records deliberately: external-dns
      only manages records it owns via TXT registry entries. Audit the live
      zone for conflicts and take ownership record-by-record rather than
      letting the first sync surprise.
- [ ] Flip `--policy=upsert-only` → `sync` once ownership is verified, so
      deleted HTTPRoutes clean up their records (needed when Phase 16
      decommission removes `legacy.`/`api.`/`ws.`).
- [ ] Not used in dev (Kind uses lvh.me; nothing to reconcile).

