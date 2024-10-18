//! Public key structures and algorithms
//!
//! Definition of public key structure and helpers

/// Affine curve point type
pub use crate::proof_systems::curves::pasta::Pallas as CurvePoint;
use crate::proof_systems::{signer::signature::BaseField, FieldHelpers};
use ark_ff::{BigInteger, PrimeField};
use sha2::{Digest, Sha256};
use std::ops::Neg;
use thiserror::Error;

/// Length of Mina addresses
const MINA_ADDRESS_LEN: usize = 55;
const MINA_ADDRESS_RAW_LEN: usize = 40;

/// Public key errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PubKeyError {
    /// Invalid address length
    #[error("invalid address length")]
    AddressLength,
    /// Invalid address base58
    #[error("invalid address base58")]
    AddressBase58,
    /// Invalid raw address bytes length
    #[error("invalid raw address bytes length")]
    AddressRawByteLength,
    /// Invalid address checksum
    #[error("invalid address checksum")]
    AddressChecksum,
    /// Invalid address version
    #[error("invalid address version")]
    AddressVersion,
    /// Invalid x-coordinate bytes
    #[error("invalid x-coordinate bytes")]
    XCoordinateBytes,
    /// Invalid x-coordinate
    #[error("invalid x-coordinate")]
    XCoordinate,
    /// Point not on curve
    #[error("point not on curve")]
    YCoordinateBytes,
    /// Invalid y-coordinate
    #[error("invalid y-coordinate bytes")]
    YCoordinateParityBytes,
    /// Invalid y-coordinate parity
    #[error("invalid y-coordinate parity bytes")]
    YCoordinateParity,
    /// Invalid y-coordinate parity
    #[error("invalid y-coordinate parity")]
    NonCurvePoint,
    /// Invalid hex
    #[error("invalid public key hex")]
    Hex,
    /// Invalid secret key
    #[error("invalid secret key")]
    SecKey,
}

/// Public key Result
pub type Result<T> = std::result::Result<T, PubKeyError>;

/// Public key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PubKey(CurvePoint);

impl PubKey {
    /// Create public key from curve point
    /// Note: Does not check point is on curve
    pub fn from_point_unsafe(point: CurvePoint) -> Self {
        Self(point)
    }

    /// Deserialize Mina address into public key
    ///
    /// # Errors
    ///
    /// Will give error if `address` string does not match certain requirements.
    pub fn from_address(address: &str) -> Result<Self> {
        if address.len() != MINA_ADDRESS_LEN {
            return Err(PubKeyError::AddressLength);
        }

        let bytes = bs58::decode(address)
            .into_vec()
            .map_err(|_| PubKeyError::AddressBase58)?;

        if bytes.len() != MINA_ADDRESS_RAW_LEN {
            return Err(PubKeyError::AddressRawByteLength);
        }

        let (raw, checksum) = (&bytes[..bytes.len() - 4], &bytes[bytes.len() - 4..]);
        let hash = Sha256::digest(&Sha256::digest(raw)[..]);
        if checksum != &hash[..4] {
            return Err(PubKeyError::AddressChecksum);
        }

        let (version, x_bytes, y_parity) = (
            &raw[..3],
            &raw[3..bytes.len() - 5],
            raw[bytes.len() - 5] == 0x01,
        );
        if version != [0xcb, 0x01, 0x01] {
            return Err(PubKeyError::AddressVersion);
        }

        let x = BaseField::from_bytes(x_bytes).map_err(|_| PubKeyError::XCoordinateBytes)?;
        let mut pt = CurvePoint::get_point_from_x(x, y_parity).ok_or(PubKeyError::XCoordinate)?;

        if pt.y.into_repr().is_even() == y_parity {
            pt.y = pt.y.neg();
        }

        if !pt.is_on_curve() {
            return Err(PubKeyError::NonCurvePoint);
        }

        // Safe now because we checked point pt is on curve
        Ok(PubKey::from_point_unsafe(pt))
    }

    /// Borrow public key as curve point
    pub fn point(&self) -> &CurvePoint {
        &self.0
    }

    /// Convert public key into curve point
    pub fn into_point(self) -> CurvePoint {
        self.0
    }

    /// Convert public key into compressed public key
    pub fn into_compressed(&self) -> CompressedPubKey {
        let point = self.0;
        CompressedPubKey {
            x: point.x,
            is_odd: point.y.into_repr().is_odd(),
        }
    }
}

fn into_address(x: &BaseField, is_odd: bool) -> String {
    let mut raw: Vec<u8> = vec![
        0xcb, // version for base58 check
        0x01, // non_zero_curve_point version
        0x01, // compressed_poly version
    ];

    // pub key x-coordinate
    raw.extend(x.to_bytes());

    // pub key y-coordinate parity
    raw.push(u8::from(is_odd));

    // 4-byte checksum
    let hash = Sha256::digest(&Sha256::digest(&raw[..])[..]);
    raw.extend(&hash[..4]);

    // The raw buffer is MINA_ADDRESS_RAW_LEN (= 40) bytes in length
    bs58::encode(raw).into_string()
}

/// Compressed public keys consist of x-coordinate and y-coordinate parity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompressedPubKey {
    /// X-coordinate
    pub x: BaseField,

    /// Parity of y-coordinate
    pub is_odd: bool,
}

impl CompressedPubKey {
    /// Serialize compressed public key into corresponding Mina address
    pub fn into_address(&self) -> String {
        into_address(&self.x, self.is_odd)
    }

    /// Deserialize Mina address into compressed public key (via an uncompressed
    /// `PubKey`)
    ///
    /// # Errors
    ///
    /// Will give error if `PubKey::from_address()` returns error.
    pub fn from_address(address: &str) -> Result<Self> {
        Ok(PubKey::from_address(address)?.into_compressed())
    }
}
