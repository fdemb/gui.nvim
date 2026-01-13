#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::{create_fallback_resolver_with_embedded, CoreTextSystemFallback, Face};
