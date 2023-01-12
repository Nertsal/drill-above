use super::*;

pub fn report_err<T, E: Display>(result: Result<T, E>, msg: impl AsRef<str>) {
    if let Err(err) = result {
        error!("{}: {err}", msg.as_ref());
    }
}
