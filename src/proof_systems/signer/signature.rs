use crate::proof_systems::{signer::pubkey::CurvePoint, FieldHelpers};
use ark_ec::AffineCurve;
use std::fmt;

/// Base field element type
pub type BaseField = <CurvePoint as AffineCurve>::BaseField;

/// Scalar field element type
pub type ScalarField = <CurvePoint as AffineCurve>::ScalarField;

/// Signature structure
#[derive(Clone, Eq, fmt::Debug, PartialEq)]
pub struct Signature {
    /// Base field component
    pub rx: BaseField,

    /// Scalar field component
    pub s: ScalarField,
}

impl Signature {
    /// Create a new signature
    pub fn new(rx: BaseField, s: ScalarField) -> Self {
        Self { rx, s }
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut rx_bytes = self.rx.to_bytes();
        let mut s_bytes = self.s.to_bytes();
        rx_bytes.reverse();
        s_bytes.reverse();

        write!(f, "{}{}", hex::encode(rx_bytes), hex::encode(s_bytes))
    }
}
