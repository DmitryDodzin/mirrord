use mirrord_preview::{connect, ProxiedRequest, ProxiedResponse};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, mut rx) = connect(
        "http://layer.preview.metalbear.co:3000".to_owned(),
        Some("dima".to_owned()),
    )
    .await?;

    while let Some(ProxiedRequest {
        request_id,
        port,
        method: _,
        path,
        payload,
    }) = rx.recv().await
    {
        println!("got request to 127.0.0.1:{}{}", port, path);

        if let Err(_) = tx
            .send(ProxiedResponse {
                request_id,
                payload,
                status: 200,
            })
            .await
        {
            println!("send dropped");
        }
    }

    Ok(())
}
