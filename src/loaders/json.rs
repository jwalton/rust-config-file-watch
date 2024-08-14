use std::{fs::File, io::BufReader};

use crate::{Context, Loader};

#[derive(Debug)]
pub struct JsonLoader;

impl<T> Loader<T> for JsonLoader
where
    T: serde::de::DeserializeOwned + Default,
{
    fn load(&mut self, context: &mut Context) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        let path = context.path();
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                Ok(serde_json::from_reader(reader)?)
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(T::default())
                } else {
                    Err(Box::new(err))
                }
            }
        }
    }
}
