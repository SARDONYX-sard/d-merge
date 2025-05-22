use crate::errors::Error;
use rayon::{iter::Either, prelude::*};

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

pub fn partition_results<T, E>(results: Vec<Result<T, E>>) -> (Vec<T>, Vec<E>)
where
    T: Send,
    E: Send,
{
    results.into_par_iter().partition_map(|res| match res {
        Ok(v) => Either::Left(v),
        Err(e) => Either::Right(e),
    })
}
