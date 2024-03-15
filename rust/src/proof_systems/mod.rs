use ark_ff::{BigInteger, Field, FpParameters, PrimeField};
use num::BigUint;
use thiserror::Error;

pub mod curves;
pub mod signer;

/// Field helpers error
#[allow(missing_docs)]
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum FieldHelpersError {
    #[error("failed to deserialize field bytes")]
    DeserializeBytes,
    #[error("failed to deserialize field bits")]
    DeserializeBits,
    #[error("failed to decode hex")]
    DecodeHex,
    #[error("failed to convert BigUint into field element")]
    FromBigToField,
}

/// Result alias using [FieldHelpersError]
pub type Result<T> = std::result::Result<T, FieldHelpersError>;

/// Field element helpers
///   Unless otherwise stated everything is in little-endian byte order.
pub trait FieldHelpers<F> {
    /// Deserialize from bytes
    fn from_bytes(bytes: &[u8]) -> Result<F>;

    /// Deserialize from little-endian hex
    fn from_hex(hex: &str) -> Result<F>;

    /// Deserialize from bits
    fn from_bits(bits: &[bool]) -> Result<F>;

    /// Serialize to bytes
    fn to_bytes(&self) -> Vec<u8>;

    /// Serialize to hex
    fn to_hex(&self) -> String;

    /// Serialize to bits
    fn to_bits(&self) -> Vec<bool>;

    /// Serialize field element to a BigUint
    fn to_biguint(&self) -> BigUint
    where
        F: PrimeField,
    {
        BigUint::from_bytes_le(&self.to_bytes())
    }

    /// Create a new field element from this field elements bits
    fn bits_to_field(&self, start: usize, end: usize) -> Result<F>;

    /// Field size in bytes
    fn size_in_bytes() -> usize
    where
        F: PrimeField,
    {
        F::size_in_bits() / 8 + (F::size_in_bits() % 8 != 0) as usize
    }

    /// Get the modulus as `BigUint`
    fn modulus_biguint() -> BigUint
    where
        F: PrimeField,
    {
        BigUint::from_bytes_le(&F::Params::MODULUS.to_bytes_le())
    }
}

impl<F: Field> FieldHelpers<F> for F {
    fn from_bytes(bytes: &[u8]) -> Result<F> {
        F::deserialize(&mut &*bytes).map_err(|_| FieldHelpersError::DeserializeBytes)
    }

    fn from_hex(hex: &str) -> Result<F> {
        let bytes: Vec<u8> = hex::decode(hex).map_err(|_| FieldHelpersError::DecodeHex)?;
        F::deserialize(&mut &bytes[..]).map_err(|_| FieldHelpersError::DeserializeBytes)
    }

    fn from_bits(bits: &[bool]) -> Result<F> {
        let bytes = bits
            .iter()
            .enumerate()
            .fold(F::zero().to_bytes(), |mut bytes, (i, bit)| {
                bytes[i / 8] |= (*bit as u8) << (i % 8);
                bytes
            });

        F::deserialize(&mut &bytes[..]).map_err(|_| FieldHelpersError::DeserializeBytes)
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];
        self.serialize(&mut bytes)
            .expect("Failed to serialize field");

        bytes
    }

    fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    fn to_bits(&self) -> Vec<bool> {
        self.to_bytes().iter().fold(vec![], |mut bits, byte| {
            let mut byte = *byte;
            for _ in 0..8 {
                bits.push(byte & 0x01 == 0x01);
                byte >>= 1;
            }
            bits
        })
    }

    fn bits_to_field(&self, start: usize, end: usize) -> Result<F> {
        F::from_bits(&self.to_bits()[start..end]).map_err(|_| FieldHelpersError::DeserializeBits)
    }
}
