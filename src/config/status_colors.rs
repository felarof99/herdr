use serde::Deserialize;

use super::theme::parse_color_checked;

/// Agent state whose indicator color can be overridden.
///
/// The variants mirror the states the sidebar renders, including the split
/// between a finished-but-unseen agent (`done`) and a seen idle agent (`idle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusColorSlot {
    Working,
    Blocked,
    Done,
    Idle,
    Unknown,
}

impl StatusColorSlot {
    pub const ALL: [Self; 5] = [
        Self::Working,
        Self::Blocked,
        Self::Done,
        Self::Idle,
        Self::Unknown,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Idle => "idle",
            Self::Unknown => "unknown",
        }
    }
}

/// Optional per-state colors for the agent status indicator and its status text.
///
/// Every key is optional. An absent table, or an absent key, keeps the color the
/// active theme already supplies, so the defaults are unchanged.
///
/// ```toml
/// [ui.status_colors]
/// working = "#89b4fa"
/// blocked = "#f9e2af"
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct StatusColorsConfig {
    /// Agents that are actively working. Default: the theme's yellow.
    pub working: Option<String>,
    /// Agents waiting on you. Default: the theme's red.
    pub blocked: Option<String>,
    /// Agents that finished and have not been looked at yet. Default: the theme's teal.
    pub done: Option<String>,
    /// Agents that are idle and already seen. Default: the theme's green.
    pub idle: Option<String>,
    /// Agents with no detected state. Default: the theme's muted overlay color.
    pub unknown: Option<String>,
}

impl StatusColorsConfig {
    /// The configured value for one slot, if any.
    pub fn raw(&self, slot: StatusColorSlot) -> Option<&str> {
        match slot {
            StatusColorSlot::Working => self.working.as_deref(),
            StatusColorSlot::Blocked => self.blocked.as_deref(),
            StatusColorSlot::Done => self.done.as_deref(),
            StatusColorSlot::Idle => self.idle.as_deref(),
            StatusColorSlot::Unknown => self.unknown.as_deref(),
        }
    }

    /// Resolve one slot. Unparseable values resolve to `None` so the theme default stands.
    pub fn resolved(&self, slot: StatusColorSlot) -> Option<ratatui::style::Color> {
        self.raw(slot).and_then(parse_color_checked)
    }

    pub fn diagnostics(&self) -> Vec<String> {
        StatusColorSlot::ALL
            .into_iter()
            .filter_map(|slot| {
                let value = self.raw(slot)?;
                parse_color_checked(value).is_none().then(|| {
                    format!(
                        "unknown color ui.status_colors.{} = {value:?}; using the theme default; \
                         accepts hex (#89b4fa), named colors (cyan, blue), or rgb(r,g,b)",
                        slot.as_str()
                    )
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use ratatui::style::Color;

    #[test]
    fn status_colors_default_to_empty() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.ui.status_colors, StatusColorsConfig::default());
        for slot in StatusColorSlot::ALL {
            assert_eq!(config.ui.status_colors.resolved(slot), None, "{slot:?}");
        }
        assert!(config.ui.status_colors.diagnostics().is_empty());
    }

    #[test]
    fn status_colors_parse_every_slot() {
        let config: Config = toml::from_str(
            r##"
[ui.status_colors]
working = "#71b7ff"
blocked = "#ffd60a"
done = "rgb(148, 226, 213)"
idle = "green"
unknown = "reset"
"##,
        )
        .unwrap();

        let colors = &config.ui.status_colors;
        assert_eq!(
            colors.resolved(StatusColorSlot::Working),
            Some(Color::Rgb(0x71, 0xb7, 0xff))
        );
        assert_eq!(
            colors.resolved(StatusColorSlot::Blocked),
            Some(Color::Rgb(0xff, 0xd6, 0x0a))
        );
        assert_eq!(
            colors.resolved(StatusColorSlot::Done),
            Some(Color::Rgb(148, 226, 213))
        );
        assert_eq!(colors.resolved(StatusColorSlot::Idle), Some(Color::Green));
        assert_eq!(
            colors.resolved(StatusColorSlot::Unknown),
            Some(Color::Reset)
        );
        assert!(colors.diagnostics().is_empty());
    }

    #[test]
    fn partial_status_colors_leave_other_slots_unset() {
        let config: Config = toml::from_str(
            r##"
[ui.status_colors]
working = "#71b7ff"
blocked = "#ffd60a"
"##,
        )
        .unwrap();

        let colors = &config.ui.status_colors;
        assert!(colors.resolved(StatusColorSlot::Working).is_some());
        assert!(colors.resolved(StatusColorSlot::Blocked).is_some());
        assert_eq!(colors.resolved(StatusColorSlot::Done), None);
        assert_eq!(colors.resolved(StatusColorSlot::Idle), None);
        assert_eq!(colors.resolved(StatusColorSlot::Unknown), None);
    }

    #[test]
    fn unknown_status_colors_are_diagnosed_and_fall_back() {
        let config: Config = toml::from_str(
            r##"
[ui.status_colors]
working = "not-a-color"
blocked = "#ffd60a"
"##,
        )
        .unwrap();

        let diagnostics = config.ui.status_colors.diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("ui.status_colors.working = \"not-a-color\""));
        assert!(diagnostics[0].contains("using the theme default"));
        assert_eq!(
            config.ui.status_colors.resolved(StatusColorSlot::Working),
            None
        );

        assert!(config
            .collect_diagnostics()
            .iter()
            .any(|d| d.contains("ui.status_colors.working")));
    }
}
