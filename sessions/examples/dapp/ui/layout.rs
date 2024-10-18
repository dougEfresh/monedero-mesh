use ratatui::layout::{Constraint as TuiConstraint, Rect as TuiRect};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum UiConstraint {
    Percentage(u16),
    Ratio(u32, u32),
    Length(u16),
    LengthLessThanScreenHeight(u16),
    LengthLessThanScreenWidth(u16),
    LengthLessThanLayoutHeight(u16),
    LengthLessThanLayoutWidth(u16),
    Max(u16),
    MaxLessThanScreenHeight(u16),
    MaxLessThanScreenWidth(u16),
    MaxLessThanLayoutHeight(u16),
    MaxLessThanLayoutWidth(u16),
    Min(u16),
    MinLessThanScreenHeight(u16),
    MinLessThanScreenWidth(u16),
    MinLessThanLayoutHeight(u16),
    MinLessThanLayoutWidth(u16),
}

impl UiConstraint {
    pub fn to_tui(self, screen_size: TuiRect, layout_size: TuiRect) -> TuiConstraint {
        match self {
            Self::Percentage(n) => TuiConstraint::Percentage(n),
            Self::Ratio(x, y) => TuiConstraint::Ratio(x, y),
            Self::Length(n) => TuiConstraint::Length(n),
            Self::LengthLessThanScreenHeight(n) => {
                TuiConstraint::Length(screen_size.height.max(n) - n)
            }
            Self::LengthLessThanScreenWidth(n) => {
                TuiConstraint::Length(screen_size.width.max(n) - n)
            }
            Self::LengthLessThanLayoutHeight(n) => {
                TuiConstraint::Length(layout_size.height.max(n) - n)
            }
            Self::LengthLessThanLayoutWidth(n) => {
                TuiConstraint::Length(layout_size.width.max(n) - n)
            }
            Self::Max(n) => TuiConstraint::Max(n),
            Self::MaxLessThanScreenHeight(n) => TuiConstraint::Max(screen_size.height.max(n) - n),
            Self::MaxLessThanScreenWidth(n) => TuiConstraint::Max(screen_size.width.max(n) - n),
            Self::MaxLessThanLayoutHeight(n) => TuiConstraint::Max(layout_size.height.max(n) - n),
            Self::MaxLessThanLayoutWidth(n) => TuiConstraint::Max(layout_size.width.max(n) - n),
            Self::Min(n) => TuiConstraint::Min(n),
            Self::MinLessThanScreenHeight(n) => TuiConstraint::Min(screen_size.height.max(n) - n),
            Self::MinLessThanScreenWidth(n) => TuiConstraint::Min(screen_size.width.max(n) - n),
            Self::MinLessThanLayoutHeight(n) => TuiConstraint::Min(layout_size.height.max(n) - n),
            Self::MinLessThanLayoutWidth(n) => TuiConstraint::Min(layout_size.width.max(n) - n),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LayoutOptions {
    #[serde(default)]
    pub margin: Option<u16>,

    #[serde(default)]
    pub horizontal_margin: Option<u16>,

    #[serde(default)]
    pub vertical_margin: Option<u16>,

    #[serde(default)]
    pub constraints: Option<Vec<UiConstraint>>,
}

impl LayoutOptions {
    pub fn extend(mut self, other: &Self) -> Self {
        self.margin = other.margin.or(self.margin);
        self.horizontal_margin = other.horizontal_margin.or(self.horizontal_margin);
        self.vertical_margin = other.vertical_margin.or(self.vertical_margin);
        self.constraints = other.constraints.clone().or(self.constraints);
        self
    }

    pub fn constraints(x: u16, y: u16) -> Self {
        Self {
            margin: None,
            horizontal_margin: None,
            vertical_margin: None,
            constraints: Some(vec![
                UiConstraint::Percentage(x),
                UiConstraint::Percentage(y),
            ]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum AppLayout {
    Nothing,
    Table,
    InputAndLogs,
    Selection,
    HelpMenu,
    SortAndFilter,
    Dynamic(String),
    Horizontal {
        config: LayoutOptions,
        splits: Vec<AppLayout>,
    },
    Vertical {
        config: LayoutOptions,
        splits: Vec<AppLayout>,
    },
}

impl Default for AppLayout {
    fn default() -> Self {
        Self::Horizontal {
            config: LayoutOptions::constraints(70, 30),
            splits: vec![
                Self::Vertical {
                    config: LayoutOptions {
                        margin: None,
                        horizontal_margin: None,
                        vertical_margin: None,
                        constraints: vec![
                            UiConstraint::Length(2),
                            UiConstraint::Min(1),
                            UiConstraint::Length(2),
                        ]
                        .into(),
                    },
                    splits: vec![Self::Table, Self::InputAndLogs],
                },
                Self::Vertical {
                    config: LayoutOptions::constraints(30, 70),
                    splits: vec![Self::Selection, Self::HelpMenu],
                },
            ],
        }
    }
}

impl AppLayout {
    pub fn extend(self, other: &Self) -> Self {
        match (self, other) {
            (s, Self::Nothing) => s,
            (
                Self::Horizontal {
                    config: sconfig,
                    splits: _,
                },
                Self::Horizontal {
                    config: oconfig,
                    splits: osplits,
                },
            ) => Self::Horizontal {
                config: sconfig.extend(oconfig),
                splits: osplits.clone(),
            },

            (
                Self::Vertical {
                    config: sconfig,
                    splits: _,
                },
                Self::Vertical {
                    config: oconfig,
                    splits: osplits,
                },
            ) => Self::Vertical {
                config: sconfig.extend(oconfig),
                splits: osplits.clone(),
            },
            (_, other) => other.clone(),
        }
    }

    pub fn replace(self, target: &Self, replacement: &Self) -> Self {
        match self {
            Self::Horizontal { splits, config } => Self::Horizontal {
                splits: splits
                    .into_iter()
                    .map(|s| s.replace(target, replacement))
                    .collect(),
                config,
            },
            Self::Vertical { splits, config } => Self::Vertical {
                splits: splits
                    .into_iter()
                    .map(|s| s.replace(target, replacement))
                    .collect(),
                config,
            },
            other => {
                if other == *target {
                    replacement.clone()
                } else {
                    other
                }
            }
        }
    }
}
