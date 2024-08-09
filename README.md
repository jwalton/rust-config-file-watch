# config-file-watch

Library used for watching a configuration file or a set of configuration files, and automatically reloading files when they change. This can be used safely in sync or async code.

## How It Works

This uses the [notify](https://crates.io/crates/notify) library to watch files for changes in a background thread. Whenever a file changes, this will call into a user-defined loader function to load the contents of the file and store the resulting struct in an [ArcSwap](https://docs.rs/arc-swap/latest/arc_swap/index.html). If loading fails for any reason, the stored value remains unchanged. The loading function is called from the background thread, so is synchronous, but the library is safe to use in an async application.

If you're unfamiliar with `ArcSwap`, you can think of it a bit like an atomic `Arc`. When you call `watch.value()`, you get back a cheap copy of the Arc wrapping the config file. If the configuration file's contents change, the whole Arc is replaced. This means any tasks that already have a copy of the configuration will continue using the old version (until they call `watch.value()` again) but any new tasks will get the new configuration.

## Usage

The simplest usage is if you have configuration in a JSON or YAML file or similar:

```rs
use std::fs;

use config_file_watch::{Builder, Watch};
use serde::Deserialize;

// Struct for our JSON config file.
#[derive(Debug, Deserialize)]
struct ConfigFile {
    value: i32,
}

fn use_config() {
    let default_config = ConfigFile { value: 7 };
    let watch: Watch<ConfigFile> = Builder::with_default(default_config)
        .file("./my-config.json")
        .json(|err| eprintln!("Error reading config: {err}"))?;

    // Make sure the value was loaded correctly.
    let config = watch.value();
    println!("Value: {}", config.value);
}
```

By default, this will debounce file events by 100ms. Here we used `Builder::with_default()` to supply a default ConfigFile to use if the "my-config.json" file doesn't exist or can't be read. We could instead use `Builder::new()` to create a `Watch<Option<ConfigFile>>` with no default value.

Here's another example, using a custom function to load the contents of the file:

```rs
let watch = Builder::new()
    .file("./my-config")
    .build(|res: Result<&Path, Error>| {
        match res {
            Ok(path) => {
                let contents = fs::read_to_string(path).unwrap();
                // TODO: Do things to `contents` here.
                let config = 0;

                // We return `Some` to update the config, or `None` to skip
                // an update.  Here, the watch is a `Watch<Option<i32>`, so we
                // return `Some(Some(config))` to update the config, or
                // `Some(None)` to clear the stored config value.
                Some(Some(config))
            }
            Err(err) => {
                println!("Error watching file: {err}");
                None
            }
        }
    })
    .unwrap();
```

If you have a more complicated use case, you can also pass a `Loader` to `Builder::build()` which will give you control over when the value is updated, and allow [tracking multiple files and their dependencies](./tests/dependencies.rs).

## Usage with Tokio

TODO
