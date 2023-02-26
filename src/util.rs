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

pub fn aabb_outline(aabb: Aabb2<f32>) -> Chain<f32> {
    let [a, b, c, d] = aabb.corners();
    Chain::new(vec![(a + b) / 2.0, a, d, c, b, (a + b) / 2.0])
}
