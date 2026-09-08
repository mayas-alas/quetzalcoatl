use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedRelease {
    pub schema: u32,
    pub version: String,
    pub platform: String,
    pub access: String,
    pub dns: String,
    pub control: String,
    pub compute: String,
}
pub const DEFINITION: &str = include_str!("../../runtime/release.toml");
impl PinnedRelease {
    pub fn embedded() -> Result<Self, String> {
        let r: Self = toml::from_str(DEFINITION).map_err(|_| "RELEASE_SCHEMA_INVALID")?;
        if r.schema != 1 || r.version != env!("CARGO_PKG_VERSION") || r.platform != "linux-amd64" {
            return Err("RELEASE_SCHEMA_INVALID".into());
        }
        for image in [&r.access, &r.dns, &r.control, &r.compute] {
            let (_, digest) = image.split_once("@sha256:").ok_or("RELEASE_UNPINNED")?;
            if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("RELEASE_UNPINNED".into());
            }
        }
        Ok(r)
    }
}
impl crate::port::release::Release for PinnedRelease {
    fn verify(&self) -> Result<(), String> {
        Self::embedded().map(|_| ())
    }
}
