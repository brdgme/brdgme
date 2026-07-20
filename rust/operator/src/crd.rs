use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn default_interface_version() -> i32 {
    1
}

#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[kube(
    group = "brdgme.com",
    version = "v1",
    kind = "GameVersion",
    namespaced,
    status = "GameVersionStatus",
    printcolumn = r#"{"name":"Display Name","type":"string","jsonPath":".spec.typeName"}"#,
    printcolumn = r#"{"name":"Players","type":"string","jsonPath":".spec.playerCounts"}"#
)]
pub struct GameVersionSpec {
    /// Human-readable game type name stored in game_types.name (e.g. "Acquire").
    pub type_name: String,
    /// Game complexity weight (0.0 = light, 5.0 = heavy).
    #[serde(default)]
    pub weight: f32,
    /// Short 1-2 sentence description shown on the new game page.
    #[serde(default)]
    pub blurb: String,
    /// Deprecated versions cannot be used to start new games but remain running
    /// to serve existing in-progress games.
    #[serde(default)]
    pub is_deprecated: bool,
    /// Game interface version (1 = legacy, 2 = data docs + strategies).
    #[serde(rename = "interfaceVersion", default = "default_interface_version")]
    pub interface_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameVersionStatus {
    pub ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_generation: Option<i64>,
}
