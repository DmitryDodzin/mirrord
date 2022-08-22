use mirrord_preview::connect;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut statuses = connect(
        "http://layer.preview.metalbear.co:3000".to_owned(),
        "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiZW1haWwiOiJqb2huLmRvZUBnbWFpbC5jb20iLCJhZG1pbiI6dHJ1ZSwiaWF0IjoxNjYxMTg2MTMyLCJleHAiOjE2NjEyODYxMzJ9.fgAc9drKCqrgy4HDP4F8_3npCnqXCGL43VecE4STuA9YP-MIhZasrTZdi8URZqRyaTQu5ehOmHzrHdYKhJopwbsiWQTIdIOgvTq0m23gzw6dKSQUTCupedSOEWUpEydLZPpTCChDEw4cfpwtgmJU1V6ENwY8HRjeO4X6VhlSXz0zbF4DeyaUg0nET3bDzmv4jBWSnnO3X0YRn-NYGULYz0Sv2e_5X0if-TqS20E1CgquqKjTSp9k-QUGknx0mE38IK4IxSMM40aZ_7w0JqcRDJRzsgjalSmfzwl6Rv90h0HCXlDKa7UfvIU-fQgL3bCH-zjUcgABO77N16VAKmrlfg".to_owned(),
        Some("dima".to_owned()),
    )
    .await?;

    while let Some(status) = statuses.recv().await {
        println!("{:?}", status);
    }

    Ok(())
}
