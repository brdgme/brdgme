# Existing resource - to be imported, not created. Values must match the
# live cluster exactly before `tofu import`; see infra/README.md.
#
# Encodes the Phase 14 prod prerequisite (docs/plan/14-drop-knative-gateway-api.md):
# cluster version >= 1.33 with VPC-native networking (vpc_uuid set), which is
# required for the managed Gateway API (GatewayClass) DOKS provides on
# Cilium.
resource "digitalocean_kubernetes_cluster" "brdgme" {
  name     = var.cluster_name
  region   = var.region
  vpc_uuid = digitalocean_vpc.brdgme.id

  # Placeholder - confirm the exact live patch version with
  # `doctl kubernetes options versions` / `doctl kubernetes cluster get
  # <cluster>` before import. Must be >= 1.33 per the Phase 14 prerequisite.
  version = "1.33.1-do.0"

  node_pool {
    name       = var.node_pool_name
    size       = var.node_pool_size
    node_count = var.node_pool_node_count
  }

  lifecycle {
    # node_pool changes (scaling, taints, etc.) are common day-to-day
    # operations that shouldn't require going through tofu; the pool is
    # still imported so the cluster resource itself has no drift.
    ignore_changes = [node_pool]
  }
}
