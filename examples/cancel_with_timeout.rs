use std::time::{Duration, Instant};

use futures_cancel::FutureExt;

#[tokio::main]
pub async fn main() {
    let start = Instant::now();
    let shutdown_fut = tokio::time::sleep(Duration::from_millis(500));
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
    })
    .cancel_with_timeout(shutdown_fut, Duration::from_secs(5));
    let res = handle.await;
    println!("cancelled after {:?}: {:?}", start.elapsed(), res);

    let start = Instant::now();
    let shutdown_fut = tokio::time::sleep(Duration::from_millis(500));
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
    })
    .cancel_with_timeout(shutdown_fut, Duration::from_secs(5));
    let res = handle.await;
    println!("completed after {:?}: {:?}", start.elapsed(), res);
}
