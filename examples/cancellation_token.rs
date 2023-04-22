use std::time::Duration;

use futures_cancel::FutureExt;
use tokio_util::sync::CancellationToken;

#[tokio::main]
pub async fn main() {
    let cancellation_token = CancellationToken::default();
    let handle1 = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
    })
    .cancel_on_shutdown(&cancellation_token);

    let handle2 = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
    })
    .cancel_on_shutdown(&cancellation_token);

    let cancellation_token = cancellation_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        cancellation_token.cancel();
    });

    println!("{:?}", handle1.await);
    println!("{:?}", handle2.await);
}
