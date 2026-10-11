//! Logging to the host. On wasm this calls the `host.log` import; on native
//! targets (unit tests) it prints to stderr.

pub use crate::bindings::saveforge::plugin::host::Level;

pub fn log(level: Level, message: &str) {
    #[cfg(target_arch = "wasm32")]
    crate::bindings::saveforge::plugin::host::log(level, message);
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("[{level:?}] {message}");
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => { $crate::log($crate::Level::Debug, &::std::format!($($arg)*)) };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::log($crate::Level::Info, &::std::format!($($arg)*)) };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => { $crate::log($crate::Level::Warn, &::std::format!($($arg)*)) };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => { $crate::log($crate::Level::Error, &::std::format!($($arg)*)) };
}
