variable "region" {
  description = "DigitalOcean region for all resources. ARCHITECTURE.md documents SYD1 as the current region."
  type        = string
  default     = "syd1"
}

variable "cluster_name" {
  description = "Name of the existing DOKS cluster to import. Must match reality before `tofu import` - check with `doctl kubernetes cluster list`."
  type        = string
  default     = "brdgme"
}

variable "vpc_name" {
  description = "Name of the existing VPC to import. Must match reality before `tofu import` - check with `doctl vpcs list`."
  type        = string
  default     = "brdgme-vpc"
}

variable "node_pool_name" {
  description = "Name of the DOKS cluster's default node pool. Must match reality before `tofu import`."
  type        = string
  default     = "brdgme-pool"
}

variable "node_pool_size" {
  description = "Droplet size slug for the default node pool. Must match reality before `tofu import` - check with `doctl kubernetes cluster node-pool list <cluster>`."
  type        = string
  default     = "s-2vcpu-4gb"
}

variable "node_pool_node_count" {
  description = "Number of nodes in the default node pool. Must match reality before `tofu import`."
  type        = number
  default     = 1
}

variable "domain_name" {
  description = "The DNS zone owned by this configuration (records themselves are managed by external-dns, phase 20)."
  type        = string
  default     = "brdg.me"
}

variable "cnpg_backup_bucket_name" {
  description = "Spaces bucket name for CloudNativePG (phase 19) Barman Cloud backups. New resource, created by tofu."
  type        = string
  default     = "brdgme-cnpg-backups"
}

variable "tofu_state_bucket_name" {
  description = "Spaces bucket name holding this configuration's own tofu state. New resource, created by tofu. Must match the backend `bucket` in versions.tf."
  type        = string
  default     = "brdgme-tofu-state"
}
