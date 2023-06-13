#[macro_export]
macro_rules! value_or_continue {
    ($expr:expr) => {
        match $expr {
            Some(value) => value,
            None => {
                debug!(
                    "Problem getting value from expression: {:?}",
                    stringify!($expr)
                );

                continue;
            }
        }
    };
}
