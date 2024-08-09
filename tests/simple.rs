use std::{fs, path::Path, thread, time::Duration};

use config_file_watch::{Builder, Error};

#[test]
fn should_create_default_file_watch() {
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");
    fs::write(&config_file, "1").unwrap();

    // Note the `watch` is a `Watch<Option<i32>>` because we didn't specify an initial value.
    let watch = Builder::new()
        .file(&config_file)
        .no_debounce()
        .build(|res: Result<&Path, Error>| {
            let path = res.unwrap();
            let contents = fs::read_to_string(path).unwrap();
            contents.parse::<i32>().map(Some).ok()
        })
        .unwrap();

    // Initial value should be 1, since the file already exists on disk.
    assert_eq!(watch.value().unwrap(), 1);

    fs::write(&config_file, "2").unwrap();
    thread::sleep(Duration::from_millis(100));
    assert_eq!(watch.value().unwrap(), 2);

    // Write a garbage value to the file.
    fs::write(&config_file, "foo").unwrap();

    // This shouldn't change the value.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(watch.value().unwrap(), 2);
}

#[test]
fn should_create_file_watch() {
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");
    fs::write(&config_file, "1").unwrap();

    // Note the `watch` is a `Watch<i32>` because we did specify an initial value.
    let watch = Builder::with_default(0)
        .file(&config_file)
        .no_debounce()
        .build(|path: Result<&Path, Error>| {
            let path = path.unwrap();
            let contents = fs::read_to_string(path).ok()?;
            contents.parse::<i32>().ok()
        })
        .unwrap();

    // Initial value should be 1, since the file already exists on disk.
    assert_eq!(**watch.value(), 1);
}

#[test]
fn should_create_a_watch_for_file_that_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");

    // Note the `watch` is a `Watch<Option<i32>>` because we didn't specify an initial value.
    let watch = Builder::default()
        .file(config_file)
        .no_debounce()
        .build(|path: Result<&Path, Error>| -> Option<Option<i32>> {
            let path = path.unwrap();
            let contents = fs::read_to_string(path).ok()?;
            Some(contents.parse().ok())
        })
        .unwrap();

    assert_eq!(**watch.value(), None);
}

#[test]
fn should_create_a_watch_no_file_and_initial_value() {
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("test");

    // Note the `watch` is a `Watch<i32>` because we did specify an initial value.
    let watch = Builder::with_default(0)
        .file(&config_file)
        .no_debounce()
        .build(|path: Result<&Path, Error>| -> Option<i32> {
            let path = path.unwrap();
            let contents = fs::read_to_string(path).ok()?;
            contents.parse().ok()
        })
        .unwrap();

    // Since the file doesn't exist, should start with our initial value.
    assert_eq!(**watch.value(), 0);

    // Create the file
    fs::write(&config_file, "1").unwrap();

    // Let the value be loaded.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(**watch.value(), 1);
}
