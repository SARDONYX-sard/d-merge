#[derive(Debug, snafu::Snafu)]
pub enum DiffCastError {
    #[snafu(display("Missing required field: {field}"))]
    MissingField { field: &'static str },
}
