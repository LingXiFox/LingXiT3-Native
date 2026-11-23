use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use crate::RemoteError;

pub async fn connect_with_backoff(
    ws_url: &str,
    max_attempts: usize,
) -> Result<mpsc::UnboundedReceiver<String>, RemoteError> {
    let mut attempt = 0usize;
    let mut delay_ms = 100u64;

    loop {
        attempt += 1;
        match connect_async(ws_url).await {
            Ok((stream, _)) => {
                let (tx, rx) = mpsc::unbounded_channel();
                let (_write, mut read) = stream.split();
                tokio::spawn(async move {
                    while let Some(Ok(msg)) = read.next().await {
                        if let Ok(text) = msg.into_text() {
                            let _ = tx.send(text.to_string());
                        }
                    }
                });
                return Ok(rx);
            }
            Err(err) => {
                if attempt >= max_attempts {
                    return Err(RemoteError::Transport(format!(
                        "failed to connect after {attempt} attempts: {err}"
                    )));
                }
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(5_000);
            }
        }
    }
}
