use std::path::Path;

use glob_match::glob_match;

use crate::rules::types::{MatchCondition, Rule};

/// Returns `true` when `name` satisfies `condition`.
///
/// Only the entry name (last component) is checked, not the full path.
pub fn matches_entry_name(name: &str, condition: &MatchCondition) -> bool {
    match condition {
        MatchCondition::DirName(expected) => name == expected,
        MatchCondition::DirGlob(pattern) => glob_match(pattern, name),
        MatchCondition::FileName(expected) => name == expected,
        MatchCondition::FileGlob(pattern) => glob_match(pattern, name),
        MatchCondition::FileExtension(ext) => Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == ext)
            .unwrap_or(false),
    }
}

/// Returns `true` when at least one marker in `markers` exists as a file or
/// directory inside `parent`.
///
/// An empty `markers` slice is treated as "no context required" and always
/// returns `true`.
pub fn check_context_markers(parent: &Path, markers: &[String]) -> bool {
    if markers.is_empty() {
        return true;
    }
    markers.iter().any(|m| parent.join(m).exists())
}

/// Finds the first enabled rule that matches `name` and (if the rule requires
/// it) has its context markers satisfied in `parent`.
///
/// `is_dir` must be set correctly: directory rules (`DirName`, `DirGlob`) only
/// match when `is_dir` is `true`; file rules match when `is_dir` is `false`.
pub fn find_matching_rule<'a>(
    name: &str,
    parent: &Path,
    is_dir: bool,
    rules: &'a [Rule],
) -> Option<&'a Rule> {
    rules.iter().find(|rule| {
        if !rule.enabled {
            return false;
        }

        // Check whether the match type aligns with the filesystem entry type.
        let matches_dir = rule.matches_directories();
        if matches_dir != is_dir {
            return false;
        }

        if !matches_entry_name(name, &rule.match_condition) {
            return false;
        }

        check_context_markers(parent, &rule.context_markers)
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::rules::types::{Category, SafetyLevel};

    fn make_rule(id: &str, condition: MatchCondition, markers: Vec<&str>, enabled: bool) -> Rule {
        Rule {
            id: id.to_string(),
            name: id.to_string(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: condition,
            context_markers: markers.into_iter().map(str::to_string).collect(),
            description: String::new(),
            enabled,
        }
    }

    // -------------------------------------------------------------------------
    // matches_entry_name
    // -------------------------------------------------------------------------

    #[test]
    fn dir_name_match() {
        let cond = MatchCondition::DirName("node_modules".to_string());
        assert!(matches_entry_name("node_modules", &cond));
        assert!(!matches_entry_name("Node_Modules", &cond));
        assert!(!matches_entry_name("node_modules_extra", &cond));
    }

    #[test]
    fn dir_glob_match() {
        let cond = MatchCondition::DirGlob("cmake-build-*".to_string());
        assert!(matches_entry_name("cmake-build-debug", &cond));
        assert!(matches_entry_name("cmake-build-release", &cond));
        assert!(!matches_entry_name("build", &cond));
    }

    #[test]
    fn file_name_match() {
        let cond = MatchCondition::FileName(".eslintcache".to_string());
        assert!(matches_entry_name(".eslintcache", &cond));
        assert!(!matches_entry_name("eslintcache", &cond));
    }

    #[test]
    fn file_glob_match() {
        let cond = MatchCondition::FileGlob("*.swp".to_string());
        assert!(matches_entry_name("main.rs.swp", &cond));
        assert!(matches_entry_name(".foo.swp", &cond));
        assert!(!matches_entry_name("main.rs", &cond));
    }

    #[test]
    fn file_extension_match() {
        let cond = MatchCondition::FileExtension("pyc".to_string());
        assert!(matches_entry_name("module.pyc", &cond));
        assert!(!matches_entry_name("module.py", &cond));
        assert!(!matches_entry_name("pyc", &cond));
    }

    // -------------------------------------------------------------------------
    // check_context_markers
    // -------------------------------------------------------------------------

    #[test]
    fn context_markers_empty() {
        let tmp = tempdir().unwrap();
        // Empty markers -> always true, even in an empty directory.
        assert!(check_context_markers(tmp.path(), &[]));
    }

    #[test]
    fn context_markers_with_existing_file() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();

        let markers = vec!["Cargo.toml".to_string()];
        assert!(check_context_markers(tmp.path(), &markers));
    }

    #[test]
    fn context_markers_any_match() {
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("pyproject.toml"), "").unwrap();

        // Only one of the three needs to be present.
        let markers = vec![
            "requirements.txt".to_string(),
            "pyproject.toml".to_string(),
            "setup.py".to_string(),
        ];
        assert!(check_context_markers(tmp.path(), &markers));
    }

    // -------------------------------------------------------------------------
    // find_matching_rule
    // -------------------------------------------------------------------------

    #[test]
    fn find_matching_rule_respects_dir_type() {
        let tmp = tempdir().unwrap();
        let rules = vec![make_rule(
            "dir-rule",
            MatchCondition::DirName("target".to_string()),
            vec![],
            true,
        )];

        // is_dir = true -> should match
        assert!(find_matching_rule("target", tmp.path(), true, &rules).is_some());
        // is_dir = false -> should not match
        assert!(find_matching_rule("target", tmp.path(), false, &rules).is_none());
    }

    #[test]
    fn find_matching_rule_respects_context() {
        let tmp = tempdir().unwrap();
        let rules = vec![make_rule(
            "cargo-target",
            MatchCondition::DirName("target".to_string()),
            vec!["Cargo.toml"],
            true,
        )];

        // No Cargo.toml present -> no match.
        assert!(find_matching_rule("target", tmp.path(), true, &rules).is_none());

        // Create the marker.
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        assert!(find_matching_rule("target", tmp.path(), true, &rules).is_some());
    }

    #[test]
    fn find_matching_rule_skips_disabled() {
        let tmp = tempdir().unwrap();
        let rules = vec![make_rule(
            "disabled-rule",
            MatchCondition::DirName("node_modules".to_string()),
            vec![],
            false, // disabled
        )];

        assert!(find_matching_rule("node_modules", tmp.path(), true, &rules).is_none());
    }
}
