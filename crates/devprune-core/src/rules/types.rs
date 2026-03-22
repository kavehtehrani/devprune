use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Dependencies,
    BuildOutput,
    Cache,
    VirtualEnv,
    IdeArtifact,
    Coverage,
    Logs,
    CompiledGenerated,
    Misc,
}

impl Category {
    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Dependencies => "Dependencies",
            Category::BuildOutput => "Build Output",
            Category::Cache => "Cache",
            Category::VirtualEnv => "Virtual Environments",
            Category::IdeArtifact => "IDE Artifacts",
            Category::Coverage => "Coverage",
            Category::Logs => "Logs",
            Category::CompiledGenerated => "Compiled / Generated",
            Category::Misc => "Miscellaneous",
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Category::Dependencies,
            Category::BuildOutput,
            Category::Cache,
            Category::VirtualEnv,
            Category::IdeArtifact,
            Category::Coverage,
            Category::Logs,
            Category::CompiledGenerated,
            Category::Misc,
        ]
    }
}

/// How risky it is to delete artifacts matched by a rule. The ordering is
/// intentional: Safe < Cautious < Risky.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SafetyLevel {
    Safe,
    Cautious,
    Risky,
}

impl SafetyLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            SafetyLevel::Safe => "Safe",
            SafetyLevel::Cautious => "Cautious",
            SafetyLevel::Risky => "Risky",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SafetyLevel::Safe => "Can be deleted and recreated automatically (e.g. build output).",
            SafetyLevel::Cautious => {
                "Usually safe to delete, but verify first (e.g. large caches)."
            }
            SafetyLevel::Risky => {
                "Deletion may cause data loss or require manual recovery (e.g. virtual envs with local packages)."
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MatchCondition {
    DirName(String),
    DirGlob(String),
    FileName(String),
    FileGlob(String),
    FileExtension(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub category: Category,
    pub safety: SafetyLevel,
    pub match_condition: MatchCondition,
    /// Optional directory/file names that must exist alongside a match for the
    /// rule to fire (e.g. a `package.json` next to a `node_modules` dir).
    pub context_markers: Vec<String>,
    pub description: String,
    pub enabled: bool,
}

impl Rule {
    /// Returns true when context_markers is non-empty, meaning the rule should
    /// only fire when at least one marker is present in the same directory.
    pub fn needs_context(&self) -> bool {
        !self.context_markers.is_empty()
    }

    /// Returns true when the match condition targets directories rather than
    /// individual files.
    pub fn matches_directories(&self) -> bool {
        matches!(
            self.match_condition,
            MatchCondition::DirName(_) | MatchCondition::DirGlob(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(condition: MatchCondition, markers: Vec<String>) -> Rule {
        Rule {
            id: "test-rule".to_string(),
            name: "Test Rule".to_string(),
            category: Category::Cache,
            safety: SafetyLevel::Safe,
            match_condition: condition,
            context_markers: markers,
            description: "A test rule.".to_string(),
            enabled: true,
        }
    }

    #[test]
    fn category_display_names_not_empty() {
        for cat in Category::all() {
            assert!(!cat.display_name().is_empty(), "{cat:?} has empty display name");
        }
    }

    #[test]
    fn category_all_returns_nine() {
        assert_eq!(Category::all().len(), 9);
    }

    #[test]
    fn safety_ordering() {
        assert!(SafetyLevel::Safe < SafetyLevel::Cautious);
        assert!(SafetyLevel::Cautious < SafetyLevel::Risky);
        assert!(SafetyLevel::Safe < SafetyLevel::Risky);
    }

    #[test]
    fn needs_context_false_when_markers_empty() {
        let rule = make_rule(MatchCondition::DirName("node_modules".to_string()), vec![]);
        assert!(!rule.needs_context());
    }

    #[test]
    fn needs_context_true_when_markers_present() {
        let rule = make_rule(
            MatchCondition::DirName("node_modules".to_string()),
            vec!["package.json".to_string()],
        );
        assert!(rule.needs_context());
    }

    #[test]
    fn matches_directories_true_for_dir_name() {
        let rule = make_rule(MatchCondition::DirName("target".to_string()), vec![]);
        assert!(rule.matches_directories());
    }

    #[test]
    fn matches_directories_true_for_dir_glob() {
        let rule = make_rule(MatchCondition::DirGlob("*.egg-info".to_string()), vec![]);
        assert!(rule.matches_directories());
    }

    #[test]
    fn matches_directories_false_for_file_name() {
        let rule = make_rule(MatchCondition::FileName(".DS_Store".to_string()), vec![]);
        assert!(!rule.matches_directories());
    }

    #[test]
    fn matches_directories_false_for_file_glob() {
        let rule = make_rule(MatchCondition::FileGlob("*.pyc".to_string()), vec![]);
        assert!(!rule.matches_directories());
    }

    #[test]
    fn matches_directories_false_for_file_extension() {
        let rule = make_rule(MatchCondition::FileExtension("class".to_string()), vec![]);
        assert!(!rule.matches_directories());
    }

    #[test]
    fn rule_serialization_roundtrip() {
        let rule = make_rule(
            MatchCondition::DirName("node_modules".to_string()),
            vec!["package.json".to_string()],
        );
        let json = serde_json::to_string(&rule).unwrap();
        let decoded: Rule = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, rule.id);
        assert_eq!(decoded.name, rule.name);
        assert!(decoded.needs_context());
        assert!(decoded.matches_directories());
    }
}
