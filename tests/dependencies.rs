use std::{fs, path::PathBuf, thread, time::Duration};

use config_file_watch::{Builder, Context, Loader};
use serde::Deserialize;

/// This tests the case where we have some configuration file that includes other
/// configuration files, and we want to reload if any of the affected files change.
///
/// The idea here is that we have a main configuration file that includes two other
/// files. We implement a custom loader that adds these files as "dependencies"
/// to the `Loaded` result, which will cause the watch to start watching these
/// files as well.
#[test]
fn should_handle_dependencies() {
    // The format for our config files.
    #[derive(Debug, Deserialize)]
    struct ConfigFile {
        value: i32,
        #[serde(default)]
        include: Vec<String>,
    }

    // Our custom loader
    struct ConfigLoader {
        config_file: PathBuf,
    }

    impl ConfigLoader {
        /// Returns the configuration, and a list of files that were loaded.
        fn load(&self) -> anyhow::Result<(Vec<i32>, Vec<PathBuf>)> {
            // Build up a list of all the files we loaded.
            let mut dependencies = vec![self.config_file.clone()];

            // Start by loading the main configuration file.
            let main_config: ConfigFile =
                serde_json::from_str(&fs::read_to_string(&self.config_file)?)?;

            // For each included file, load it and add the value to the list.
            let mut values = vec![main_config.value];
            for include in main_config.include {
                let included_file = &self.config_file.parent().unwrap().join(include);
                let include_config: ConfigFile =
                    serde_json::from_str(&fs::read_to_string(included_file)?)?;
                values.push(include_config.value);

                // Add each included config file to the list of dependencies.
                dependencies.push(included_file.to_owned());
            }

            Ok((values, dependencies))
        }
    }

    impl Loader<Vec<i32>> for ConfigLoader {
        /// Called when a file changes.
        fn files_changed(&mut self, context: &mut Context<Vec<i32>>) {
            match self.load() {
                Ok((new_value, dependencies)) => {
                    // Update the value stored in the watch.
                    context.update_value(new_value);

                    // Update the list of watched files.
                    if let Err(err) = context.update_watched_files(&dependencies) {
                        println!("Error updating dependencies: {err:?}");
                    }
                }
                Err(err) => {
                    println!("Error loading config: {err:?}");
                }
            }
        }
    }

    let dir = tempfile::tempdir().unwrap();

    let main_config_file = dir.path().join("file.json");
    fs::write(
        &main_config_file,
        r#"{
            "value": 1,
            "include": ["included_1.json", "included_2.json"]
        }"#,
    )
    .unwrap();

    let included_1 = dir.path().join("included_1.json");
    fs::write(&included_1, r#"{ "value": 2 }"#).unwrap();

    let included_2 = dir.path().join("included_2.json");
    fs::write(&included_2, r#"{ "value": 3 }"#).unwrap();

    let watch = Builder::with_default(Vec::new())
        .file(&main_config_file)
        .no_debounce()
        .build(ConfigLoader {
            config_file: main_config_file.clone(),
        })
        .unwrap();

    assert_eq!(**watch.value(), vec![1, 2, 3]);
    assert_eq!(
        **watch.watched_files(),
        vec![
            main_config_file.clone(),
            included_1.clone(),
            included_2.clone()
        ]
    );

    // Update one of the dependencies.
    fs::write(&included_1, r#"{ "value": 5 }"#).unwrap();

    // Wait for the watcher to pick up the change.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(**watch.value(), vec![1, 5, 3]);
    assert_eq!(
        **watch.watched_files(),
        vec![
            main_config_file.clone(),
            included_1.clone(),
            included_2.clone()
        ]
    );

    // Remove one of the included files.
    fs::write(
        &main_config_file,
        r#"{
            "value": 1,
            "include": ["included_2.json"]
        }"#,
    )
    .unwrap();

    thread::sleep(Duration::from_millis(100));
    assert_eq!(**watch.value(), vec![1, 3]);
    // Should no longer be watching the extra dependency.
    assert_eq!(
        **watch.watched_files(),
        vec![main_config_file.clone(), included_2.clone()]
    );
}
