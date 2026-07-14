//! Hex encoding helpers for validated hash identifier types.

use strata_identifiers::{L2BlockId, RBuf32};

/// Converts a validated hash identifier to the full hex string used by the explorer API.
///
/// The output is non-truncated and follows the identifier's display convention:
/// OL identifiers use normal byte order, while Bitcoin [`RBuf32`] values use
/// Bitcoin explorer/bitcoin-cli display order.
pub(crate) trait DisplayHashHex {
    fn to_display_hex(&self) -> String;
}

impl DisplayHashHex for L2BlockId {
    fn to_display_hex(&self) -> String {
        full_hash_hex(self.as_ref())
    }
}

impl DisplayHashHex for RBuf32 {
    fn to_display_hex(&self) -> String {
        bitcoin_hash_hex(self.as_ref())
    }
}

/// Converts raw hash bytes into full lowercase hex.
fn full_hash_hex(bytes: &[u8; 32]) -> String {
    hex::encode(bytes)
}

/// Converts raw Bitcoin hash bytes into the explorer/bitcoin-cli display order.
///
/// Bitcoin txids and block hashes are conventionally displayed with the byte
/// order reversed from the internal hash byte order stored by [`strata_identifiers::RBuf32`].
fn bitcoin_hash_hex(bytes: &[u8; 32]) -> String {
    let mut display_bytes = *bytes;
    // Preserve Bitcoin explorer link compatibility by storing display-order hex.
    display_bytes.reverse();
    full_hash_hex(&display_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strata_identifiers::Buf32;

    fn ascending_hex32() -> String {
        (0u8..32).map(|byte| format!("{byte:02x}")).collect()
    }

    fn descending_hex32() -> String {
        (0u8..32).rev().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn l2_block_id_display_hex_preserves_byte_order() {
        let hex = ascending_hex32();
        let id = L2BlockId::from(Buf32::from_str(&hex).expect("test hash should parse"));

        assert_eq!(id.to_display_hex(), hex);
    }

    #[test]
    fn bitcoin_hash_display_hex_reverses_byte_order() {
        let internal_hex = ascending_hex32();
        let internal_hash = Buf32::from_str(&internal_hex).expect("test bitcoin hash should parse");
        let txid = RBuf32::from(*internal_hash.as_ref());

        assert_eq!(txid.to_display_hex(), descending_hex32());
    }
}
