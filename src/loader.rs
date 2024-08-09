use std::path::{Path, PathBuf};

use crate::Error;

/// Result of loading a configuration file.
pub struct Loaded<T> {
    pub(crate) value: T,
    pub(crate) dependencies: Option<Vec<PathBuf>>,
}

impl<T> Loaded<T> {
    /// Create a new Loaded instance.
    pub fn new(value: T) -> Self {
        Self {
            value,
            dependencies: None,
        }
    }

    /// Add a dependency to the loaded value.
    pub fn add_dependency(&mut self, dependency: impl AsRef<Path>) {
        let dependency = dependency.as_ref().to_path_buf();

        match self.dependencies.as_mut() {
            Some(deps) => {
                deps.push(dependency);
            }
            None => {
                self.dependencies = Some([dependency].into());
            }
        }
    }

    /// Add dependencies to the loaded value.
    pub fn add_dependencies<I>(&mut self, dependencies: I)
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let dependencies = dependencies.into_iter().map(|f| f.as_ref().to_path_buf());

        match self.dependencies.as_mut() {
            Some(deps) => {
                deps.extend(dependencies);
            }
            None => {
                self.dependencies = Some(dependencies.collect());
            }
        }
    }

    pub fn with_dependency(mut self, dependency: impl AsRef<Path>) -> Self {
        self.add_dependency(dependency);
        self
    }

    pub fn with_dependencies(mut self, dependencies: impl IntoIterator<Item = PathBuf>) -> Self {
        self.add_dependencies(dependencies);
        self
    }
}

/// Loads a configuration file.
pub trait Loader<T, E = ()> {
    /// Called when a file changes.
    ///
    /// `paths` is a list of files that have changed. The loader should return a
    /// `Loaded` instance, and may optionally set any files that should be watched
    /// for changes as dependencies. Any dependencies added will overwrite the
    /// original set of watched files.
    fn files_changed(&mut self, paths: &[&Path]) -> Result<Loaded<T>, E>;

    /// Called when an error occurs watching the file.
    fn on_error(&mut self, _error: Error) {}
}

impl<T, F, E> Loader<T, E> for F
where
    F: FnMut(Result<&Path, Error>) -> Result<T, E>,
{
    fn files_changed(&mut self, paths: &[&Path]) -> Result<Loaded<T>, E> {
        let path = paths.first().expect("Should always have at least one modified path.");
        let value = self(Ok(path))?;
        Ok(Loaded::new(value))
    }

    fn on_error(&mut self, error: Error) {
        let _ = self(Err(error));
    }
}
