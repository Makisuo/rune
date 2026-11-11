//! UIKit window geometry. A UIKit window always covers its screen, and winit's `inner_size` is
//! the safe area inside it, while the view and its Metal layer cover the whole window. Rune
//! draws edge to edge and reports the safe area as insets (`Cx::safe_area`).

#![cfg_attr(not(target_os = "ios"), allow(dead_code))]

use winit::dpi::PhysicalSize;
use winit::window::Window;

thread_local! {
    static INSETS: std::cell::Cell<[f32; 4]> = const { std::cell::Cell::new([0.0; 4]) };
}

/// The window's drawable size: all of it, status bar and home indicator areas included.
pub(crate) fn surface_size(window: &Window) -> PhysicalSize<u32> {
    window.outer_size()
}

/// Reads the window's safe area into `safe_area`.
pub(crate) fn sync_safe_area(window: &Window) {
    let (Ok(outer), Ok(inner)) = (window.outer_position(), window.inner_position()) else { return };
    let (whole, safe) = (window.outer_size(), window.inner_size());
    INSETS.set(insets([outer.x, outer.y], [whole.width, whole.height], [inner.x, inner.y], [safe.width, safe.height], window.scale_factor()));
}

/// The safe area's insets in points, in top/right/bottom/left order.
pub(crate) fn safe_area() -> [f32; 4] { INSETS.get() }

/// Insets in points from the whole window's and its safe area's physical origins and sizes.
fn insets(outer: [i32; 2], whole: [u32; 2], inner: [i32; 2], safe: [u32; 2], scale: f64) -> [f32; 4] {
    let left = f64::from(inner[0] - outer[0]).max(0.0);
    let top = f64::from(inner[1] - outer[1]).max(0.0);
    let right = (f64::from(whole[0]) - f64::from(safe[0]) - left).max(0.0);
    let bottom = (f64::from(whole[1]) - f64::from(safe[1]) - top).max(0.0);
    [top, right, bottom, left].map(|v| (v / scale) as f32)
}

#[cfg(test)]
#[path = "ios_tests.rs"]
mod tests;
