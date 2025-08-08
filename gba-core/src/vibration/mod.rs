#[cfg(feature = "vibration")]
pub mod ble;

#[cfg(feature = "vibration")]
pub use ble::VibrationManager;

#[cfg(not(feature = "vibration"))]
pub struct VibrationManager;

#[cfg(not(feature = "vibration"))]
impl VibrationManager {
    pub fn new() -> Self {
        Self
    }

    pub fn vibrate(&self, _duration_ms: u64) {
        // No-op when vibration feature is disabled
    }

    pub fn stop_all_vibration(&self) {
        // No-op when vibration feature is disabled
    }
}
