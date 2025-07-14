use snafu::prelude::*;


pub(crate) type CrateResult<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub(crate) enum Error {
    #[snafu(display("Fail when read/write with tcp connection: {}", raw_err))]
    ConnectionError { raw_err: String },
}

impl Error {
    pub(crate) fn connection_error<S: ToString>(s: S) -> Self {
        Self::ConnectionError { raw_err: s.to_string() }
    }
}

