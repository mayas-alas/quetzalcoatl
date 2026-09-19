use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInput {
    pub bundle: PathBuf,
    pub manifest_sha256: String,
    pub rootfs: PathBuf,
    pub rootfs_sha256: String,
}
