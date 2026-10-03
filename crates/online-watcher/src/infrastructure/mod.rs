mod browser_log;
mod entropy;

pub(crate) use entropy::fill_secure_random;

pub(crate) use browser_log::info as log_info;

#[cfg(test)]
mod unit_tests;
