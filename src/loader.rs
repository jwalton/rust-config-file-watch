use std::path::Path;

use crate::{context::Context, Error};

/// Loads a configuration file.
pub trait Loader<T> {
    /// Called when a file changes.
    ///
    /// The context can be used to get the list of `modified_paths`, and to
    /// update the current value of the watch, or change the set of files being
    /// watched.
    fn files_changed(&mut self, context: &mut Context<T>);

    /// Called when an error occurs watching the file.
    fn on_error(&mut self, _error: Error) {}
}

/// Allow passing in a `|res| -> Option<T>` closure as a loader.
impl<T, F> Loader<T> for F
where
    F: FnMut(Result<&Path, Error>) -> Option<T>,
{
    fn files_changed(&mut self, context: &mut Context<T>) {
        let path = context.path();
        let value = self(Ok(path));
        if let Some(value) = value {
            context.update_value(value);
        }
    }

    fn on_error(&mut self, error: Error) {
        let _ = self(Err(error));
    }
}
