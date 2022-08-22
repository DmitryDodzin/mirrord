use mirrord_preview::connect;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut statuses = connect(
        "http://layer.preview.metalbear.co:3000".to_owned(),
        Some("dima".to_owned()),
    )
    .await?;

    while let Some(status) = statuses.recv().await {
        println!("{:?}", status);
    }

    Ok(())
}
