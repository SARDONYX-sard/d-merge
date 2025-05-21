use crate::errors::Error;
use rayon::prelude::*;

#[inline]
pub fn filter_results<T>(results: Vec<Result<T, Error>>) -> Result<(), Vec<Error>>
where
    T: Send + Sync,
{
    let errors: Vec<Error> = results.into_par_iter().filter_map(Result::err).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
