# Existing resource - to be imported, not created. The zone belongs to
# tofu; records within it belong to external-dns (Phase 20) and are not
# managed here.
resource "digitalocean_domain" "brdgme" {
  name = var.domain_name
}
