use super::*;

pub fn report_err<T, E: Display>(result: Result<T, E>, msg: impl AsRef<str>) -> Result<T, ()> {
    match result {
        Err(err) => {
            error!("{}: {err}", msg.as_ref());
            Err(())
        }
        Ok(value) => Ok(value),
    }
}

pub fn report_warn<T, E: Display>(result: Result<T, E>, msg: impl AsRef<str>) -> Result<T, ()> {
    match result {
        Err(err) => {
            warn!("{}: {err}", msg.as_ref());
            Err(())
        }
        Ok(value) => Ok(value),
    }
}
