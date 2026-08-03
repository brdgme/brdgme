use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

fn default_interface_version() -> i32 {
    1
}

/// Canonical GameVersion metadata shared by both registration paths: the
/// operator reads it from a GameVersion CRD and the local CLI reads it from
/// `k8s/base/game/<name>/game-version.yaml`. Field names, camelCase renames,
/// and defaults mirror the operator's `GameVersionSpec` so the same values
/// are never re-specified.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameVersionManifestSpec {
    /// Human-readable game type name stored in game_types.name.
    pub type_name: String,
    /// Game complexity weight (0.0 = light, 5.0 = heavy).
    #[serde(default)]
    pub weight: f32,
    /// Short 1-2 sentence description shown on the new game page.
    #[serde(default)]
    pub blurb: String,
    /// Deprecated versions cannot be used to start new games.
    #[serde(default)]
    pub is_deprecated: bool,
    /// Game interface version (1 = legacy, 2 = data docs + strategies).
    #[serde(default = "default_interface_version")]
    pub interface_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersionManifest {
    pub metadata: GameVersionManifestMetadata,
    pub spec: GameVersionManifestSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersionManifestMetadata {
    pub name: String,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to read manifest {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse manifest: {source}")]
    Parse {
        #[source]
        source: serde_yaml_ng::Error,
    },
}

impl GameVersionManifest {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(&path).map_err(|source| ManifestError::Io {
            path: path.as_ref().display().to_string(),
            source,
        })?;
        content.parse()
    }
}

impl std::str::FromStr for GameVersionManifest {
    type Err = ManifestError;

    fn from_str(content: &str) -> Result<Self, Self::Err> {
        serde_yaml_ng::from_str(content).map_err(|source| ManifestError::Parse { source })
    }
}
