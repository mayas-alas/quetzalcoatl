use zeroize::Zeroizing;
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    ComputePassword,
    AccessEnrollment,
}
pub struct Secret {
    pub kind: SecretKind,
    pub value: Zeroizing<Vec<u8>>,
}
impl Secret {
    pub fn new(kind: SecretKind, value: Vec<u8>) -> Result<Self, String> {
        let value = Zeroizing::new(value);
        if !(16..=4096).contains(&value.len()) || value.iter().any(|b| *b < 33 || *b > 126) {
            return Err("SECRET_FORMAT_INVALID".into());
        }
        Ok(Self { kind, value })
    }
}
