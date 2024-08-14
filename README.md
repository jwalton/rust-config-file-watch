# config-file-watch

config-file-watch is a library used for watching a configuration file or a set of configuration files, and automatically reloading files when they change. This can be used safely in sync or async code.

## How It Works

This uses the [notify](https://crates.io/crates/notify) library to watch files for changes in a background thread. Whenever a file changes, this will call into a user-defined loader function to load the contents of the file and store the resulting struct in an [ArcSwap](https://docs.rs/arc-swap/latest/arc_swap/index.html). If loading fails for any reason, the stored value remains unchanged. The loading function is called from the background thread, so is synchronous, but the library is safe to use in an async application.

If you're unfamiliar with `ArcSwap`, you can think of it a bit like an atomic `Arc`. When you call `watch.value()`, you get back a cheap copy of the Arc wrapping the config file. If the configuration file's contents change, the whole Arc is replaced. This means any tasks that already have a copy of the configuration will continue using the old version (until they call `watch.value()` again) but any new tasks will get the new configuration.

The library also supports callbacks for reacting to the config file changing to or errors.

## Usage

### Simple JSON Config File

The simplest usage is if you have configuration in a JSON or YAML file or similar. If we want to read in a JSON file, we can `cargo add config-file-watch -F json` and then:

```rs
use std::fs;

use config_file_watch::{Builder, Watch};
use serde::Deserialize;

// Struct for our JSON config file.
#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    value: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let watch: Watch<ConfigFile> = Builder::new()
        .watch_file(&config_file)
        .load_json()
        .on_error(|_context: &mut Context, err: _| {
            eprintln!("Error: {err}");
        })
        .build()?;

    // Make sure the value was loaded correctly.
    let config = watch.value();
    println!("Value: {}", config.value);

    Ok(())
}
```

The `watch` can be cloned and shared between threads. By default, this will debounce file events by 100ms. If the file doesn't exist, then `ConfigFile::default()` will be called to generate a default configuration. The watch actually watches not the file itself, but the parent directory of the file, so we can detect if the file gets created or deleted (although note that if other files in the same folder are frequently modified, this will consume a tiny amount of processing power, as we'll have to discard these changes). Also note that if the parent folder doesn't exist, creating the watch will fail.

By default, if an error occurs loading the file, the error will be printed to stderr, but you can override this behavior.

### No Default

Tnis example is the same as above, but we delcare the watch as a `Watch<Option<ConfigFile>>`. This means that if the file doesn't exist, or is removed, we'll replace the value with `None`. This also means our loader has to return an `Option<ConfigFile>`, but thankfully in the JSON case serde handles this for us.

```rs
let watch: Watch<Option<ConfigFile>> = Builder::new()
    .watch_file(&config_file)
    .load_json()
    .build()?;
```

### Reacting to Changes

The examples above are fine if you only ever need to passively read your config when performing an action (which is fine for many applications), but sometimes you may need to proactively update your running system when changes happen:

```rs
let (tx, rx) = mpsc::channel();

let watch: Watch<ConfigFile> = Builder::new()
    .watch_file(&config_file)
    .load_json()
    .after_update(move |_context: &mut Context, value: _| {
        // Notify some other thread that the configuration has changed.
        tx.send(value).unwrap();
    })
    .build()?;
```

### With Tokio

This example can be run by installing with `cargo add config-file-watch -F json -F tokio`:

```rs
let (tx, _rx) = tokio::sync::mpsc::channel(10);

let watch: Watch<ConfigFile> = Builder::new()
    .watch_file(&config_file)
    .load_json()
    .after_update(move |_context: &mut Context, value: _| {
        // Notify some other thread that the configuration has changed.
        tx.blocking_send(value).unwrap();
    })
    .build_async().await?;
```

Note that the functions passed to `load()`, `on_error()` or `after_update()` all get run in a blocking context in a background thread.

### Custom Loader

Here's another example, using a custom function to load the contents of the file:

```rs
let watch = Builder::new()
    .watch_file("./my-config")
    .load(
        |context: &mut Context| -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
            let path = context.path();
            let contents = fs::read_to_string(path)?;
            let value = contents.parse::<i32>()?;
            Ok(value)
        },
    )
    .build()
    .unwrap();
```

### Configuration Files With Dependencies

You can update which files are being watched via the context passed in to the loader. See [this example in the integration tests](https://github.com/jwalton/rust-config-file-watch/blob/master/tests/dependencies.rs).
