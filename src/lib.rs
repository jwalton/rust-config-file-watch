#[doc = include_str!("../README.md")]

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use arc_swap::ArcSwap;
use file_watcher::FileWatcher;

mod builder;
mod context;
mod error;
mod file_watcher;
mod loader;

pub use builder::Builder;
pub use context::Context;
pub use error::Error;
pub use loader::Loader;

/// A guard for the current value of a Watch.
pub type Guard<T> = arc_swap::Guard<Arc<T>>;

type WeakFileWatcher = Arc<Mutex<Option<Weak<FileWatcher>>>>;

#[derive(Debug, Clone)]
pub struct Watch<T> {
    value: Arc<ArcSwap<T>>,
    watcher: Arc<FileWatcher>,
}

impl<T> Watch<T> {
    /// Create a new Watch.
    ///
    /// # Parameters
    ///
    /// - `files` is the initial set of files to watch for changes.
    /// - `default` is the initial value for the configuration to use.
    /// - `debounce` is the duration to wait after a change before calling the loader.
    /// - `loader` is a function that will be called to update the value whenever
    ///   the file changes.  Loader returns the new value, and a new list of files
    ///   to watch including any dependencies
    ///
    fn create<FilesIter, LoaderImpl>(
        files: FilesIter,
        default: Arc<T>,
        debounce: Option<Duration>,
        mut loader: LoaderImpl,
    ) -> Result<Self, Error>
    where
        FilesIter: IntoIterator,
        FilesIter::Item: AsRef<Path>,
        T: Send + Sync + 'static,
        LoaderImpl: Loader<T> + Send + 'static,
    {
        let value = Arc::new(ArcSwap::from(default));
        let files = files
            .into_iter()
            .map(|f| f.as_ref().to_path_buf())
            .collect::<Vec<_>>();

        // We want to be able to update the watcher from within the loader, so
        // we need a weak reference to the watcher.
        let weak: WeakFileWatcher = Arc::new(Mutex::new(None));

        let watcher = {
            let value = value.clone();
            let weak = weak.clone();

            FileWatcher::create(files, debounce, move |res| match res {
                Ok(modified_files) => {
                    let mut context = Context::for_watch(modified_files, &value, &weak);
                    loader.files_changed(&mut context);
                }
                Err(e) => {
                    loader.on_error(e);
                }
            })?
        };

        let watcher = Arc::new(watcher);
        {
            let mut weak_lock = weak.lock().unwrap();
            *weak_lock = Some(Arc::downgrade(&watcher));
        }

        Ok(Watch { value, watcher })
    }

    /// Update the set of files to watch for changes.
    pub fn update_files(&mut self, files: &[impl AsRef<Path>]) -> Result<(), Error> {
        self.watcher.update_files(files)
    }

    /// Return the set of files this watcher is watching.
    pub fn watched_files(&self) -> Guard<Vec<PathBuf>> {
        self.watcher.watched_files()
    }

    /// Produces a temporary borrow of the current configuration value. If the
    /// underlying value is changed, the value in the guard will not be updated
    /// to preserve consistency.
    pub fn value(&self) -> Guard<T> {
        self.value.load()
    }
}

impl<T> Watch<Option<T>> {
    /// Create a watch for a JSON file.
    #[cfg(feature = "json")]
    pub fn json<F>(file: impl AsRef<Path>, f: F) -> Result<Watch<Option<T>>, Error>
    where
        T: serde::de::DeserializeOwned + Send + Sync + 'static,
        F: FnMut(Error) + Send + 'static,
    {
        Builder::default().files([file]).json(f)
    }
}
