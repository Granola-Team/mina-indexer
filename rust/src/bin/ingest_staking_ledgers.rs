use anyhow::Result;
use futures::future::try_join_all;
use log::{error, info};
use mina_indexer::{
    constants::CHANNEL_MESSAGE_CAPACITY,
    event_sourcing::{
        events::{Event, EventType},
        shared_publisher::SharedPublisher,
        staking_ledger_actors::subscribe_staking_actors,
    },
    utility::extract_height_and_hash,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{signal, sync::broadcast};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

    let staking_ledger_dir = std::env::var("STAKING_LEDGER_DIR")
        .map(PathBuf::from)
        .expect("STAKING_LEDGER_DIR environment variable must be present and valid");

    let shared_publisher = Arc::new(SharedPublisher::new(CHANNEL_MESSAGE_CAPACITY));

    let staking_ledgers = get_staking_ledgers(&staking_ledger_dir)?;

    let pre_fork_staking_ledger_hashes = get_prefork_staking_ledger_hashes();
    let mut pre_fork_staking_ledgers: Vec<PathBuf> = staking_ledgers
        .clone()
        .into_iter()
        .filter(|f| {
            let (_, hash) = extract_height_and_hash(f);
            pre_fork_staking_ledger_hashes.contains(hash)
        })
        .collect::<Vec<_>>();
    let mut post_fork_staking_ledgers: Vec<PathBuf> = staking_ledgers
        .into_iter()
        .filter(|f| {
            let (_, hash) = extract_height_and_hash(f);
            !pre_fork_staking_ledger_hashes.contains(hash)
        })
        .collect::<Vec<_>>();

    sort_entries(&mut pre_fork_staking_ledgers);
    sort_entries(&mut post_fork_staking_ledgers);

    let shared_publisher_clone = Arc::clone(&shared_publisher);

    let actors_handle = tokio::spawn(async move {
        if let Err(e) = subscribe_staking_actors(&shared_publisher, shutdown_receiver.resubscribe()).await {
            error!("Error in actor subscription: {:?}", e);
        }
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    for staking_ledger in pre_fork_staking_ledgers {
        shared_publisher_clone.publish(Event {
            event_type: EventType::PreForkStakingLedgerFilePath,
            payload: staking_ledger.to_str().unwrap().to_string(),
        });
        let (height, _) = extract_height_and_hash(staking_ledger.as_path());
        info!("Published Staking Ledger {height}");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    for staking_ledger in post_fork_staking_ledgers {
        shared_publisher_clone.publish(Event {
            event_type: EventType::PostForkStakingLedgerFilePath,
            payload: staking_ledger.to_str().unwrap().to_string(),
        });
        let (height, _) = extract_height_and_hash(staking_ledger.as_path());
        info!("Published Staking Ledger {height}");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }

    signal::ctrl_c().await?;
    info!("SIGINT received, sending shutdown signal...");

    // Send the shutdown signal
    let _ = shutdown_sender.send(());
    try_join_all([actors_handle]).await.unwrap();

    Ok(())
}

fn get_staking_ledgers(staking_ledgers_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries: Vec<PathBuf> = std::fs::read_dir(staking_ledgers_dir)?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.path())
        .collect();
    Ok(entries)
}

fn sort_entries(entries: &mut [PathBuf]) {
    entries.sort_by(|a, b| {
        let (a_num, a_hash) = extract_height_and_hash(a);
        let (b_num, b_hash) = extract_height_and_hash(b);

        a_num.cmp(&b_num).then_with(|| a_hash.cmp(b_hash))
    });
}

fn get_prefork_staking_ledger_hashes() -> HashSet<String> {
    [
        "x7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee",
        "x7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee",
        "wAAZcXndLYxb8w4LTU2d4K1qT3dL8Ck2jKzVEf9t9GAyXweQRG",
        "xRySSfk8kJZVj46zveaToDUJUC2GtprmeK7poqWymEzB6d2Tun",
        "wPwVsSPZ2tmmGbp8UrWGmDgFDrrzTPpYcjpWosckmcVZV2kcW7",
        "xVF5YbC3B5Rk6ibfsL97WaqojfxrgWtEqMJST9pb4X8s3kRD2T",
        "wJXdYzAikMHnTsw2kYyS1WQxJrGQsy1FKT5c18eHP2wGANafKf",
        "xQgtuyHp8nA2P6F9CSRrLcVeHi8Ap7wVHeNnH2UbSX15izcSHK",
        "xct9rteQ7wjhQVf7h4mGQmGZprJMkjbzEWgU7VvV6HEq2DN5yA",
        "xVLvFcBbRCDSM8MHLam6UPVPo2KDegbzJN6MTZWyhTvDrPcjYk",
        "jxhjiLBeMR7pgtV8ogcJvqXdr6asoNrC3g6hoUzEDLBSnZoxUDJ",
        "jx2XUFjvsvtTKB4HPAzih5boAtuoR34kxjEoU1RUhfXTATyx8tw",
        "jx4itrnmDkG3ptAiwhitJHt9K8stgFFoenrkZrm2prbtaS54xQU",
        "jwq7sAxDuN9MrdLjAQULoyrY5hWa6g52SVq8EmajBeBY38zamgz",
        "jxPj7F7aRew1zvpW9JaGSgt9xmJitenrRSM6YGKnuhe5HXqyZtZ",
        "jxn15ATGoe4WGgYpbssxJH9XW8NXRDy22WvSsBqvMqcnLPgPAwN",
        "jwAXd4GZgxE3YCwqs99g4MpLNiEV2ZfZPstyah4jxo753AVgL6R",
        "jwe63YTTUcc2b4sFdP54ehCZ3Dp9sZKshwCmtoVP3bidzfPfcxw",
        "jx5PU6GmyUqNuCnHNRF3pjHp7CTXXiCog4zJ1WcwHdyF3EJJ1Px",
        "jxos2foijoacWtcKdjzqwv2PrU7je8XFDnsSVNGgrgJaJLeA8VE",
        "jxBBSjakhQRKLbUM7z99KXNnMke2GbdcJyqpD9gyRoJJybsMRqh",
        "jxix1ap5gwXmiiwRqjijDv5KbHmnjAfj19CDywRLT1J8yTADcsT",
        "jwV7BsK9rBf5uRWqMZmWKVAUcEcd7pDAo9NCFTrvSvXRjHCwypF",
        "jwb5g4nyyMFvrXqN9wZLLb2TUx3Ux4gJ5F1k8Rt5nT9Eyaw9mZK",
        "jwHGGFvSf4BVMuQs65gXb384cGzdkbQDyr9rVUPnGDXa1kKJNne",
        "jx3Z9VyiCTMdif3cHZQVs1zfLKmkE8Z6N2CzTfDFi3gM6XyJaRa",
        "jxAqNQwwU31ez8JPg6aJxugdX4uYnKFwbWGjqRtAxkfBBsLf2gf",
        "jxsdc9d3AkKmVSWZQExucepfuLwfzQHtZpiCFArGqtfVe5jveiZ",
        "jx29wpTRDF8tuMFXgqT8inkJhb5chPjtZiwgTHzs6GxsvAy5KiH",
        "jxSi26fHMFyv8kxj4nBDDwi5FBt4oJummDjnfPodDbsNBzyjQdU",
        "jxDP6iJZGfNixGBiVasAnYYm1Fk29qWP2MecJ4mAg676DK7sQCM",
        "jwcWudRBTNZuMd1Tcyjzpr71buwc9RNmT2Jip1efA9eWvXcZiKL",
        "jwVvWi6GLeL6mz9jVFtD1HA7GNVuqe8tjFedisASfk8WVmdcfKE",
        "jw9ZJUdzn6NYSinWYuSEV27qKE2ZFXyvpzgxD5ZzsbyWYpeqnR8",
        "jxHoMZnbhR25patdD3SeNQPe3U9MPctcRSRvPw7p7rpTKcZLB6t",
        "jx1t9ivUkPJq9QfewYxFEc9GGLQVRZupDa9LRYFQeqpr9JPb1jj",
        "jwJLfz7Pqfr3eRDFAMDw5TJ4Q3aD7ZZpP8YWKdWWU2iHU137NUE",
        "jwpXcZgEcdvSswWPkaKYBcq1jfydzqitb87psbroyW6FSmjiSL8",
        "jwHyH1qgW4iBRHEJEDo4yaxMW82VgNCLmQwHNzVKSxTapisydbo",
        "jw9FBsiQK5uJVGd8nr333vvctg3hPKf5kZUHf7f5bnUojWyNt3Z",
        "jxxaCK9mMbnpCR3D5TS55Tit8y36E9jN8ER1P6Xry8TyHPYp1CY",
        "jwPQHxrJ94osTLCAiHYBuA6L4KGjkDV9t1A4mhdUoVEmbt2gxha",
        "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH",
        "jxRhDLj6Q62jjRDNS2yYtDu6yHziPx6yLNXvPdgMfZaF3NFvJic",
        "jxdhN2AXg5v3c6KbGdmNW58hvdiTVULhXF3yztD8CdKNnGdf3jp",
        "jxWMPncjMY9VhwehhVKHhobvJuAhZcdx5kfUtX4V3dy9Rw9aMZA",
        "jxQXzUkst2L9Ma9g9YQ3kfpgB5v5Znr1vrYb1mupakc5y7T89H8",
        "jwfyNt9AX6zRoWf67EcAzSQSDdLsS7Y8gZQPKmceCKo9C4hyKyX",
        "jxZGkwwaAEXdKaFB12jdxfedApFQ4gDJ58aiSjNw9VUffBgAmdg",
        "jwe5YREbjxzPCKe3eK7KfW5yXMdh71ca9mnMFfx9dBBQnRB6Rkb",
        "jxaswvEn5WF82AHLwbzMJN5Ty6RNAH9azqMV2R9q4sJStCpMp3w",
        "jwuGkeeB2rxs2Cr679nZMVZpWms6QoEkcgt82Z2jsjB9X1MuJwW",
        "jxWkqFVYsmQrXQZ2kkujynVj3TfbLfhVSgrY73CSVDpc17Bp3L6",
        "jxyqGrB5cSEavMbcMyNXhFMLcWpvbLR9a73GLqbTyPKVREkDjDM",
        "jx6taGcqX3HpWcz558wWNnJcne99jiQQiR7AnE7Ny8cQB1ASDVK",
        "jw8dXuUqXVgd6NvmpryGmFLnRv1176oozHAro8gMFwj8yuvhBeS",
        "jxXZTgUtCJmJnuwURmNMhoJWQ44X1kRLaKXtuYRFxnT9GFGSnnj",
        "jwgDB316LgQr15vmZYC5G9gjcizT5MPssZbQkvtBLtqpi93mbMw",
        "jwUe5igYAtQWZpcVYxt6xPJywnCZqDiNng9xZQLSZfZKpLZ61Hp",
        "jxffUAqcai9KoheQDcG46CCczjMRzFk61oXSokjaKvphicMpPj5",
        "jxKpSD4zcfKCSQQd3CG3yBqiesbUqm7eucRqLSvi9T1gUXtUPR5",
        "jxwahv5MsbGaUwSdAhyQA7Gr7atsyQbcju289PkoAnS4UgHGdce",
        "jy1jMBD7atiiheMxufJQDDfBuC2BjSXGj2HC5uSjXXsjAsGZt71",
        "jwbeXmeEZ2aYSyXnBbsMqWUxVDouwYZdzCqBejTaoecCkHoinPy",
        "jx4MPGB51t9MjrUh7NSsU6dLaouAb9bE2xu8b79kzmkEtKezwfw",
        "jxAzD4eVVmY4bFF9QnMrEmjG8rEXEgVCFbD4H85LVZu4c4Zmi9D",
        "jwvsYHPfACRUFYLL5NknBJc7zEY1q8t9rQfF8ek2pk2dUuKCz5J",
        "jxKCrryFrvzBE4iUURcS9zNTKcRdejiE9K28Bqcu7Us7RQqNfdL",
        "jxJbw37Kd7KxNvy5yd322NFwYZUdsXCeeEfjqGJ3cY9ukMmxBiW",
        "jxQwGGbtjRnhT1k7CqyASPKihyjFdtYSnJMANxdyWbHvGUofn8t",
        "jxw6YYsPFbC7bPqCcc6pVShATXbebaX1cxFqeV7Kyo1Pa5L3TU4",
        "jxiXyAr4NX6Ne1jxMU4WsiYc6SeBajSQZgmro9b63yDfQEeunD3",
        "jx4YTukDZVaFoiwYpKzzPmoCNzZgyXG1nHQkN7mwoJoB8aXMAmt",
        "jwyody4XQNTnGxkXQEKf87AN27wXadAjYgnGLAtvHahDkn2uWDU",
        "jxvumaCvujr7UzW1qCB87YR2RWu8CqvkwrCmHY8kkwpvN4WbTJn",
        "jx25quMPEvvipny2VxwDys5yCHaUL8oCMapfLv4eoRrsxEKm4pD",
        "jwqkCmcBJDi7XVRuW3dJpTGJ8ZbFeWo1iuzbQPmt536GeC5YChN",
        "jwqNEHtM8gmFAThkBWzU2DQiUuK1rW52Z8zsHyxMtwxCMovLu5K",
        "jxXwNfemxGwZcxKGhfrwzfE4QfxxGm5mkYieHQCafFkb6QBf9Xo",
        "jxxZUYeVFQriATHvBCrxmtfwtboFtMbXALVkE4y546MPy597QDD",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
