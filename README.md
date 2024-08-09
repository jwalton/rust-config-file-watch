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
    let config_file = "./my-config.json";
    let default_config = ConfigFile { value: 7 };
    let watch: Watch<ConfigFile> =
        Builder::with_default(default_config).file(config_file).json()?;

    // Make sure the value was loaded correctly.
    let config = watch.value();
    println!("Value: {}", config.value);
}
```

Here we used `Builder::with_default()` to supply a default ConfigFile to use if the "my-config.json" file doesn't exist or can't be read. We could instead use `Builder::new()` to create a `Watch<Option<ConfigFile>>` with no default value. If you have a more complicated use case, you can call `Builder::build()` and pass in a closure to handle how to load your configuration file, or pass in a `Loader` object which will let you handle more complicated cases, like [tracking multiple files and their dependencies](./tests/dependencies.rs).
