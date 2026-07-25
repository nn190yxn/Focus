use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MainWindowState {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub maximized: bool,
    pub monitor_id: Option<String>,
    pub scale_factor: f64,
}

impl MainWindowState {
    pub fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
            && self.scale_factor.is_finite()
            && self.scale_factor > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_state_requires_finite_positive_dimensions_and_scale() {
        let mut state = MainWindowState {
            x: -120.0,
            y: 80.0,
            width: 1360.0,
            height: 860.0,
            maximized: false,
            monitor_id: Some("DISPLAY1".into()),
            scale_factor: 1.25,
        };
        assert!(state.is_valid());

        state.width = 0.0;
        assert!(!state.is_valid());
        state.width = 1360.0;
        state.scale_factor = f64::NAN;
        assert!(!state.is_valid());
    }
}
