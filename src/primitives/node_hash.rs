use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy, PartialEq)]
pub struct NodeHash([u8; 32]);

impl Debug for NodeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}
impl Deref for NodeHash {
    fn deref(&self) -> &Self::Target {
        &self.0
    }
    type Target = [u8; 32];
}
impl DerefMut for NodeHash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Default for NodeHash {
    fn default() -> Self {
        NodeHash([0; 32])
    }
}
impl From<[u8; 32]> for NodeHash {
    fn from(value: [u8; 32]) -> Self {
        NodeHash(value)
    }
}

impl TryFrom<&[u8]> for NodeHash {
    type Error = String;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 32 {
            return Err(format!("Invalid length {}", value.len()));
        }
        let mut hash = NodeHash([0; 32]);
        hash.0.clone_from_slice(value);
        Ok(hash)
    }
}
impl<'a> TryFrom<&'a str> for NodeHash {
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if value.len() != 64 {
            return Err("Invalid length".into());
        }
        let hex = hex::decode(value);
        match hex {
            Ok(data) => Ok(data.as_slice().try_into().expect("We already checked it")),
            Err(e) => Err(format!("Invalid hex {e:?}")),
        }
    }
    type Error = String;
}
impl AsRef<[u8]> for NodeHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl Display for NodeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}
#[cfg(test)]
mod test {
    use std::ops::Deref;

    use crate::primitives::node_hash::NodeHash;

    #[test]
    fn test_display() {
        let hash = NodeHash::from([0; 32]);
        assert_eq!(
            format!("{hash}"),
            "0000000000000000000000000000000000000000000000000000000000000000"
        )
    }

    #[test]
    fn test_from_invalid_length_slice() {
        let res = NodeHash::try_from([0, 1, 2].as_slice());
        assert_eq!(res, Err("Invalid length 3".into()));
    }
    #[test]
    fn test_deref() {
        // This test is almost entirely compilation time, but just use it anyways
        let binding = NodeHash::from([0_u8; 32]);
        let de = binding.deref();
        assert_eq!(&[0; 32], de);
    }
    #[test]
    fn test_try_from_str_slice() {
        // echo Satoshi | sha256sum
        let hash = "fdd4d9893b23aa6cdb357e1606907c6909a1231595549e698f779a141d4534c7";
        let parsed = NodeHash::try_from(hash).expect("Valid hash");
        assert_eq!(hash.to_owned(), parsed.to_string());
    }
    #[test]
    fn test_try_from_invalid_str_slice() {
        // Invalid 'k' at pos 3
        let hash = "fdk4d9893b23aa6cdb357e1606907c6909a1231595549e698f779a141d4534c7";
        let parsed = NodeHash::try_from(hash);
        assert!(parsed.is_err());
    }

}
