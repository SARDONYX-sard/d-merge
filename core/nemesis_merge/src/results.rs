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

pub fn partition_results<T, E, O>(results: Vec<Result<T, E>>) -> Result<O, Vec<E>>
where
    T: Send, // Ensure T can be used safely in parallel
    E: Send, // Ensure E can be used safely in parallel
    O: Send + Default + ParallelExtend<T>,
{
    let (successes, errors): (O, Vec<E>) = results.into_par_iter().partition_map(|res| match res {
        Ok(value) => Either::Left(value),
        Err(err) => Either::Right(err),
    });

    if errors.is_empty() {
        Ok(successes)
    } else {
        Err(errors)
    }
}
