//! Pure 2D pan/zoom math for the canvas. Kept free of `egui` types so it can
//! be unit tested without a running GUI.
//!
//! World coordinates are `f64` (matching `cad_core::LengthMm`); screen
//! coordinates are `f32` (matching `egui`'s pixel type). The narrowing casts
//! between them are intentional — screen space has nowhere near `f64`'s
//! precision requirements at any zoom level this app supports.
#![allow(clippy::cast_possible_truncation)]

/// Screen-space pixels per world millimetre.
pub const MIN_ZOOM: f32 = 0.05;
pub const MAX_ZOOM: f32 = 200.0;
const DEFAULT_ZOOM: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Screen-space offset (pixels) applied after centering on the canvas.
    pub offset: (f32, f32),
    /// Screen pixels per world millimetre.
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            offset: (0.0, 0.0),
            zoom: DEFAULT_ZOOM,
        }
    }
}

impl Camera {
    /// Converts a world point (millimetres, Y-up) to a screen point
    /// (pixels, Y-down) relative to the canvas's on-screen center.
    #[must_use]
    pub fn world_to_screen(&self, canvas_center: (f32, f32), world: (f64, f64)) -> (f32, f32) {
        (
            canvas_center.0 + self.offset.0 + (world.0 as f32) * self.zoom,
            canvas_center.1 + self.offset.1 - (world.1 as f32) * self.zoom,
        )
    }

    /// The inverse of [`Self::world_to_screen`].
    #[must_use]
    pub fn screen_to_world(&self, canvas_center: (f32, f32), screen: (f32, f32)) -> (f64, f64) {
        (
            f64::from((screen.0 - canvas_center.0 - self.offset.0) / self.zoom),
            f64::from(-(screen.1 - canvas_center.1 - self.offset.1) / self.zoom),
        )
    }

    pub fn pan(&mut self, screen_delta: (f32, f32)) {
        self.offset.0 += screen_delta.0;
        self.offset.1 += screen_delta.1;
    }

    /// Scales zoom by `factor` (>1 zooms in, <1 zooms out) while keeping the
    /// world point currently under `anchor_screen` fixed on screen — the
    /// standard "zoom toward the cursor" behavior.
    pub fn zoom_at(&mut self, canvas_center: (f32, f32), anchor_screen: (f32, f32), factor: f32) {
        let world_before = self.screen_to_world(canvas_center, anchor_screen);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let screen_after = self.world_to_screen(canvas_center, world_before);
        self.offset.0 += anchor_screen.0 - screen_after.0;
        self.offset.1 += anchor_screen.1 - screen_after.1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_centers_the_origin_at_canvas_center() {
        let camera = Camera::default();
        let center = (400.0, 300.0);
        assert_eq!(camera.world_to_screen(center, (0.0, 0.0)), center);
    }

    #[test]
    fn world_to_screen_flips_y() {
        let camera = Camera::default();
        let center = (0.0, 0.0);
        let (_, screen_y) = camera.world_to_screen(center, (0.0, 10.0));
        assert!(screen_y < 0.0, "positive world Y should map above center");
    }

    #[test]
    fn screen_to_world_is_the_inverse_of_world_to_screen() {
        let camera = Camera::default();
        let center = (400.0, 300.0);
        let world = (12.5, -7.25);
        let screen = camera.world_to_screen(center, world);
        let round_tripped = camera.screen_to_world(center, screen);
        assert!((round_tripped.0 - world.0).abs() < 1.0e-4);
        assert!((round_tripped.1 - world.1).abs() < 1.0e-4);
    }

    #[test]
    fn pan_shifts_the_offset() {
        let mut camera = Camera::default();
        camera.pan((10.0, -5.0));
        assert_eq!(camera.offset, (10.0, -5.0));
    }

    #[test]
    fn zoom_at_keeps_the_anchor_point_fixed_on_screen() {
        let mut camera = Camera::default();
        let center = (400.0, 300.0);
        let anchor = (550.0, 250.0);
        let world_under_anchor_before = camera.screen_to_world(center, anchor);

        camera.zoom_at(center, anchor, 2.0);

        let world_under_anchor_after = camera.screen_to_world(center, anchor);
        assert!((world_under_anchor_before.0 - world_under_anchor_after.0).abs() < 1.0e-3);
        assert!((world_under_anchor_before.1 - world_under_anchor_after.1).abs() < 1.0e-3);
        assert!((camera.zoom - DEFAULT_ZOOM * 2.0).abs() < 1.0e-4);
    }

    #[test]
    fn zoom_is_clamped_to_the_allowed_range() {
        let mut camera = Camera::default();
        camera.zoom_at((0.0, 0.0), (0.0, 0.0), 0.0);
        assert!((camera.zoom - MIN_ZOOM).abs() < f32::EPSILON);

        camera.zoom_at((0.0, 0.0), (0.0, 0.0), f32::MAX);
        assert!((camera.zoom - MAX_ZOOM).abs() < f32::EPSILON);
    }
}
