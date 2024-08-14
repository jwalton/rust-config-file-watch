use std::{fs, sync::mpsc, thread, time::Duration};

use config_file_watch::{Builder, Context};

fn loader(context: &mut Context) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
    match context.path() {
        Some(path) => {
            let contents = fs::read_to_string(path)?;
            let value = contents.parse::<i32>()?;
            Ok(value)
        }
        None => Ok(0),
    }
}

fn option_loader(
    context: &mut Context,
) -> Result<Option<i32>, Box<dyn std::error::Error + Send + Sync>> {
    match context.path() {
        Some(path) => match fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents.parse::<i32>()?)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(Box::new(err))
                }
            }
        },
        None => Ok(None),
    }
}

#[test]
fn should_create_file_watch_with_default_value() {
    // tx and rx so we can signal when the value has changed.
    let (tx, rx) = mpsc::channel();

    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");
    fs::write(&config_file, "1").unwrap();

    let watch = Builder::new()
        .watch_file(&config_file)
        .load(loader)
        .after_update(move |_context: &mut Context, value: _| {
            tx.send(value).unwrap();
        })
        .build()
        .unwrap();

    // Initial value should be 1, since the file already exists on disk.
    rx.recv().unwrap();
    assert_eq!(**watch.value(), 1);

    // Update the file.
    fs::write(&config_file, "2").unwrap();
    rx.recv().unwrap();
    assert_eq!(**watch.value(), 2);

    // Write a garbage value to the file.
    fs::write(&config_file, "foo").unwrap();
    // This shouldn't change the value.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(**watch.value(), 2);

    // Remove the file.
    fs::remove_file(&config_file).unwrap();
    // This shouldn't change the value.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(**watch.value(), 2);
}

#[test]
fn should_create_watch_with_no_watched_files() {
    // tx and rx so we can signal when the value has changed.
    let (tx, rx) = mpsc::channel();

    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");
    fs::write(&config_file, "1").unwrap();

    let watch = Builder::new()
        .load(loader)
        .after_update(move |_context: &mut Context, value: _| {
            tx.send(value).unwrap();
        })
        .build()
        .unwrap();

    // Initial value should be 0, since we aren't watching any files.
    rx.recv().expect("Expected after_update for initial value");
    assert_eq!(**watch.value(), 0);

    // Add the file to the list of watched files.
    println!("Updating watched files: {:?}", config_file);
    watch.update_watched_files([&config_file]).unwrap();

    // Add a delay here to make this deterministic.
    thread::sleep(Duration::from_millis(100));

    // Update the file.
    fs::write(&config_file, "2").unwrap();
    rx.recv()
        .expect("Expected after_update after updating watch list");
    assert_eq!(**watch.value(), 2);
}

#[test]
fn should_create_file_watch_with_optional_value() {
    // tx and rx so we can signal when the value has changed.
    let (tx, rx) = mpsc::channel();

    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");
    fs::write(&config_file, "1").unwrap();

    // Note the `watch` is a `Watch<Option<i32>>`.
    let watch = Builder::new()
        .watch_file(&config_file)
        .load(option_loader)
        .after_update(move |_context: &mut Context, value: _| {
            tx.send(value).unwrap();
        })
        .build()
        .unwrap();

    // Initial value should be 1, since the file already exists on disk.
    rx.recv().unwrap();
    assert_eq!(**watch.value(), Some(1));

    // Update the file.
    fs::write(&config_file, "2").unwrap();
    rx.recv().unwrap();
    thread::sleep(Duration::from_millis(100));
    assert_eq!(**watch.value(), Some(2));

    // Write a garbage value to the file.
    fs::write(&config_file, "foo").unwrap();
    // This shouldn't change the value.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(**watch.value(), Some(2));

    // Remove the file.
    fs::remove_file(&config_file).unwrap();
    rx.recv().unwrap();
    assert_eq!(**watch.value(), None);
}

#[test]
fn should_create_a_watch_for_file_that_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");

    // Note the `watch` is a `Watch<Option<i32>>` because we didn't specify an initial value.
    let watch = Builder::default()
        .watch_file(config_file)
        .load(option_loader)
        .build()
        .unwrap();

    assert_eq!(**watch.value(), None);
}
