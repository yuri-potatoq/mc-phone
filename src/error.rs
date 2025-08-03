use snafu::prelude::*;

pub(crate) type CrateResult<T, E = Error> = std::result::Result<T, E>;


#[derive(Debug, Snafu)]
pub(crate) enum Error {
    #[snafu(display("Fail when read/write with tcp connection: {}", raw_err))]
    ConnectionError { raw_err: String },
    
    #[snafu(display("Fail to start http server: {}", raw_err))]
    ServerError { raw_err: String },
    
    #[snafu(display("Password do not match: {}", raw_err))]
    PasswordDontMatch { raw_err: String },
    
    #[snafu(display("can't create use: {}", raw_err))]
    CantCreateUser { raw_err : String }
}

impl Error {    
    pub(crate) fn connection_error<S: ToString>(s: S) -> Self {
        Self::ConnectionError { raw_err: s.to_string() }
    }
    
    pub(crate) fn server_error<S: ToString>(s: S) -> Self {
        Self::ServerError { raw_err: s.to_string() }
    }
    
    pub(crate) fn cant_create_user<S: ToString>(s: S) -> Self {
        Self::CantCreateUser { raw_err: s.to_string() }
    }
}

