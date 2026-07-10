terraform {
  required_version = ">= 1.7.0"

  required_providers {
    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.49"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5"
    }
  }

  # DO Spaces is S3-compatible; the skip_* flags disable AWS-only API calls
  # that Spaces doesn't implement. Requires AWS_ACCESS_KEY_ID /
  # AWS_SECRET_ACCESS_KEY env vars set to a Spaces access key/secret (not a
  # DigitalOcean API token - Spaces keys are generated separately).
  backend "s3" {
    bucket = "brdgme-tofu-state"
    key    = "brdgme/terraform.tfstate"
    region = "syd1"

    endpoints = {
      s3 = "https://syd1.digitaloceanspaces.com"
    }

    skip_credentials_validation = true
    skip_metadata_api_check     = true
    skip_region_validation      = true
    skip_requesting_account_id  = true
    skip_s3_checksum            = true
    use_path_style              = true
  }
}
