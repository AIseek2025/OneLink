use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignTokenSet {
    pub version: String,
    pub colors: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub radii: RadiiTokens,
    pub shadows: ShadowTokens,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorTokens {
    pub primary: String,
    pub primary_variant: String,
    pub secondary: String,
    pub secondary_variant: String,
    pub background: String,
    pub surface: String,
    pub error: String,
    pub on_primary: String,
    pub on_secondary: String,
    pub on_background: String,
    pub on_surface: String,
    pub on_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypographyTokens {
    pub headline1: TextStyleToken,
    pub headline2: TextStyleToken,
    pub headline3: TextStyleToken,
    pub subtitle1: TextStyleToken,
    pub subtitle2: TextStyleToken,
    pub body1: TextStyleToken,
    pub body2: TextStyleToken,
    pub caption: TextStyleToken,
    pub button: TextStyleToken,
    pub overline: TextStyleToken,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextStyleToken {
    pub font_size: f64,
    pub font_weight: u16,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub font_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpacingTokens {
    pub none: f64,
    pub xs: f64,
    pub sm: f64,
    pub md: f64,
    pub lg: f64,
    pub xl: f64,
    pub xxl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RadiiTokens {
    pub none: f64,
    pub sm: f64,
    pub md: f64,
    pub lg: f64,
    pub full: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowTokens {
    pub none: String,
    pub sm: String,
    pub md: String,
    pub lg: String,
}

pub fn frozen_design_tokens() -> DesignTokenSet {
    DesignTokenSet {
        version: "1.0.0".into(),
        colors: ColorTokens {
            primary: "#6200EE".into(),
            primary_variant: "#3700B3".into(),
            secondary: "#03DAC6".into(),
            secondary_variant: "#018786".into(),
            background: "#FFFFFF".into(),
            surface: "#FFFFFF".into(),
            error: "#B00020".into(),
            on_primary: "#FFFFFF".into(),
            on_secondary: "#000000".into(),
            on_background: "#000000".into(),
            on_surface: "#000000".into(),
            on_error: "#FFFFFF".into(),
        },
        typography: TypographyTokens {
            headline1: TextStyleToken {
                font_size: 24.0,
                font_weight: 700,
                line_height: 32.0,
                letter_spacing: 0.0,
                font_family: "System".into(),
            },
            headline2: TextStyleToken {
                font_size: 20.0,
                font_weight: 600,
                line_height: 28.0,
                letter_spacing: 0.0,
                font_family: "System".into(),
            },
            headline3: TextStyleToken {
                font_size: 18.0,
                font_weight: 600,
                line_height: 24.0,
                letter_spacing: 0.0,
                font_family: "System".into(),
            },
            subtitle1: TextStyleToken {
                font_size: 16.0,
                font_weight: 500,
                line_height: 22.0,
                letter_spacing: 0.15,
                font_family: "System".into(),
            },
            subtitle2: TextStyleToken {
                font_size: 14.0,
                font_weight: 500,
                line_height: 20.0,
                letter_spacing: 0.1,
                font_family: "System".into(),
            },
            body1: TextStyleToken {
                font_size: 16.0,
                font_weight: 400,
                line_height: 24.0,
                letter_spacing: 0.5,
                font_family: "System".into(),
            },
            body2: TextStyleToken {
                font_size: 14.0,
                font_weight: 400,
                line_height: 20.0,
                letter_spacing: 0.25,
                font_family: "System".into(),
            },
            caption: TextStyleToken {
                font_size: 12.0,
                font_weight: 400,
                line_height: 16.0,
                letter_spacing: 0.4,
                font_family: "System".into(),
            },
            button: TextStyleToken {
                font_size: 14.0,
                font_weight: 500,
                line_height: 20.0,
                letter_spacing: 1.25,
                font_family: "System".into(),
            },
            overline: TextStyleToken {
                font_size: 10.0,
                font_weight: 500,
                line_height: 16.0,
                letter_spacing: 1.5,
                font_family: "System".into(),
            },
        },
        spacing: SpacingTokens {
            none: 0.0,
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            xxl: 48.0,
        },
        radii: RadiiTokens {
            none: 0.0,
            sm: 4.0,
            md: 8.0,
            lg: 16.0,
            full: 9999.0,
        },
        shadows: ShadowTokens {
            none: "none".into(),
            sm: "0 1px 2px rgba(0,0,0,0.1)".into(),
            md: "0 4px 8px rgba(0,0,0,0.12)".into(),
            lg: "0 8px 16px rgba(0,0,0,0.15)".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frozen_tokens_version() {
        let tokens = frozen_design_tokens();
        assert_eq!(tokens.version, "1.0.0");
    }

    #[test]
    fn test_frozen_tokens_colors_complete() {
        let tokens = frozen_design_tokens();
        assert!(!tokens.colors.primary.is_empty());
        assert!(!tokens.colors.primary_variant.is_empty());
        assert!(!tokens.colors.secondary.is_empty());
        assert!(!tokens.colors.background.is_empty());
        assert!(!tokens.colors.error.is_empty());
        assert!(!tokens.colors.on_primary.is_empty());
        assert!(!tokens.colors.on_error.is_empty());
    }

    #[test]
    fn test_frozen_tokens_typography_complete() {
        let tokens = frozen_design_tokens();
        assert!(tokens.typography.headline1.font_size > 0.0);
        assert!(tokens.typography.body1.font_size > 0.0);
        assert!(tokens.typography.caption.font_size > 0.0);
        assert!(tokens.typography.button.font_weight > 0);
    }

    #[test]
    fn test_frozen_tokens_spacing_progression() {
        let tokens = frozen_design_tokens();
        assert!(tokens.spacing.none < tokens.spacing.xs);
        assert!(tokens.spacing.xs < tokens.spacing.sm);
        assert!(tokens.spacing.sm < tokens.spacing.md);
        assert!(tokens.spacing.md < tokens.spacing.lg);
        assert!(tokens.spacing.lg < tokens.spacing.xl);
        assert!(tokens.spacing.xl < tokens.spacing.xxl);
    }

    #[test]
    fn test_frozen_tokens_radii_progression() {
        let tokens = frozen_design_tokens();
        assert!(tokens.radii.none < tokens.radii.sm);
        assert!(tokens.radii.sm < tokens.radii.md);
        assert!(tokens.radii.md < tokens.radii.lg);
        assert!(tokens.radii.lg < tokens.radii.full);
    }

    #[test]
    fn test_tokens_serializable() {
        let tokens = frozen_design_tokens();
        let json = serde_json::to_string(&tokens).unwrap();
        assert!(!json.is_empty());
        let parsed: DesignTokenSet = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, tokens);
    }
}
