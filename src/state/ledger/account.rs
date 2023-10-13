use rust_decimal::Decimal;
use serde::{Deserialize, Serialize, Serializer, Deserializer, de::Visitor};

use super::PublicKey;

#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Hash, Serialize, Deserialize,
)]
pub struct Amount(pub u64);

#[derive(PartialEq, Eq, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Nonce(pub u32);

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Account {
    #[serde(serialize_with = "serialize_public_key")]
    #[serde(deserialize_with = "deserialize_public_key")]
    pub public_key: PublicKey,
    pub balance: Amount,
    pub nonce: Nonce,
    pub delegate: Option<PublicKey>,
}

impl Account {
    pub fn empty(public_key: PublicKey) -> Self {
        Account {
            public_key,
            balance: Amount::default(),
            nonce: Nonce::default(),
            delegate: None,
        }
    }

    pub fn from_deduction(pre: Self, amount: Amount) -> Option<Self> {
        if amount > pre.balance {
            None
        } else {
            Some(Account {
                public_key: pre.public_key.clone(),
                balance: pre.balance.sub(&amount),
                nonce: Nonce(pre.nonce.0 + 1),
                delegate: pre.delegate,
            })
        }
    }

    pub fn from_deposit(pre: Self, amount: Amount) -> Self {
        Account {
            public_key: pre.public_key.clone(),
            balance: pre.balance.add(&amount),
            nonce: Nonce(pre.nonce.0 + 1),
            delegate: pre.delegate,
        }
    }

    pub fn from_delegation(pre: Self, delegate: PublicKey) -> Self {
        Account {
            public_key: pre.public_key,
            balance: pre.balance,
            nonce: Nonce(pre.nonce.0 + 1),
            delegate: Some(delegate),
        }
    }
}

impl PartialOrd for Account {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.public_key.cmp(&other.public_key))
    }
}

impl Ord for Account {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.public_key.cmp(&other.public_key)
    }
}

const MINA_SCALE: u32 = 9;

fn nanomina_to_mina(num: u64) -> String {
    let mut dec = Decimal::from(num);
    dec.set_scale(MINA_SCALE).unwrap();
    let mut dec_str = dec.to_string();
    if dec_str.contains('.') {
        while dec_str.ends_with('0') {
            dec_str.pop();
        }
        if dec_str.ends_with('.') {
            dec_str.pop();
        }
    }
    dec_str
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pk = self.public_key.to_address();
        let delegate = self
            .delegate
            .as_ref()
            .map(|pk| pk.to_address())
            .unwrap_or(pk.clone());
        writeln!(f, "{{")?;
        writeln!(f, "  pk:       {pk}")?;
        writeln!(f, "  balance:  {}", nanomina_to_mina(self.balance.0))?;
        writeln!(f, "  nonce:    {}", self.nonce.0)?;
        writeln!(f, "  delegate: {delegate}")?;
        writeln!(f, "}}")
    }
}

#[test]
fn test_nanomina_to_mina_conversion() {
    let actual = 1_000_000_001;
    let val = nanomina_to_mina(actual);
    assert_eq!("1.000000001", val);

    let actual = 1_000_000_000;
    let val = nanomina_to_mina(actual);
    assert_eq!("1", val);
}

fn serialize_public_key<S>(public_key: &PublicKey, s: S) 
    -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where S: Serializer 
{
    let pub_key_address = public_key.to_address();
    s.serialize_str(&pub_key_address)
}

fn deserialize_public_key<'de, D>(deserializer: D) 
    -> Result<PublicKey, D::Error> where D: Deserializer<'de> 
{
    pub struct StringVisitor;

    impl <'de> Visitor<'de> for StringVisitor {
        type Value = PublicKey;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid Mina public key address")
        }

        fn visit_str<E>(self, v: &str) 
            -> Result<Self::Value, E> where E: serde::de::Error 
        {
            match PublicKey::from_address(v) {
                Err(e) => Err(E::custom(e.to_string())),
                Ok(public_key) => Ok(public_key)
            }
        }
    }

    deserializer.deserialize_any(StringVisitor)
}