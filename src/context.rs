use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use arc_swap::ArcSwap;

use crate::{Error, WeakFileWatcher};

/// This enum controls how we update the watched paths. Before we create the FileWatcher,
/// we can update the paths by adding them to the vector. After we create the FileWatcher,
/// we want to call into the FileWatcher to update paths.
enum Paths<'a> {
    Watcher(&'a WeakFileWatcher),
    Vector(&'a mut Vec<PathBuf>),
}

/// Context is used to control the Watch from within the loader.
pub struct Context<'a, T> {
    modified_paths: &'a [&'a Path],
    value: &'a ArcSwap<T>,
    paths: Paths<'a>,
}

impl<'a, T> Context<'a, T> {
    pub(crate) fn for_paths(
        modified_paths: &'a [&'a Path],
        value: &'a ArcSwap<T>,
        watch_paths: &'a mut Vec<PathBuf>,
    ) -> Self {
        Self {
            modified_paths,
            value,
            paths: Paths::Vector(watch_paths),
        }
    }

    pub(crate) fn for_watch(
        modified_paths: &'a [&'a Path],
        value: &'a ArcSwap<T>,
        watcher: &'a WeakFileWatcher,
    ) -> Self {
        Self {
            modified_paths,
            value,
            paths: Paths::Watcher(watcher),
        }
    }

    /// Get the list of modified paths.
    pub fn modified_paths(&self) -> &[&Path] {
        self.modified_paths
    }

    /// Get the first modified path.
    pub fn path(&self) -> &Path {
        self.modified_paths
            .first()
            .expect("Should always have at least one modified path in a context")
    }

    /// Update the value for the Watch.
    pub fn update_value(&self, value: T) {
        self.value.store(Arc::new(value));
    }

    /// Update the set of files to watch for changes.
    pub fn update_watched_files(&mut self, files: &[impl AsRef<Path>]) -> Result<(), Error> {
        match &mut self.paths {
            Paths::Vector(paths) => {
                let mut files: Vec<_> = files.iter().map(|f| f.as_ref().to_path_buf()).collect();
                std::mem::swap(&mut **paths, &mut files);
            }
            Paths::Watcher(watcher) => {
                let guard = watcher.lock().unwrap();
                if let Some(watcher) = guard.as_ref().and_then(|w| w.upgrade()) {
                    watcher.update_files(files)?;
                } else {
                    // This means the Watch has been dropped, so there's no one left
                    // to notify about changes. Do nothing.
                }
            }
        }
        Ok(())
    }
}
