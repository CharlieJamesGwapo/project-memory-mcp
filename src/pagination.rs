use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::error::{MemoryError, Result};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursor { pub v: u8, pub scope: String, pub after: i64, pub until: i64 }

impl Cursor {
    pub fn start(scope: &str, encoded: Option<&str>, high_water: i64) -> Result<Self> {
        let scope = format!("{:x}", Sha256::digest(scope.as_bytes()));
        if let Some(encoded) = encoded {
            if encoded.len() > 1024 { return Err(MemoryError::InvalidCursor); }
            let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| MemoryError::InvalidCursor)?;
            let value: Self = serde_json::from_slice(&bytes).map_err(|_| MemoryError::InvalidCursor)?;
            if value.v != 1 || value.scope != scope || value.until < 0 || value.until > high_water || value.after < 0 || value.after > value.until.saturating_add(1) {
                return Err(MemoryError::InvalidCursor);
            }
            Ok(value)
        } else { Ok(Self { v: 1, scope, after: high_water.saturating_add(1), until: high_water }) }
    }
    pub fn encode_after(&self, seq: i64) -> String {
        let value = Self { v: self.v, scope: self.scope.clone(), after: seq, until: self.until };
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&value).expect("cursor serialization cannot fail"))
    }
}
