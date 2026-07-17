//! Tabs builder struct — a tabbed content switcher.

/// A tabbed container: `bg-muted rounded-lg p-0.5` tab bar with content area below.
#[must_use]
pub struct Tabs {
    pub(crate) labels: Vec<String>,
}

impl Tabs {
    pub fn new(labels: Vec<String>) -> Self {
        Self { labels }
    }
}

/// Tab entry that can be a text label or icon with tooltip.
pub enum TabEntry {
    Text(String),
    Icon {
        icon: crate::icons::lucide_icon::LucideIcon,
        tooltip: String,
    },
}

/// Icon-based tabs variant.
#[must_use]
pub struct IconTabs {
    pub(crate) entries: Vec<TabEntry>,
}

impl IconTabs {
    pub fn new(entries: Vec<TabEntry>) -> Self {
        Self { entries }
    }
}
