#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{create_fallback_resolver_with_embedded, CoreTextSystemFallback, Face};

#[cfg(target_os = "linux")]
pub use linux::{create_fallback_resolver, Face, LinuxSystemFallback};

#[cfg(target_os = "windows")]
pub use windows::{create_fallback_resolver, Face, WindowsSystemFallback};
