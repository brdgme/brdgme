//! Kubernetes-independent GameVersion-to-database registration shared by the
//! operator reconciler and the local registration CLI, so both paths persist
//! identical rows from the same canonical manifest metadata.

mod manifest;
mod registration;

pub use manifest::{GameVersionManifest, GameVersionManifestSpec, ManifestError};
pub use registration::{
    Registration, RegistrationError, SetStats, bulk_set, mark_others_non_public, set_public, upsert,
};
