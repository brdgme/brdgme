# New resources, created via tofu (not imported).

# Phase 19 (CloudNativePG): Barman Cloud backup target.
resource "digitalocean_spaces_bucket" "cnpg_backups" {
  name   = var.cnpg_backup_bucket_name
  region = var.region
}

# This configuration's own tofu state (see the `s3` backend in versions.tf).
# Bootstrapping note: this bucket must exist before `tofu init` can use it as
# a backend, so on a fresh setup it needs to be created once with a local
# backend (or manually via `doctl`/the DO console) before switching to the
# S3 backend and importing it here.
resource "digitalocean_spaces_bucket" "tofu_state" {
  name   = var.tofu_state_bucket_name
  region = var.region
}
