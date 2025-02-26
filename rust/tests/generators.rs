use mina_indexer::mina_blocks::v2::{ActionState, ZkappEvent};
use quickcheck::{Arbitrary, Gen};

#[derive(Clone)]
pub struct TestGen<T>(pub T)
where
    T: Clone;

////////////
// zkapps //
////////////

pub fn gen() -> quickcheck::Gen {
    quickcheck::Gen::new(1000)
}

impl Arbitrary for TestGen<ActionState> {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut bytes = [0u8; 32];

        for byte in bytes.iter_mut() {
            *byte = u8::arbitrary(g);
        }

        Self(format!("0x{}", hex::encode(bytes)).into())
    }
}

impl Arbitrary for TestGen<ZkappEvent> {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut bytes = [0u8; 32];

        for byte in bytes.iter_mut() {
            *byte = u8::arbitrary(g);
        }

        Self(format!("0x{}", hex::encode(bytes)).into())
    }
}
