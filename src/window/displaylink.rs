//! macOS CADisplayLink implementation for frame synchronization.
//!
//! Uses NSView.displayLink(target:selector:) introduced in macOS 14 to get
//! proper vblank-synchronized frame callbacks without relying on
//! broken PresentMode::Fifo on macOS.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::NSView;
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSRunLoop};
use objc2_quartz_core::CADisplayLink;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

struct FrameState {
    ready: AtomicBool,
}

impl FrameState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            ready: AtomicBool::new(true),
        })
    }

    fn set_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }

    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    fn clear_ready(&self) {
        self.ready.store(false, Ordering::Release);
    }
}

#[derive(Default)]
struct DisplayLinkTargetIvars {
    frame_state: Cell<Option<Arc<FrameState>>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, we don't implement Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "GUIDisplayLinkTarget"]
    #[ivars = DisplayLinkTargetIvars]
    struct DisplayLinkTarget;

    impl DisplayLinkTarget {
        #[unsafe(method(onDisplayLink:))]
        fn on_display_link(&self, _link: &CADisplayLink) {
            if let Some(state) = DefinedClass::ivars(self).frame_state.take() {
                state.set_ready();
                DefinedClass::ivars(self).frame_state.set(Some(state));
            }
        }
    }

    unsafe impl NSObjectProtocol for DisplayLinkTarget {}
);

impl DisplayLinkTarget {
    fn new(mtm: MainThreadMarker, frame_state: Arc<FrameState>) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(DisplayLinkTargetIvars {
            frame_state: Cell::new(Some(frame_state)),
        });
        unsafe { msg_send![super(this), init] }
    }
}

/// CADisplayLink wrapper for macOS frame synchronization.
pub struct DisplayLink {
    _target: Retained<DisplayLinkTarget>,
    link: Retained<CADisplayLink>,
    frame_state: Arc<FrameState>,
}

impl DisplayLink {
    /// Create a new DisplayLink for the given window.
    ///
    /// Returns `None` if the window handle cannot be obtained or the
    /// display link cannot be created.
    pub fn new(window: &Window) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;

        let handle = window.window_handle().ok()?;
        let ns_view = match handle.as_raw() {
            RawWindowHandle::AppKit(h) => h.ns_view,
            _ => return None,
        };

        let view: &NSView = unsafe { ns_view.cast::<NSView>().as_ref() };

        let frame_state = FrameState::new();
        let target = DisplayLinkTarget::new(mtm, Arc::clone(&frame_state));

        let selector = sel!(onDisplayLink:);

        let link: Retained<CADisplayLink> = unsafe {
            msg_send![
                view,
                displayLinkWithTarget: &*target,
                selector: selector
            ]
        };

        let run_loop = NSRunLoop::mainRunLoop();
        let common_modes = unsafe { objc2_foundation::NSRunLoopCommonModes };
        unsafe {
            link.addToRunLoop_forMode(&run_loop, common_modes);
        }

        Some(Self {
            _target: target,
            link,
            frame_state,
        })
    }

    pub fn request_frame(&self) {
        self.frame_state.clear_ready();
    }

    pub fn is_frame_ready(&self) -> bool {
        self.frame_state.is_ready()
    }
}

impl Drop for DisplayLink {
    fn drop(&mut self) {
        self.link.invalidate();
    }
}
