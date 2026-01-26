//! Built-in themes

/// List of built-in theme names
pub const BUILTIN_THEMES: &[&str] = &["dark", "light", "sepia", "high-contrast"];

/// Built-in theme definition
#[derive(Debug, Clone)]
pub struct ThemeDefinition {
    pub name: &'static str,
    pub background: &'static str,
    pub text: &'static str,
    pub heading: &'static str,
    pub link: &'static str,
    pub code_background: &'static str,
    pub code_text: &'static str,
    pub sidebar_background: &'static str,
    pub selection: &'static str,
}

/// Get a built-in theme by name
pub fn get_builtin_theme(name: &str) -> Option<ThemeDefinition> {
    match name.to_lowercase().as_str() {
        "dark" => Some(ThemeDefinition {
            name: "dark",
            background: "#1e1e1e",
            text: "#d4d4d4",
            heading: "#569cd6",
            link: "#4ec9b0",
            code_background: "#2d2d2d",
            code_text: "#ce9178",
            sidebar_background: "#252526",
            selection: "#264f78",
        }),
        "light" => Some(ThemeDefinition {
            name: "light",
            background: "#ffffff",
            text: "#333333",
            heading: "#0066cc",
            link: "#0077cc",
            code_background: "#f5f5f5",
            code_text: "#d73a49",
            sidebar_background: "#f3f3f3",
            selection: "#add6ff",
        }),
        "sepia" => Some(ThemeDefinition {
            name: "sepia",
            background: "#f4ecd8",
            text: "#5b4636",
            heading: "#8b4513",
            link: "#8b6914",
            code_background: "#e8dcc8",
            code_text: "#6b4226",
            sidebar_background: "#efe5d0",
            selection: "#d4c4a8",
        }),
        "high-contrast" => Some(ThemeDefinition {
            name: "high-contrast",
            background: "#000000",
            text: "#ffffff",
            heading: "#00ff00",
            link: "#00ffff",
            code_background: "#1a1a1a",
            code_text: "#ffff00",
            sidebar_background: "#0a0a0a",
            selection: "#0000ff",
        }),
        _ => None,
    }
}

/// Check if a theme name is a built-in theme
pub fn is_builtin_theme(name: &str) -> bool {
    BUILTIN_THEMES.contains(&name.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_theme() {
        assert!(get_builtin_theme("dark").is_some());
        assert!(get_builtin_theme("light").is_some());
        assert!(get_builtin_theme("sepia").is_some());
        assert!(get_builtin_theme("nonexistent").is_none());
    }

    #[test]
    fn test_is_builtin_theme() {
        assert!(is_builtin_theme("dark"));
        assert!(is_builtin_theme("DARK"));
        assert!(is_builtin_theme("Light"));
        assert!(!is_builtin_theme("custom"));
    }
}
