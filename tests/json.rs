use std::fs;

use config_file_watch::{Builder, Watch};
use serde::Deserialize;

#[cfg(feature = "json")]
#[test]
fn should_load_a_json_file() -> Result<(), Box<dyn std::error::Error>> {
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
    let watch: Watch<Option<ConfigFile>> = Builder::default()
        .file(&config_file)
        .no_debounce()
        .json(|err| eprintln!("Error reading config: {err}"))?;

    // Make sure the value was loaded correctly.
    let guard = watch.value();
    match guard.as_ref() {
        Some(v) => assert_eq!(v.value, 1),
        None => panic!("Expected a value"),
    }

    // Update the config file.
    fs::write(&config_file, r#"{"value": 2}"#).unwrap();

    // Make sure we get our new value.
    std::thread::sleep(std::time::Duration::from_millis(100));
    let guard_2 = watch.value();
    assert_eq!(guard_2.as_ref().as_ref().unwrap().value, 2);

    // Value in first guard should not have changed.
    assert_eq!(guard.as_ref().as_ref().unwrap().value, 1);

    Ok(())
}
