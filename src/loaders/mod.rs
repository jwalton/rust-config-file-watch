#[cfg(feature = "json")]
mod json;

#[cfg(feature = "json")]
pub use json::JsonLoader;

#[cfg(feature = "json")]
fn load_from_file<T, F>(
    context: &mut crate::Context,
    mut load: F,
) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: serde::de::DeserializeOwned + Default,
    F: FnMut(std::fs::File) -> Result<T, Box<dyn std::error::Error + Send + Sync>>,
{
    match context.path() {
        None => Ok(T::default()),
        Some(path) => match std::fs::File::open(path) {
            Ok(file) => load(file),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(T::default())
                } else {
                    Err(Box::new(err))
                }
            }
        },
    }
}
