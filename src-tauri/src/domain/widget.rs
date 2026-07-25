use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::DomainError;

pub const WIDGET_LAYOUT_ID: &str = "default";
pub const WIDGET_WINDOW_LABEL: &str = "widget";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetSize {
    Compact,
    Standard,
    Expanded,
}

impl WidgetSize {
    pub fn default_dimensions(self) -> (f64, f64) {
        match self {
            Self::Compact => (320.0, 132.0),
            Self::Standard => (360.0, 420.0),
            Self::Expanded => (440.0, 640.0),
        }
    }

    pub fn task_limit(self) -> usize {
        match self {
            Self::Compact => 1,
            Self::Standard => 5,
            Self::Expanded => 10,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Standard => "standard",
            Self::Expanded => "expanded",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "compact" => Ok(Self::Compact),
            "standard" => Ok(Self::Standard),
            "expanded" => Ok(Self::Expanded),
            _ => Err(widget_error(
                "WIDGET_CONFIG_CORRUPTED",
                "unknown widget size",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetMode {
    Desktop,
    Floating,
}

impl WidgetMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Floating => "floating",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "desktop" => Ok(Self::Desktop),
            "floating" => Ok(Self::Floating),
            _ => Err(widget_error(
                "WIDGET_CONFIG_CORRUPTED",
                "unknown widget mode",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetModule {
    Clock,
    CurrentFocus,
    TodayProgress,
    Tasks,
    QuickActions,
    ProjectProgress,
    WeeklyGoals,
    NoteEntry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetConfigInput {
    pub size: WidgetSize,
    pub mode: WidgetMode,
    pub locked: bool,
    pub opacity: f64,
    pub modules: Vec<WidgetModule>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub monitor_id: Option<String>,
    pub scale_factor: f64,
}

impl WidgetConfigInput {
    pub fn validate(&self) -> Result<(), DomainError> {
        if !(0.2..=1.0).contains(&self.opacity) || !self.opacity.is_finite() {
            return Err(widget_field_error(
                "WIDGET_OPACITY_INVALID",
                "opacity must be between 0.2 and 1",
                "opacity",
            ));
        }
        if self.width <= 0.0 || !self.width.is_finite() {
            return Err(widget_field_error(
                "WIDGET_SIZE_INVALID",
                "width must be positive",
                "width",
            ));
        }
        if self.height <= 0.0 || !self.height.is_finite() {
            return Err(widget_field_error(
                "WIDGET_SIZE_INVALID",
                "height must be positive",
                "height",
            ));
        }
        if !self.x.is_finite() || !self.y.is_finite() {
            return Err(widget_field_error(
                "WIDGET_POSITION_INVALID",
                "window coordinates must be finite",
                "x",
            ));
        }
        if self.scale_factor <= 0.0 || !self.scale_factor.is_finite() {
            return Err(widget_field_error(
                "WIDGET_SCALE_INVALID",
                "scale factor must be positive",
                "scaleFactor",
            ));
        }
        if self.modules.is_empty() {
            return Err(widget_field_error(
                "WIDGET_MODULES_INVALID",
                "select at least one widget module",
                "modules",
            ));
        }
        let mut modules = self.modules.clone();
        modules.sort_by_key(|module| *module as u8);
        modules.dedup();
        if modules.len() != self.modules.len() {
            return Err(widget_field_error(
                "WIDGET_MODULES_INVALID",
                "widget modules must be unique",
                "modules",
            ));
        }
        Ok(())
    }
}

impl Default for WidgetConfigInput {
    fn default() -> Self {
        let size = WidgetSize::Standard;
        let (width, height) = size.default_dimensions();
        Self {
            size,
            mode: WidgetMode::Desktop,
            locked: false,
            opacity: 1.0,
            modules: vec![
                WidgetModule::Clock,
                WidgetModule::CurrentFocus,
                WidgetModule::TodayProgress,
                WidgetModule::Tasks,
                WidgetModule::QuickActions,
            ],
            x: 40.0,
            y: 40.0,
            width,
            height,
            monitor_id: None,
            scale_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetConfig {
    #[serde(flatten)]
    pub input: WidgetConfigInput,
    pub last_visible_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl WidgetConfig {
    pub fn new(input: WidgetConfigInput, now: DateTime<Utc>) -> Result<Self, DomainError> {
        input.validate()?;
        Ok(Self {
            input,
            last_visible_at: None,
            updated_at: now,
        })
    }
}

fn widget_error(code: &str, message: &str) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        field: None,
    }
}

fn widget_field_error(code: &str, message: &str, field: &str) -> DomainError {
    DomainError {
        code: code.into(),
        message: message.into(),
        field: Some(field.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_define_the_required_dimensions_and_task_limits() {
        assert_eq!(WidgetSize::Compact.default_dimensions(), (320.0, 132.0));
        assert_eq!(WidgetSize::Compact.task_limit(), 1);
        assert_eq!(WidgetSize::Standard.default_dimensions(), (360.0, 420.0));
        assert_eq!(WidgetSize::Standard.task_limit(), 5);
        assert_eq!(WidgetSize::Expanded.default_dimensions(), (440.0, 640.0));
        assert_eq!(WidgetSize::Expanded.task_limit(), 10);
    }

    #[test]
    fn configuration_rejects_invalid_window_and_module_values() {
        let input = WidgetConfigInput {
            opacity: 0.1,
            ..Default::default()
        };
        assert_eq!(input.validate().unwrap_err().code, "WIDGET_OPACITY_INVALID");

        let input = WidgetConfigInput {
            modules: vec![WidgetModule::Clock, WidgetModule::Clock],
            ..Default::default()
        };
        assert_eq!(input.validate().unwrap_err().code, "WIDGET_MODULES_INVALID");

        let input = WidgetConfigInput {
            scale_factor: 0.0,
            ..Default::default()
        };
        assert_eq!(input.validate().unwrap_err().code, "WIDGET_SCALE_INVALID");
    }
}
