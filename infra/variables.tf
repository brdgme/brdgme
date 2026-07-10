variable "region" {
  description = "DigitalOcean region for all resources. ARCHITECTURE.md documents SYD1 as the region."
  type        = string
  default     = "syd1"
}

variable "cluster_name" {
  description = "Name of the DOKS cluster."
  type        = string
  default     = "brdgme"
}

variable "cluster_version" {
  description = "DOKS version slug. Must be >= 1.33 (Phase 14 Gateway API prerequisite). Check current slugs with `doctl kubernetes options versions`."
  type        = string
  default     = "1.36.0-do.2"
}

variable "vpc_name" {
  description = "Name of the VPC the cluster attaches to (VPC-native networking)."
  type        = string
  default     = "brdgme-vpc"
}

variable "node_pool_name" {
  description = "Name of the DOKS cluster's default node pool."
  type        = string
  default     = "brdgme-pool"
}

variable "node_pool_size" {
  description = "Droplet size slug for the default node pool. Basic (shared CPU) tier per the cost posture in docs/plan/21-opentofu-iac.md."
  type        = string
  default     = "s-2vcpu-4gb"
}

variable "node_pool_node_count" {
  description = "Initial node count. Ongoing scaling is manual via doctl/console; tofu ignores node_pool changes (see cluster.tf)."
  type        = number
  default     = 1
}

variable "domain_name" {
  description = "The DNS zone owned by this configuration (post-cutover records are managed by external-dns, phase 20)."
  type        = string
  default     = "brdg.me"
}

variable "cnpg_backup_bucket_name" {
  description = "Spaces bucket name for CloudNativePG (phase 19) Barman Cloud backups."
  type        = string
  default     = "brdgme-cnpg-backups"
}

variable "tofu_state_bucket_name" {
  description = "Spaces bucket name holding this configuration's own tofu state. Bootstrapped out of band, then imported. Must match the backend `bucket` in versions.tf."
  type        = string
  default     = "brdgme-tofu-state"
}

variable "cloudflare_account_id" {
  description = "Cloudflare account ID owning the brdg.me zone. Not secret (it appears in dashboard URLs); also in .env as CLOUDFLARE_ACCOUNT_ID for direct API calls outside tofu."
  type        = string
  default     = "cada680352b729d5b0c87470b05c55f7"
}
