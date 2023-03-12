//! This module represents a key in the key-value store. A key is just a [u8; 32], where
//! the i-th bit represents whether we should look at the left or at the right sibling
//! to precess a path. For optimization, it's internally stored as two limbs of 128 bits,
//! so we don't have to loop too much

use std::ops::Deref;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key([u8; 32]);

impl Key {
    /// Tells which node should we take while descending into a tree (left or right sibling)
    pub fn bit_index(&self, i: u8) -> bool {
        // which u8 should we take? We simple look at integer division of i by 8
        let limb = i / 8;
        let mask = 1 << (i % 8);
        return (self.0[limb as usize] & mask) > 0;
    }
}

impl From<[u8; 32]> for Key {
    fn from(value: [u8; 32]) -> Self {
        Key(value)
    }
}

impl Deref for Key {
    fn deref(&self) -> &Self::Target {
        &self.0
    }
    type Target = [u8; 32];
}
impl From<i32> for Key {
    fn from(value: i32) -> Self {
        let bytes = value.to_le_bytes();
        let mut key = Key([0; 32]);
        key.0[0..4].clone_from_slice(&bytes);
        key
    }
}
#[cfg(test)]
mod test {
    use super::Key;

    #[test]
    fn test_bit_index() {
        // 0111 0100
        let mut expected = 0x74;
        let key = Key([0x74; 32]);
        for i in 0..=255 {
            // We reset this for each byte
            if i % 8 == 0 {
                expected = 0x74;
            }
            if expected & 1 == 1 {
                assert!(key.bit_index(i));
            } else {
                assert!(!key.bit_index(i));
            }
            expected >>= 1;
        }
    }
}
