use std::io::BufReader;

use crate::{Context, Loader};

use super::load_from_file;

#[derive(Debug)]
pub struct JsonLoader;

impl<T> Loader<T> for JsonLoader
where
    T: serde::de::DeserializeOwned + Default,
{
    fn load(
        &mut self,
        context: &mut Context,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        load_from_file(context, |file| {
            let reader = BufReader::new(file);
            Ok(serde_json::from_reader(reader)?)
        })
    }
}
