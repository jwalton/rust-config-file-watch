use crate::{context::Context, Error, Guard};

/// Loads a configuration file.
pub trait Loader<T> {
    /// Called when a file changes.
    ///
    /// The context can be used to get the list of `modified_paths`, and to
    /// update the current value of the watch, or change the set of files being
    /// watched.
    fn load(&mut self, context: &mut Context) -> Result<T, Box<dyn std::error::Error + Send + Sync>>;
}

/// Handles errors that occur during loading.
pub trait ErrorHandler {
    /// Called when an error occurs.
    fn on_error(&mut self, context: &mut Context, error: Error);
}

/// Handles updates.
pub trait UpdatedHandler<T> {
    fn after_update(&mut self, context: &mut Context, value: Guard<T>);
}

/// Allow passing in a closure as a loader.
impl<T, F> Loader<T> for F
where
    F: FnMut(&mut Context) -> Result<T, Box<dyn std::error::Error + Send + Sync>>,
{
    fn load(&mut self, context: &mut Context) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        self(context)
    }
}

/// Allow passing in a `|context, error|` closure as an error handler.
impl<F> ErrorHandler for F
where
    F: FnMut(&mut Context, Error),
{
    fn on_error(&mut self, context: &mut Context, error: Error) {
        self(context, error);
    }
}

/// Allow passing in a closure as an event handler.
impl<F, T> UpdatedHandler<T> for F
where
    F: FnMut(&mut Context, Guard<T>),
{
    fn after_update(&mut self, context: &mut Context, value: Guard<T>) {
        self(context, value)
    }
}

pub struct DefaultLoader;

impl Loader<()> for DefaultLoader {
    fn load(&mut self, _context: &mut Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

pub struct DefaultErrorHandler;

impl ErrorHandler for DefaultErrorHandler {
    fn on_error(&mut self, _context: &mut Context, error: Error) {
        eprintln!("Error loading config: {error:?}");
    }
}

pub struct DefaultUpdatedHandler;

impl<T> UpdatedHandler<T> for DefaultUpdatedHandler {
    fn after_update(&mut self, _context: &mut Context, _value: Guard<T>) {
        // Do nothing.
    }
}
