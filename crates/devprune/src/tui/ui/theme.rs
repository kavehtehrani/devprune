#![allow(dead_code)]

use devprune_core::rules::types::{Category, SafetyLevel};
use ratatui::style::Color;

// ── General UI colours ────────────────────────────────────────────────────────

pub const BACKGROUND: Color = Color::Reset;
pub const FOREGROUND: Color = Color::White;
pub const DIMMED: Color = Color::Rgb(90, 90, 90);

pub const BORDER: Color = Color::Rgb(60, 60, 60);
pub const BORDER_ACTIVE: Color = Color::Cyan;

pub const HIGHLIGHT_BG: Color = Color::Rgb(40, 40, 60);
pub const HIGHLIGHT_FG: Color = Color::White;

pub const HEADER_BG: Color = Color::Rgb(25, 25, 40);
pub const HEADER_FG: Color = Color::Cyan;

pub const FOOTER_BG: Color = Color::Rgb(25, 25, 40);
pub const FOOTER_FG: Color = Color::Rgb(180, 180, 180);
pub const FOOTER_KEY_FG: Color = Color::Yellow;

pub const SPINNER_FG: Color = Color::Cyan;
pub const COMPLETE_FG: Color = Color::Green;
pub const ERROR_FG: Color = Color::Red;

pub const DIALOG_BG: Color = Color::Rgb(30, 30, 45);
pub const DIALOG_BORDER: Color = Color::Yellow;
pub const DIALOG_TITLE: Color = Color::White;

pub const CHECKBOX_CHECKED: Color = Color::Green;
pub const CHECKBOX_PARTIAL: Color = Color::Yellow;
pub const CHECKBOX_EMPTY: Color = Color::Rgb(90, 90, 90);

pub const SIZE_FG: Color = Color::Cyan;
pub const COUNT_FG: Color = Color::Rgb(90, 90, 90);

// ── Category colours ──────────────────────────────────────────────────────────

pub fn category_color(cat: Category) -> Color {
    match cat {
        Category::Dependencies => Color::Blue,
        Category::BuildOutput => Color::Magenta,
        Category::Cache => Color::Cyan,
        Category::VirtualEnv => Color::Yellow,
        Category::IdeArtifact => Color::Rgb(120, 120, 120),
        Category::Coverage => Color::Green,
        Category::Logs => Color::Rgb(150, 150, 150),
        Category::CompiledGenerated => Color::LightMagenta,
        Category::Misc => Color::White,
    }
}

// ── Safety level colours ──────────────────────────────────────────────────────

pub fn safety_color(level: SafetyLevel) -> Color {
    match level {
        SafetyLevel::Safe => Color::Green,
        SafetyLevel::Cautious => Color::Yellow,
        SafetyLevel::Risky => Color::Red,
    }
}
