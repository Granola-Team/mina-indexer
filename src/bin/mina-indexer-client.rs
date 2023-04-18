use clap::Parser;
use futures::{
    io::{AsyncWriteExt, BufReader},
    AsyncReadExt,
};
use interprocess::local_socket::tokio::LocalSocketStream;
use mina_indexer::{
    block::{precomputed::PrecomputedBlock, Block},
    state::ledger::account::Account,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub enum ClientCli {
    BestChain,
    Balance(AccountArgs),
}

#[derive(clap::Args, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AccountArgs {
    public_key: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let conn = LocalSocketStream::connect(mina_indexer::SOCKET_NAME).await?;
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);

    let mut buffer = Vec::with_capacity(1280000);

    match ClientCli::parse() {
        ClientCli::BestChain => {
            writer.write_all(b"best_chain\0").await?;
            dbg!(reader.read_to_end(&mut buffer).await?);
            let blocks: Vec<PrecomputedBlock> = bcs::from_bytes(&buffer)?;
            blocks.iter().for_each(|block| {
                println!(
                    "{:?}",
                    Block::from_precomputed(block, block.blockchain_length.unwrap())
                )
            });
        }
        ClientCli::Balance(account_args) => {
            let command = format!("account_balance {}\0", account_args.public_key);
            writer.write_all(command.as_bytes()).await?;
            reader.read_to_end(&mut buffer).await?;
            let account: Account = bcs::from_bytes(&buffer)?;
            println!("{account:?}");
        }
    }

    Ok(())
}
