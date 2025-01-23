use mina_indexer::mina_blocks::v2::ActionState;
use quickcheck::{Arbitrary, Gen};

#[derive(Clone)]
pub struct TestGen<T>(pub T)
where
    T: Clone;

impl Arbitrary for TestGen<ActionState> {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut bytes = [0u8; 32];

        for byte in bytes.iter_mut() {
            *byte = u8::arbitrary(g);
        }

        Self(format!("0x{}", hex::encode(bytes)).into())
    }
}
