#[cfg(feature = "json")]
mod tests {
    use std::{fs, sync::mpsc};

    use config_file_watch::{Builder, Context, Watch};
    use serde::Deserialize;

    #[test]
    fn should_load_a_json_file() -> Result<(), Box<dyn std::error::Error>> {
        // TX and RX so we can signal when the value has changed.
        let (tx, rx) = mpsc::channel();

        // Struct for our JSON config file.
        #[derive(Debug, Deserialize)]
        struct ConfigFile {
            value: i32,
        }

        // Create a temporary folder and write the config file contents.
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config.json");
        fs::write(&config_file, r#"{"value": 1}"#).unwrap();

        // Create our watch.
        let watch: Watch<Option<ConfigFile>> = Builder::new()
            .watch_file(&config_file)
            .load_json()
            .after_update(move |_context: &mut Context, _value: _| {
                tx.send(()).unwrap();
            })
            .build()?;

        // Make sure the value was loaded correctly.
        rx.recv().unwrap();
        let guard = watch.value();
        match guard.as_ref() {
            Some(v) => assert_eq!(v.value, 1),
            None => panic!("Expected a value"),
        }

        // Update the config file.
        fs::write(&config_file, r#"{"value": 2}"#).unwrap();

        // Make sure we get our new value.
        rx.recv().unwrap();
        let guard_2 = watch.value();
        assert_eq!(guard_2.as_ref().as_ref().unwrap().value, 2);

        // Value in first guard should not have changed.
        assert_eq!(guard.as_ref().as_ref().unwrap().value, 1);

        Ok(())
    }

    #[test]
    fn should_load_a_json_file_with_default() -> Result<(), Box<dyn std::error::Error>> {
        // TX and RX so we can signal when the value has changed.
        let (tx, rx) = mpsc::channel();

        // Struct for our JSON config file.
        #[derive(Debug, Deserialize, Default)]
        struct ConfigFile {
            value: i32,
        }

        // Create a temporary folder and write the config file contents.
        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("config.json");
        fs::write(&config_file, r#"{"value": 1}"#).unwrap();

        // Create our watch.
        let watch: Watch<ConfigFile> = Builder::new()
            .watch_file(&config_file)
            .load_json()
            .after_update(move |_context: &mut Context, _value: _| {
                tx.send(()).unwrap();
            })
            .build()?;

        // Make sure the value was loaded correctly.
        rx.recv().unwrap();
        let config = watch.value();
        assert_eq!(config.value, 1);

        Ok(())
    }
}
