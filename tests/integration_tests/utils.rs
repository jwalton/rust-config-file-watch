use std::{fs, path::PathBuf, thread, time::Duration};

pub fn create_files(
    files: &[(&str, &str)],
) -> Result<(tempfile::TempDir, Vec<PathBuf>), Box<dyn std::error::Error>> {
    let mut paths = vec![];

    let dir = tempfile::tempdir()?;
    for (name, contents) in files {
        let path = dir.path().join(name);
        fs::write(&path, contents)?;
        paths.push(path);
    }

    // Sleep to make tests deterministic. Without this, on MacOS if we create a
    // watch immediately after creating files, we sometimes get an event for each
    // created file, and sometimes we don't.
    thread::sleep(Duration::from_millis(100));

    Ok((dir, paths))
}
