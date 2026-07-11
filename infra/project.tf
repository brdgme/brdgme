# The "brdgme" project groups this configuration's resources in the DO
# console. Created manually alongside the state bucket (2026-07-05), then
# imported here. VPCs are account-level and cannot be project-assigned.
resource "digitalocean_project" "brdgme" {
  name        = "brdgme"
  purpose     = "Web Application"
  environment = "Production"
}

# Assignment is one resource; it can only be created once every URN in it
# exists, so it lands with the stage-2 apply (the cluster). Until then the
# non-cluster resources sit in the account's default project.
resource "digitalocean_project_resources" "brdgme" {
  project = digitalocean_project.brdgme.id
  resources = [
    digitalocean_spaces_bucket.tofu_state.urn,
    digitalocean_spaces_bucket.cnpg_backups.urn,
    digitalocean_kubernetes_cluster.brdgme.urn,
  ]
}
