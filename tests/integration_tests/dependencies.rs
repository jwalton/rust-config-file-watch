use std::{fs, path::PathBuf, sync::mpsc, thread, time::Duration};

use config_file_watch::{Builder, Context, Loader};
use serde::Deserialize;

use crate::utils::create_files;

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
        /// Value from this configuration file.
        value: i32,
        /// List of other configuration files to include.
        #[serde(default)]
        include: Vec<String>,
    }

    // The type for our loaded configuration.
    type ConfigValue = Vec<i32>;

    // Our custom loader
    struct ConfigLoader {
        config_file: PathBuf,
    }

    impl Loader<ConfigValue> for ConfigLoader {
        /// Called when a file changes.
        fn load(
            &mut self,
            context: &mut Context,
        ) -> Result<ConfigValue, Box<dyn std::error::Error + Send + Sync>> {
            // Build up a list of all the files we load as we go.
            let mut dependencies = vec![self.config_file.clone()];

            println!("Loading: {:?}", self.config_file);

            // Start by loading the main configuration file.
            let contents = fs::read_to_string(&self.config_file)?;
            println!("Loaded: {contents}");
            let main_config: ConfigFile = serde_json::from_str(&contents)?;
            println!("Including: {:?}", main_config.include);
            println!("Main value: {}", main_config.value);
            let mut values = vec![main_config.value];

            // For each included file, load it and add the value to the list.
            for include in main_config.include {
                let included_file = &self.config_file.parent().unwrap().join(&include);
                let include_config: ConfigFile =
                    serde_json::from_str(&fs::read_to_string(included_file)?)?;
                values.push(include_config.value);
                println!("{include} value: {}", include_config.value);

                // Add each included config file to the list of dependencies.
                dependencies.push(included_file.to_owned());
            }

            // Update the list of watched files via the context.
            if let Err(err) = context.update_watched_files(&dependencies) {
                println!("Error updating dependencies: {err:?}");
            }

            // Return the updated list of values.
            Ok(values)
        }
    }

    let (_guard, files) = create_files(&[
        (
            "file.json",
            r#"{
            "value": 1,
            "include": ["included_1.json", "included_2.json"]
        }"#,
        ),
        ("included_1.json", r#"{ "value": 2 }"#),
        ("included_2.json", r#"{ "value": 3 }"#),
    ])
    .unwrap();
    let main_config_file = &files[0];
    let included_1 = &files[1];
    let included_2 = &files[2];

    // Sleep to make this deterministic. Without this we sometimes
    // get a second set of events for the files we just created.
    thread::sleep(Duration::from_millis(100));

    // TX and RX so we can signal when the value has changed.
    let (tx, rx) = mpsc::channel();

    let watch = Builder::new()
        .watch_file(main_config_file)
        .load(ConfigLoader {
            config_file: main_config_file.clone(),
        })
        .after_update(move |_context: &mut Context, value: _| {
            println!("Updated: {value:?}");
            tx.send(()).unwrap();
        })
        .build()
        .unwrap();

    rx.recv().unwrap();
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
    fs::write(included_1, r#"{ "value": 5 }"#).unwrap();
    // Wait for the watcher to pick up the change.
    rx.recv().unwrap();
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
        main_config_file,
        r#"{
            "value": 1,
            "include": ["included_2.json"]
        }"#,
    )
    .unwrap();
    rx.recv().unwrap();
    assert_eq!(**watch.value(), vec![1, 3]);
    // Should no longer be watching the extra dependency.
    assert_eq!(
        **watch.watched_files(),
        vec![main_config_file.clone(), included_2.clone()]
    );
}
