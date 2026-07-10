variable "do_token" {
  description = "DigitalOcean API token. Prefer the DIGITALOCEAN_TOKEN env var over setting this directly."
  type        = string
  sensitive   = true
  default     = null
}

provider "digitalocean" {
  token = var.do_token
}

# Auth via the native CLOUDFLARE_API_TOKEN env var (set in .env,
# exported by devenv's dotenv integration on shell entry) - never a tofu
# variable, never committed. Scoped to the brdg.me zone: Zone.DNS Edit +
# Zone.Zone Settings Edit + Zone.Zone Read (spec W2).
provider "cloudflare" {}
