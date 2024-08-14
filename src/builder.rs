use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use arc_swap::ArcSwap;

use crate::{
    types::{DefaultErrorHandler, DefaultLoader, DefaultUpdatedHandler},
    Context, Error, ErrorHandler, Loader, UpdatedHandler, Watch,
};

const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(100);

/// Used to create file watches.
///
pub struct Builder<Load, Updated, ErrHandler> {
    /// The initial set of files to watch for changes.
    files: Vec<PathBuf>,
    /// The time to debounce changes before calling the loader.
    debounce: Option<Duration>,
    /// The loader to use to load the file or files.
    loader: Load,
    /// The error handler to use when an error occurs.
    error_handler: ErrHandler,
    /// The handler to use when the configuration is updated.
    after_update: Updated,
}

impl Builder<DefaultLoader, DefaultUpdatedHandler, DefaultErrorHandler> {
    /// Create a new Builder for a Watch.
    pub fn new() -> Self {
        Self {
            files: vec![],
            debounce: Some(DEFAULT_DEBOUNCE),
            loader: DefaultLoader,
            error_handler: DefaultErrorHandler,
            after_update: DefaultUpdatedHandler,
        }
    }
}

impl Default for Builder<DefaultLoader, DefaultUpdatedHandler, DefaultErrorHandler> {
    fn default() -> Self {
        Self::new()
    }
}

/// A builder for creating a new Watch instance.
impl<Load, Updated, ErrHandler> Builder<Load, Updated, ErrHandler> {
    /// Add a file to the watch. This is the initial set of files to watch for changes.
    pub fn watch_file(mut self, file: impl AsRef<Path>) -> Self {
        self.files.push(file.as_ref().to_path_buf());
        self
    }

    /// Add a set of files to the watch. This is the initial set of files to watch for changes.
    pub fn watch_files<I>(mut self, files: I) -> Self
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

    /// Set the loader to use to load the file or files.
    pub fn load<Load2>(self, loader: Load2) -> Builder<Load2, Updated, ErrHandler> {
        Builder {
            files: self.files,
            debounce: self.debounce,
            loader,
            error_handler: self.error_handler,
            after_update: self.after_update,
        }
    }

    /// Set the error handler to use when an error occurs.
    pub fn on_error<ErrHandler2>(
        self,
        error_handler: ErrHandler2,
    ) -> Builder<Load, Updated, ErrHandler2> {
        Builder {
            files: self.files,
            debounce: self.debounce,
            loader: self.loader,
            error_handler,
            after_update: self.after_update,
        }
    }

    /// Set the handler to call when the loaded value changes.
    pub fn after_update<Updated2>(
        self,
        after_update: Updated2,
    ) -> Builder<Load, Updated2, ErrHandler> {
        Builder {
            files: self.files,
            debounce: self.debounce,
            loader: self.loader,
            error_handler: self.error_handler,
            after_update,
        }
    }

    /// Build the Watch instance with the specified loader.
    pub fn build<T>(self) -> Result<Watch<T>, Error>
    where
        T: Default + Send + Sync + 'static,
        Load: Loader<T> + Send + 'static,
        Updated: UpdatedHandler<T> + Send + 'static,
        ErrHandler: ErrorHandler + Send + 'static,
    {
        let mut loader = self.loader;
        let mut error_handler = self.error_handler;
        let mut after_update = self.after_update;

        let mut files = self.files.clone();

        // Try to load here to set the initial value.
        let changed_files: Vec<_> = self.files.iter().map(|f| f.as_ref()).collect();
        let mut context = Context::for_paths(&changed_files, &mut files);
        let value = match loader.load(&mut context) {
            Ok(v) => ArcSwap::from_pointee(v),
            Err(e) => {
                error_handler.on_error(&mut context, Error::LoadError(e));
                ArcSwap::from_pointee(T::default())
            }
        };
        after_update.after_update(&mut context, value.load());

        Watch::create(
            files,
            value,
            self.debounce,
            loader,
            after_update,
            error_handler,
        )
    }

    #[cfg(feature = "tokio")]
    pub async fn build_async<T>(self) -> Result<Watch<T>, Error>
    where
        T: Default + Send + Sync + 'static,
        Load: Loader<T> + Send + 'static,
        Updated: UpdatedHandler<T> + Send + 'static,
        ErrHandler: ErrorHandler + Send + 'static,
    {
        tokio::task::spawn_blocking(move || self.build())
            .await
            .unwrap()
    }

    /// Configure the watch to load files from JSON.
    ///
    /// If the file is removed, the watch will be updated with the default value.
    /// If the file cannot be parsed, the watch's current value will be unchanged.
    ///
    #[cfg(feature = "json")]
    pub fn load_json(self) -> Builder<crate::loaders::JsonLoader, Updated, ErrHandler> {
        self.load(crate::loaders::JsonLoader)
    }
}
