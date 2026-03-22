#![allow(dead_code)]

use devprune_core::rules::types::{Category, SafetyLevel};
use ratatui::style::Color;

// ── General UI colours ────────────────────────────────────────────────────────

pub const BACKGROUND: Color = Color::Reset;
pub const FOREGROUND: Color = Color::White;
pub const DIMMED: Color = Color::DarkGray;

pub const BORDER: Color = Color::DarkGray;
pub const BORDER_ACTIVE: Color = Color::Cyan;

pub const HIGHLIGHT_BG: Color = Color::DarkGray;
pub const HIGHLIGHT_FG: Color = Color::White;

pub const HEADER_BG: Color = Color::DarkGray;
pub const HEADER_FG: Color = Color::Cyan;

pub const FOOTER_BG: Color = Color::DarkGray;
pub const FOOTER_FG: Color = Color::White;
pub const FOOTER_KEY_FG: Color = Color::Yellow;

pub const SPINNER_FG: Color = Color::Cyan;
pub const COMPLETE_FG: Color = Color::Green;
pub const ERROR_FG: Color = Color::Red;

pub const DIALOG_BG: Color = Color::DarkGray;
pub const DIALOG_BORDER: Color = Color::Yellow;
pub const DIALOG_TITLE: Color = Color::White;

pub const CHECKBOX_CHECKED: Color = Color::Green;
pub const CHECKBOX_PARTIAL: Color = Color::Yellow;
pub const CHECKBOX_EMPTY: Color = Color::DarkGray;

pub const SIZE_FG: Color = Color::Cyan;
pub const COUNT_FG: Color = Color::DarkGray;

// ── Category colours ──────────────────────────────────────────────────────────

pub fn category_color(cat: Category) -> Color {
    match cat {
        Category::Dependencies => Color::Blue,
        Category::BuildOutput => Color::Magenta,
        Category::Cache => Color::Cyan,
        Category::VirtualEnv => Color::Yellow,
        Category::IdeArtifact => Color::DarkGray,
        Category::Coverage => Color::Green,
        Category::Logs => Color::Gray,
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
