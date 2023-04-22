use std::time::Duration;

use futures_cancel::FutureExt;

#[tokio::main]
pub async fn main() {
    let shutdown_fut = tokio::time::sleep(Duration::from_millis(500));
    let handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
    })
    .cancel_with(shutdown_fut);
    println!("{:?}", handle.await);
}
