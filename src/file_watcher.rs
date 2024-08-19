use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use arc_swap::ArcSwap;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{DebounceEventResult, Debouncer};

use crate::{Error, Guard};

/// Watches a set of files for changes.  This is essentially a thin wrapper around
/// `notify::RecommendedWatcher` which takes care of watching parent directories
/// instead of individual files, so we can be notified when files are created or
/// deleted.
#[derive(Debug)]
pub struct FileWatcher {
    watcher: Arc<Mutex<InnerWatcher>>,
    watched_files: Arc<ArcSwap<Vec<PathBuf>>>,
}

#[derive(Debug)]
enum InnerWatcher {
    Watcher(RecommendedWatcher),
    Debouncer(Debouncer<RecommendedWatcher>),
}

impl InnerWatcher {
    /// Get the underlying Watcher instance.
    fn watcher(&mut self) -> &mut dyn Watcher {
        match self {
            InnerWatcher::Watcher(w) => w,
            InnerWatcher::Debouncer(d) => d.watcher(),
        }
    }
}

impl FileWatcher {
    /// Create a new file watcher. This will watch the given set of files and
    /// call `on_change` whenever a file changes. Files do not have to exist at
    /// the time the FileWatcher is created; we will notify when files are
    /// created or deleted. The parent of the file DOES have to exist, however.
    pub fn create<FilesIter, Callback>(
        files: FilesIter,
        debounce: Option<Duration>,
        mut on_change: Callback,
    ) -> Result<Self, Error>
    where
        FilesIter: IntoIterator,
        FilesIter::Item: AsRef<Path>,
        Callback: (FnMut(Result<&[&Path], Error>)) + Send + 'static,
    {
        let watched_files: Arc<ArcSwap<Vec<PathBuf>>> = Arc::new(ArcSwap::from_pointee(vec![]));

        let watcher = {
            let watched_files = watched_files.clone();

            match debounce {
                None => InnerWatcher::Watcher(notify::recommended_watcher(
                    move |res: Result<Event, notify::Error>| match res {
                        Ok(event) => {
                            // Ignore any events not for our desired path.
                            let watched_files = watched_files.load();
                            let changed = matching_files(&watched_files, event.paths);
                            if !changed.is_empty() {
                                on_change(Ok(&changed));
                            }
                        }
                        Err(err) => {
                            on_change(Err(err.into()));
                        }
                    },
                )?),
                Some(debounce) => InnerWatcher::Debouncer(notify_debouncer_mini::new_debouncer(
                    debounce,
                    move |res: DebounceEventResult| match res {
                        Ok(events) => {
                            // Find the set of all files that have changed.
                            let watched_files = watched_files.load();
                            let changed_files = events.iter().map(|e| e.path.clone());
                            let changed = matching_files(&watched_files, changed_files);
                            if !changed.is_empty() {
                                on_change(Ok(&changed));
                            }
                        }
                        Err(err) => {
                            on_change(Err(err.into()));
                        }
                    },
                )?),
            }
        };

        let result = FileWatcher {
            watcher: Arc::new(Mutex::new(watcher)),
            watched_files,
        };

        let files: Vec<_> = files
            .into_iter()
            .map(|f| f.as_ref().to_path_buf())
            .collect();
        result.update_files(files)?;

        Ok(result)
    }

    /// Get the set of files this watcher is watching.
    pub fn watched_files(&self) -> Guard<Vec<PathBuf>> {
        self.watched_files.load()
    }

    /// Update the set of files this watcher is watching.
    pub fn update_files<I>(&self, files: I) -> Result<(), Error>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let files: Vec<_> = files
            .into_iter()
            .map(|f| f.as_ref().to_path_buf())
            .collect();

        let old_watched_files = self.watched_files.load();
        self.watched_files.store(Arc::new(files.clone()));

        {
            let old_folders = folders(&old_watched_files);
            let new_folders = folders(&files);
            let mut watcher_lock = self.watcher.lock().unwrap();
            let watcher = watcher_lock.watcher();

            // Note that instead of watching the files directly, we watch the
            // parent folder, so we can be notified if the file is created.
            let added_folders = new_folders.difference(&old_folders);
            for folder in added_folders {
                watcher.watch(folder, RecursiveMode::NonRecursive)?;
            }

            let removed_folders = old_folders.difference(&new_folders);
            for folder in removed_folders {
                let _ = watcher.unwatch(folder).ok();
            }
        }

        Ok(())
    }
}

/// Get the set of folders containing the given files.
fn folders(files: &[PathBuf]) -> HashSet<&Path> {
    files.iter().filter_map(|f| f.parent()).collect()
}

/// Returns the set of changed files that match files in `watched_files`.
fn matching_files<I>(watched_files: &Vec<PathBuf>, changed_files: I) -> Vec<&Path>
where
    I: IntoIterator,
    I::Item: AsRef<Path>,
{
    // Collect changes into a HashSet to deduplicate.
    changed_files
        .into_iter()
        .filter_map(|changed_file| {
            // We need to canonicalize the paths from the event here and from
            // the list of files to watch, since either could include
            // a symlink.
            if let Ok(event_path) = canonicalize(changed_file.as_ref()) {
                for file in watched_files {
                    if let Ok(file_path) = canonicalize(file) {
                        if event_path == file_path {
                            return Some(file as &Path);
                        }
                    }
                }
            }
            None
        })
        .collect()
}

fn canonicalize(path: &Path) -> std::io::Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(_) => {
            // If the file doesn't exist, canonicalize will fail. If the file is
            // removed, though, we still want to match it, so in this case we
            // canonicalize the parent path and add the filename in.
            match (path.parent(), path.file_name()) {
                (Some(parent), Some(file_name)) => {
                    // Canonicalize the parent path, then add in our path
                    let parent = parent.canonicalize()?;
                    let path = parent.join(file_name);
                    Ok(parent.join(path))
                }
                _ => Ok(path.to_owned()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use map_macro::hash_set;

    use super::*;
    use std::{fs, sync::mpsc, thread};

    #[test]
    fn should_watch_a_file() {
        let (tx, rx) = mpsc::channel();

        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("test");
        fs::write(&config_file, "test").unwrap();
        thread::sleep(Duration::from_millis(100));

        let _watcher = FileWatcher::create(
            &[&config_file],
            Some(Duration::from_millis(100)),
            move |res| {
                let files = res
                    .unwrap()
                    .iter()
                    .map(|f| f.to_path_buf())
                    .collect::<HashSet<_>>();
                tx.send(files).unwrap();
            },
        )
        .unwrap();

        fs::write(&config_file, "test2").unwrap();
        assert_eq!(rx.recv().unwrap(), hash_set![config_file.clone()]);

        fs::remove_file(&config_file).unwrap();
        assert_eq!(rx.recv().unwrap(), hash_set![config_file]);
    }

    #[test]
    fn should_debounce() {
        let (tx, rx) = mpsc::channel();

        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("test");
        let config_file2 = dir.path().join("test2");
        fs::write(&config_file, "1").unwrap();
        thread::sleep(Duration::from_millis(500));

        let _watcher = FileWatcher::create(
            &[&config_file, &config_file2],
            Some(Duration::from_millis(500)),
            move |res| {
                let files = res
                    .unwrap()
                    .iter()
                    .map(|f| f.to_path_buf())
                    .collect::<HashSet<_>>();
                tx.send(files).unwrap();
            },
        )
        .unwrap();

        fs::write(&config_file, "test2").unwrap();
        fs::write(&config_file, "test3").unwrap();
        fs::write(&config_file2, "test2").unwrap();

        // Should get one change event.
        assert_eq!(rx.recv().unwrap(), hash_set![config_file, config_file2]);
    }

    #[test]
    fn should_watch_a_file_that_does_not_exist() {
        let (tx, rx) = mpsc::channel();

        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("test");

        let _watcher = FileWatcher::create(&[&config_file], None, move |res| {
            let files = res
                .unwrap()
                .iter()
                .map(|res| res.to_path_buf())
                .collect::<HashSet<_>>();
            tx.send(files).unwrap();
        })
        .unwrap();

        fs::write(&config_file, "test").unwrap();
        fs::write(dir.path().join("test2"), "test").unwrap();
        fs::write(&config_file, "test").unwrap();

        // Should not get any events for `test2`, since we're only watching `test`.
        assert_eq!(rx.recv().unwrap(), hash_set![config_file.clone()]);
        assert_eq!(rx.recv().unwrap(), hash_set![config_file]);
    }

    #[test]
    fn should_switch_which_files_are_being_watched() {
        let (tx, rx) = mpsc::channel();

        let dir = tempfile::tempdir().unwrap();
        let config_file_a = dir.path().join("a");
        let config_file_b = dir.path().join("b");
        let config_file_c = dir.path().join("c");
        thread::sleep(Duration::from_millis(100));

        let watcher = FileWatcher::create(
            &[&config_file_a, &config_file_b],
            Some(Duration::from_millis(100)),
            move |res| {
                let files = res
                    .unwrap()
                    .iter()
                    .map(|f| f.to_path_buf())
                    .collect::<HashSet<_>>();
                tx.send(files).unwrap();
            },
        )
        .unwrap();

        fs::write(&config_file_a, "test").unwrap();
        assert_eq!(rx.recv().unwrap(), hash_set![config_file_a.clone()]);

        fs::write(&config_file_b, "test").unwrap();
        assert_eq!(rx.recv().unwrap(), hash_set![config_file_b.clone()]);

        thread::sleep(Duration::from_millis(100));
        watcher
            .update_files(&[&config_file_b, &config_file_c])
            .unwrap();

        // Writing to `a` should not generate an event, since we've updated the
        // file list.
        fs::write(&config_file_a, "test2").unwrap();
        fs::write(&config_file_c, "test3").unwrap();
        assert_eq!(rx.recv().unwrap(), hash_set![config_file_c.clone()]);
    }

    #[test]
    fn should_not_generate_event_when_adding_file() {
        let (tx, rx) = mpsc::channel();

        let dir = tempfile::tempdir().unwrap();
        let config_file = dir.path().join("a");

        let initial_paths: Vec<PathBuf> = vec![];
        let watcher = FileWatcher::create(initial_paths, None, move |res| {
            let files = res
                .unwrap()
                .iter()
                .map(|f| f.to_path_buf())
                .collect::<HashSet<_>>();
            tx.send(files).unwrap();
        })
        .unwrap();

        fs::write(&config_file, "test").unwrap();

        // Add a delay here to make this deterministic. Without this delay we
        // sometimes get the event, and sometimes don't.
        std::thread::sleep(Duration::from_millis(100));

        watcher.update_files([&config_file]).unwrap();

        rx.recv_timeout(Duration::from_millis(100)).unwrap_err();
    }
}
