wasip3::cli::command::export!(Smoke);

struct Smoke;

impl wasip3::exports::cli::run::Guest for Smoke {
    async fn run() -> Result<(), ()> {
        let (mut tx, rx) = wasip3::wit_stream::new();
        futures::join!(
            async { wasip3::cli::stdout::write_via_stream(rx).await.unwrap() },
            async {
                assert!(tx.write_all(b"WASI 0.3".to_vec()).await.is_empty());
                drop(tx);
            }
        );
        Ok(())
    }
}
