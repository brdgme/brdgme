# Encodes the Phase 14 prod prerequisite (docs/plan/14-drop-knative-gateway-api.md):
# cluster version >= 1.33 with VPC-native networking (vpc_uuid set), which is
# required for the managed Gateway API (GatewayClass) DOKS provides on
# Cilium.
#
# Cost posture (docs/plan/21-opentofu-iac.md): `ha = false` must be explicit -
# the HA control plane is $40/mo, and since DOKS 1.36.0 (May 2026) DO enables
# HA by default when the field is unset. HA cannot be disabled after creation,
# only avoided at create time. Single basic node pool, no autoscaling.
resource "digitalocean_kubernetes_cluster" "brdgme" {
  name     = var.cluster_name
  region   = var.region
  vpc_uuid = digitalocean_vpc.brdgme.id
  ha       = false

  version = var.cluster_version

  node_pool {
    name       = var.node_pool_name
    size       = var.node_pool_size
    node_count = var.node_pool_node_count
  }

  lifecycle {
    # Node scaling is a manual human decision (decided 2026-07-05) - pool
    # changes (scaling, taints, etc.) happen via doctl/console and must not
    # require going through tofu or be reverted by it.
    ignore_changes = [node_pool]
  }
}
