use std::path::Path;

use serde::Deserialize;

use crate::error::{DevpruneError, Result};
use crate::rules::types::{Category, MatchCondition, Rule, SafetyLevel};

// ── TOML schema types ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct UserConfig {
    #[serde(default)]
    rules: RulesSection,
}

#[derive(Deserialize, Default)]
struct RulesSection {
    #[serde(default)]
    disable: DisableSection,
    #[serde(default)]
    custom: Vec<CustomRule>,
}

#[derive(Deserialize, Default)]
struct DisableSection {
    #[serde(default)]
    ids: Vec<String>,
}

#[derive(Deserialize)]
struct CustomRule {
    id: String,
    name: String,
    category: String,
    safety: String,
    match_condition: TomlMatchCondition,
    #[serde(default)]
    context_markers: Vec<String>,
    #[serde(default)]
    description: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct TomlMatchCondition {
    #[serde(rename = "type")]
    kind: String,
    value: String,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load user rules from `<config_dir>/rules.toml`.
///
/// Applies any disabled IDs to the builtin list and appends custom rules.
/// Returns the unmodified builtin list when the config file does not exist.
pub fn load_user_rules(config_dir: &Path, mut builtin: Vec<Rule>) -> Result<Vec<Rule>> {
    let config_path = config_dir.join("rules.toml");
    if !config_path.exists() {
        return Ok(builtin);
    }

    let raw = std::fs::read_to_string(&config_path)?;
    let config: UserConfig =
        toml::from_str(&raw).map_err(|e| DevpruneError::RuleConfig {
            message: format!("failed to parse {}: {e}", config_path.display()),
        })?;

    // Disable built-in rules by ID.
    for id in &config.rules.disable.ids {
        if let Some(rule) = builtin.iter_mut().find(|r| &r.id == id) {
            rule.enabled = false;
        }
    }

    // Append custom rules.
    for custom in config.rules.custom {
        let rule = parse_custom_rule(custom)?;
        builtin.push(rule);
    }

    Ok(builtin)
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn parse_custom_rule(c: CustomRule) -> Result<Rule> {
    let category = parse_category(&c.category)?;
    let safety = parse_safety(&c.safety)?;
    let match_condition = parse_match_condition(c.match_condition)?;

    Ok(Rule {
        id: c.id,
        name: c.name,
        category,
        safety,
        match_condition,
        context_markers: c.context_markers,
        description: c.description,
        enabled: c.enabled,
    })
}

fn parse_category(s: &str) -> Result<Category> {
    match s {
        "Dependencies" => Ok(Category::Dependencies),
        "BuildOutput" => Ok(Category::BuildOutput),
        "Cache" => Ok(Category::Cache),
        "VirtualEnv" => Ok(Category::VirtualEnv),
        "IdeArtifact" => Ok(Category::IdeArtifact),
        "Coverage" => Ok(Category::Coverage),
        "Logs" => Ok(Category::Logs),
        "CompiledGenerated" => Ok(Category::CompiledGenerated),
        "Misc" => Ok(Category::Misc),
        _ => Err(DevpruneError::RuleConfig {
            message: format!("unknown category: {s}"),
        }),
    }
}

fn parse_safety(s: &str) -> Result<SafetyLevel> {
    match s {
        "Safe" => Ok(SafetyLevel::Safe),
        "Cautious" => Ok(SafetyLevel::Cautious),
        "Risky" => Ok(SafetyLevel::Risky),
        _ => Err(DevpruneError::RuleConfig {
            message: format!("unknown safety level: {s}"),
        }),
    }
}

fn parse_match_condition(c: TomlMatchCondition) -> Result<MatchCondition> {
    match c.kind.as_str() {
        "DirName" => Ok(MatchCondition::DirName(c.value)),
        "DirGlob" => Ok(MatchCondition::DirGlob(c.value)),
        "FileName" => Ok(MatchCondition::FileName(c.value)),
        "FileGlob" => Ok(MatchCondition::FileGlob(c.value)),
        "FileExtension" => Ok(MatchCondition::FileExtension(c.value)),
        _ => Err(DevpruneError::RuleConfig {
            message: format!("unknown match condition type: {}", c.kind),
        }),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::rules::catalog::builtin_rules;

    fn write_config(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("rules.toml"), content).unwrap();
    }

    #[test]
    fn no_config_file_returns_builtins_unchanged() {
        let tmp = tempdir().unwrap();
        let builtins = builtin_rules();
        let count = builtins.len();
        let result = load_user_rules(tmp.path(), builtins).unwrap();
        assert_eq!(result.len(), count);
    }

    #[test]
    fn disable_by_id_marks_rule_disabled() {
        let tmp = tempdir().unwrap();
        write_config(
            tmp.path(),
            r#"
[rules.disable]
ids = ["npm-node-modules"]
"#,
        );

        let builtins = builtin_rules();
        let result = load_user_rules(tmp.path(), builtins).unwrap();

        let npm = result.iter().find(|r| r.id == "npm-node-modules").unwrap();
        assert!(!npm.enabled);

        // All other rules should still be enabled.
        let still_enabled = result
            .iter()
            .filter(|r| r.id != "npm-node-modules" && !r.enabled)
            .count();
        assert_eq!(still_enabled, 0);
    }

    #[test]
    fn custom_rule_is_appended() {
        let tmp = tempdir().unwrap();
        write_config(
            tmp.path(),
            r#"
[[rules.custom]]
id = "my-logs"
name = "My Logs"
category = "Logs"
safety = "Safe"
match_condition = { type = "DirName", value = "logs" }
context_markers = ["package.json"]
description = "Custom log rule"
enabled = true
"#,
        );

        let builtins = builtin_rules();
        let before_len = builtins.len();
        let result = load_user_rules(tmp.path(), builtins).unwrap();

        assert_eq!(result.len(), before_len + 1);
        let custom = result.iter().find(|r| r.id == "my-logs").unwrap();
        assert_eq!(custom.name, "My Logs");
        assert_eq!(custom.category, Category::Logs);
        assert_eq!(custom.safety, SafetyLevel::Safe);
        assert!(matches!(custom.match_condition, MatchCondition::DirName(ref v) if v == "logs"));
        assert_eq!(custom.context_markers, vec!["package.json"]);
        assert!(custom.enabled);
    }

    #[test]
    fn empty_config_leaves_rules_unchanged() {
        let tmp = tempdir().unwrap();
        write_config(tmp.path(), "");
        let builtins = builtin_rules();
        let count = builtins.len();
        let result = load_user_rules(tmp.path(), builtins).unwrap();
        assert_eq!(result.len(), count);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let tmp = tempdir().unwrap();
        write_config(tmp.path(), "not valid toml {{{{");
        let builtins = builtin_rules();
        let result = load_user_rules(tmp.path(), builtins);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_category_returns_error() {
        let tmp = tempdir().unwrap();
        write_config(
            tmp.path(),
            r#"
[[rules.custom]]
id = "bad"
name = "Bad"
category = "NonExistent"
safety = "Safe"
match_condition = { type = "DirName", value = "foo" }
"#,
        );
        let builtins = builtin_rules();
        let result = load_user_rules(tmp.path(), builtins);
        assert!(result.is_err());
    }

    #[test]
    fn all_match_condition_types_parse() {
        let tmp = tempdir().unwrap();
        write_config(
            tmp.path(),
            r#"
[[rules.custom]]
id = "r1"
name = "DirName rule"
category = "Cache"
safety = "Safe"
match_condition = { type = "DirName", value = ".cache" }

[[rules.custom]]
id = "r2"
name = "DirGlob rule"
category = "Cache"
safety = "Cautious"
match_condition = { type = "DirGlob", value = "*.egg-info" }

[[rules.custom]]
id = "r3"
name = "FileName rule"
category = "Misc"
safety = "Safe"
match_condition = { type = "FileName", value = ".DS_Store" }

[[rules.custom]]
id = "r4"
name = "FileGlob rule"
category = "CompiledGenerated"
safety = "Safe"
match_condition = { type = "FileGlob", value = "*.pyc" }

[[rules.custom]]
id = "r5"
name = "FileExtension rule"
category = "CompiledGenerated"
safety = "Safe"
match_condition = { type = "FileExtension", value = "class" }
"#,
        );
        let result = load_user_rules(tmp.path(), vec![]).unwrap();
        assert_eq!(result.len(), 5);
        assert!(matches!(result[0].match_condition, MatchCondition::DirName(_)));
        assert!(matches!(result[1].match_condition, MatchCondition::DirGlob(_)));
        assert!(matches!(result[2].match_condition, MatchCondition::FileName(_)));
        assert!(matches!(result[3].match_condition, MatchCondition::FileGlob(_)));
        assert!(matches!(result[4].match_condition, MatchCondition::FileExtension(_)));
    }
}
