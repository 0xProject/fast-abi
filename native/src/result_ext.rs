//! Extension trait for Result so we don't have to repeat JS exception creation.

use neon::{prelude::*, object::This, result::Throw};
use std::fmt::Display;

pub trait ResultExt<T> {
    fn or_throw<'cx, C: This>(self, cx: &mut CallContext<'cx, C>) -> Result<T, Throw>;
}

impl<T, E: Display> ResultExt<T> for Result<T, E> {
    fn or_throw<'cx, C: This>(self, cx: &mut CallContext<'cx, C>) -> Result<T, Throw> {
        self.or_else(|e| cx.throw_error(format!("{}", e)))
    }
}
