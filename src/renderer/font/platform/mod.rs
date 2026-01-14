#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

// Platform-specific Face type
#[cfg(target_os = "macos")]
pub use macos::Face;

#[cfg(target_os = "linux")]
pub use linux::Face;

#[cfg(target_os = "windows")]
pub use windows::Face;

// Platform-agnostic SystemFallback alias
#[cfg(target_os = "macos")]
pub use macos::CoreTextSystemFallback as PlatformSystemFallback;

#[cfg(target_os = "linux")]
pub use linux::LinuxSystemFallback as PlatformSystemFallback;

#[cfg(target_os = "windows")]
pub use windows::WindowsSystemFallback as PlatformSystemFallback;

// Platform-agnostic fallback resolver creation
#[cfg(target_os = "macos")]
pub use macos::create_fallback_resolver_with_embedded;

#[cfg(target_os = "linux")]
pub use linux::create_fallback_resolver_with_embedded;

#[cfg(target_os = "windows")]
pub use windows::create_fallback_resolver_with_embedded;
