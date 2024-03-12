pub mod filesystem;

use std::path::PathBuf;

pub trait Parser {
    type Data;
    const KIND: &'static str;
    fn path(&self) -> PathBuf;
}

#[async_trait::async_trait]
pub trait Receiver<P: Parser> {
    async fn recv_data(&mut self) -> anyhow::Result<Option<P::Data>>;
}
