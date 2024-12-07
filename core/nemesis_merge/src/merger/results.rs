use crate::error::Error;
use rayon::prelude::*;

#[inline]
pub fn filter_results(results: Vec<Result<(), Error>>) -> Result<(), Vec<Error>> {
    let errors: Vec<Error> = results.into_par_iter().filter_map(Result::err).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
