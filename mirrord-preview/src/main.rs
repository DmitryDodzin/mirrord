use mirrord_preview::{connect, ProxiedRequest, ProxiedResponse};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, mut rx) = connect(
        "http://agent.preview.metalbear.co:3000".to_owned(),
        "dima".to_owned(),
        "abcd".to_owned(),
    )
    .await?;

    while let Some(ProxiedRequest {
        port,
        method: _,
        path,
        payload,
    }) = rx.recv().await
    {
        println!("got request to 127.0.0.1:{}{}", port, path);

        if let Err(_) = tx
            .send(ProxiedResponse {
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
