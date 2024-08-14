#[cfg(feature = "tokio")]
mod tests {
    use std::fs;

    use config_file_watch::{Builder, Context};

    fn loader(context: &mut Context) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        let path = context.path().unwrap();
        let contents = fs::read_to_string(path)?;
        let value = contents.parse::<i32>()?;
        Ok(value)
    }

    #[tokio::test]
    async fn should_create_watch_async() {
        let (tx, _rx) = tokio::sync::mpsc::channel(10);

        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("test");
        fs::write(&config_file, "1").unwrap();

        Builder::new()
            .watch_file(&config_file)
            .load(loader)
            .after_update(move |_context: &mut Context, value: _| {
                // Notify some other thread that the configuration has changed.
                tx.blocking_send(value).unwrap();
            })
            .build_async()
            .await
            .unwrap();
    }
}
