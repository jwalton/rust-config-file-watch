use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{Error, Loader, Watch};

const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(100);

/// Used to create file watches.
pub struct Builder<T> {
    default: T,
    files: Vec<PathBuf>,
    debounce: Option<Duration>,
}

/// A builder for creating a new Watch instance.
impl<T: Send + Sync + 'static> Builder<T> {
    /// Create a builder for a Watch with the given default value. If the loader
    /// can't load the file, this value will be used.
    pub fn with_default(default: T) -> Self {
        Self {
            default,
            files: vec![],
            debounce: Some(DEFAULT_DEBOUNCE),
        }
    }

    /// Add a file to the watch. This is the initial set of files to watch for changes.
    pub fn file(mut self, file: impl AsRef<Path>) -> Self {
        self.files.push(file.as_ref().to_path_buf());
        self
    }

    /// Add a set of files to the watch. This is the initial set of files to watch for changes.
    pub fn files<I>(mut self, files: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        for f in files {
            self.files.push(f.as_ref().to_path_buf());
        }
        self
    }

    /// Set the duration to wait after a change before calling the loader.
    /// The default is 100ms.
    pub fn debounce(mut self, duration: Duration) -> Self {
        self.debounce = Some(duration);
        self
    }

    /// Clear the debounce duration.
    pub fn no_debounce(mut self) -> Self {
        self.debounce = None;
        self
    }

    /// Build the Watch instance with the specified loader.
    pub fn build<L, E>(self, mut loader: L) -> Result<Watch<T>, Error>
    where
        L: Loader<T, E> + Send + 'static,
    {
        // Try to load here to set the initial value.
        let changed_files: Vec<_> = self.files.iter().map(|f| f.as_ref()).collect();
        let (files, value) = match loader.files_changed(&changed_files) {
            Ok(loaded) => {
                let files = loaded.dependencies.unwrap_or(self.files);
                (files, loaded.value)
            }
            Err(_) => {
                // Use the default value if we can't load the file.
                (self.files, self.default)
            }
        };

        Watch::create(files, value, self.debounce, loader)
    }

    #[cfg(feature = "json")]
    /// Build the Watch instance with the specified loader, using a JSON loader.
    pub fn json(self) -> Result<Watch<T>, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.build(|res: Result<&Path, Error>| match res {
            Ok(path) => {
                let contents = std::fs::read_to_string(path).map_err(Error::Io)?;
                let value: T = serde_json::from_str(&contents)
                    .map_err(|e| Error::ParseError(e.to_string()))?;
                Ok(value)
            }
            Err(e) => Err(e),
        })
    }
}
impl<T> Builder<Option<T>> {
    /// Create a new Builder for a Watch with no initial value.
    fn new() -> Self {
        Self {
            default: None,
            files: vec![],
            debounce: Some(DEFAULT_DEBOUNCE),
        }
    }
}

impl<T> Default for Builder<Option<T>> {
    fn default() -> Self {
        Self::new()
    }
}
