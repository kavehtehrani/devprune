use super::types::{Category, MatchCondition, Rule, SafetyLevel};

/// Convenience macro for building a [`Rule`] with less boilerplate.
///
/// # Usage
///
/// Without context markers:
/// ```text
/// rule!(vec, "id", "name", Category, Safety, Condition("arg"), "description")
/// ```
///
/// With context markers:
/// ```text
/// rule!(vec, "id", "name", Category, Safety, Condition("arg"), ["m1", "m2"], "description")
/// ```
macro_rules! rule {
    // With context markers.
    ($rules:expr, $id:expr, $name:expr, $cat:expr, $safety:expr, $cond:ident($arg:expr),
     [$($marker:expr),+], $desc:expr) => {
        $rules.push(Rule {
            id: $id.into(),
            name: $name.into(),
            category: $cat,
            safety: $safety,
            match_condition: MatchCondition::$cond($arg.into()),
            context_markers: vec![$($marker.into()),+],
            description: $desc.into(),
            enabled: true,
        })
    };
    // Without context markers.
    ($rules:expr, $id:expr, $name:expr, $cat:expr, $safety:expr, $cond:ident($arg:expr), $desc:expr) => {
        $rules.push(Rule {
            id: $id.into(),
            name: $name.into(),
            category: $cat,
            safety: $safety,
            match_condition: MatchCondition::$cond($arg.into()),
            context_markers: vec![],
            description: $desc.into(),
            enabled: true,
        })
    };
}

/// Returns all built-in detection rules.
pub fn builtin_rules() -> Vec<Rule> {
    let mut rules = Vec::new();

    // -------------------------------------------------------------------------
    // Dependencies
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "npm-node-modules",
        "npm / Node.js node_modules",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName("node_modules"),
        "npm / Yarn / pnpm dependency cache. Recreated by running the package manager install command."
    );

    rule!(
        rules,
        "bower-components",
        "Bower bower_components",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName("bower_components"),
        "Bower front-end package directory. Recreated by running `bower install`."
    );

    rule!(
        rules,
        "php-vendor",
        "Composer vendor (PHP)",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName("vendor"),
        ["composer.json"],
        "PHP Composer dependency directory. Recreated by running `composer install`."
    );

    rule!(
        rules,
        "go-vendor",
        "Go vendor",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName("vendor"),
        ["go.mod"],
        "Go module vendor directory. Recreated by running `go mod vendor`."
    );

    rule!(
        rules,
        "ruby-bundle",
        "Bundler .bundle",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName(".bundle"),
        "Ruby Bundler local gem cache. Recreated by running `bundle install`."
    );

    rule!(
        rules,
        "cocoapods",
        "CocoaPods Pods",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName("Pods"),
        ["Podfile"],
        "CocoaPods dependency directory. Recreated by running `pod install`."
    );

    rule!(
        rules,
        "dart-pub-cache",
        "Dart / Flutter .pub-cache",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName(".pub-cache"),
        "Dart / Flutter pub package cache. Recreated automatically by pub."
    );

    rule!(
        rules,
        "elm-stuff",
        "Elm elm-stuff",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName("elm-stuff"),
        "Elm package cache and compiled artefacts. Recreated by `elm make`."
    );

    rule!(
        rules,
        "jspm-packages",
        "jspm jspm_packages",
        Category::Dependencies,
        SafetyLevel::Safe,
        DirName("jspm_packages"),
        "jspm package directory. Recreated by `jspm install`."
    );

    // -------------------------------------------------------------------------
    // Build Outputs
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "cargo-target",
        "Cargo target (Rust)",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("target"),
        ["Cargo.toml"],
        "Rust Cargo build output. Recreated by `cargo build`."
    );

    rule!(
        rules,
        "maven-target",
        "Maven target (Java)",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("target"),
        ["pom.xml"],
        "Maven build output directory. Recreated by `mvn compile`."
    );

    rule!(
        rules,
        "sbt-target",
        "sbt target (Scala)",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("target"),
        ["build.sbt"],
        "sbt / Scala build output directory. Recreated by running `sbt compile`."
    );

    rule!(
        rules,
        "gradle-build",
        "Gradle build output",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("build"),
        ["build.gradle"],
        "Gradle build output directory. Recreated by running the Gradle build task."
    );

    rule!(
        rules,
        "cmake-build",
        "CMake build output",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("build"),
        ["CMakeLists.txt"],
        "CMake build output directory. Recreated by running `cmake --build .`."
    );

    rule!(
        rules,
        "python-build",
        "Python build output",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("build"),
        ["setup.py"],
        "Python setuptools build directory. Recreated by `python setup.py build`."
    );

    rule!(
        rules,
        "npm-build",
        "npm / Node.js build output",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("build"),
        ["package.json"],
        "Node.js / React build output directory. Recreated by running the project's build script."
    );

    rule!(
        rules,
        "npm-dist",
        "npm dist output",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("dist"),
        ["package.json"],
        "Node.js distribution / bundle output. Recreated by running the build script."
    );

    rule!(
        rules,
        "python-dist",
        "Python dist output",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("dist"),
        ["setup.py", "pyproject.toml"],
        "Python distribution packages (wheels, sdists). Recreated by `python -m build`."
    );

    rule!(
        rules,
        "gradle-out",
        "Gradle out directory",
        Category::BuildOutput,
        SafetyLevel::Cautious,
        DirName("out"),
        ["build.gradle"],
        "Gradle / IntelliJ out directory. Usually safe to delete, but verify if it contains generated sources."
    );

    rule!(
        rules,
        "intellij-out",
        "IntelliJ out directory",
        Category::BuildOutput,
        SafetyLevel::Cautious,
        DirName("out"),
        [".classpath"],
        "IntelliJ IDEA compiled class output directory. Usually recreatable via a project rebuild."
    );

    rule!(
        rules,
        "elixir-build",
        "Elixir _build",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName("_build"),
        ["mix.exs"],
        "Elixir / Erlang Mix build artefacts. Recreated by `mix compile`."
    );

    rule!(
        rules,
        "swift-build",
        "Swift Package Manager .build",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirName(".build"),
        ["Package.swift"],
        "Swift Package Manager build directory. Recreated by `swift build`."
    );

    rule!(
        rules,
        "cmake-build-glob",
        "CMake cmake-build-* directories",
        Category::BuildOutput,
        SafetyLevel::Safe,
        DirGlob("cmake-build-*"),
        "CMake build directories created by IDEs such as CLion. Safe to delete and regenerate."
    );

    // -------------------------------------------------------------------------
    // Caches
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "python-pycache",
        "Python __pycache__",
        Category::Cache,
        SafetyLevel::Safe,
        DirName("__pycache__"),
        "Python bytecode cache. Automatically recreated on next import."
    );

    rule!(
        rules,
        "pytest-cache",
        "pytest cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".pytest_cache"),
        "pytest results and cache. Recreated on the next test run."
    );

    rule!(
        rules,
        "mypy-cache",
        "mypy cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".mypy_cache"),
        "mypy type-checker cache. Recreated on the next mypy run."
    );

    rule!(
        rules,
        "ruff-cache",
        "Ruff cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".ruff_cache"),
        "Ruff linter cache. Recreated on the next ruff run."
    );

    rule!(
        rules,
        "parcel-cache",
        "Parcel cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".parcel-cache"),
        "Parcel bundler cache. Recreated on the next build."
    );

    rule!(
        rules,
        "turbo-cache",
        "Turborepo cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".turbo"),
        "Turborepo build cache. Recreated on the next turbo run."
    );

    rule!(
        rules,
        "next-cache",
        "Next.js cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".next"),
        ["package.json"],
        "Next.js build cache and compiled output. Recreated by `next build`."
    );

    rule!(
        rules,
        "nuxt-cache",
        "Nuxt.js cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".nuxt"),
        ["package.json"],
        "Nuxt.js build cache. Recreated by `nuxt build`."
    );

    rule!(
        rules,
        "angular-cache",
        "Angular cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".angular"),
        "Angular CLI build cache. Recreated on the next build."
    );

    rule!(
        rules,
        "svelte-kit-cache",
        "SvelteKit cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".svelte-kit"),
        "SvelteKit generated files and cache. Recreated by running the dev/build command."
    );

    rule!(
        rules,
        "gradle-cache",
        "Gradle wrapper cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".gradle"),
        "Gradle build cache and wrapper files. Recreated on the next Gradle run."
    );

    rule!(
        rules,
        "sass-cache",
        "Sass cache",
        Category::Cache,
        SafetyLevel::Safe,
        DirName(".sass-cache"),
        "Sass/SCSS compiler cache. Recreated on the next compilation."
    );

    rule!(
        rules,
        "eslint-cache",
        "ESLint cache file",
        Category::Cache,
        SafetyLevel::Safe,
        FileName(".eslintcache"),
        "ESLint lint result cache. Recreated on the next eslint run with --cache."
    );

    rule!(
        rules,
        "stylelint-cache",
        "Stylelint cache file",
        Category::Cache,
        SafetyLevel::Safe,
        FileName(".stylelintcache"),
        "Stylelint lint result cache. Recreated on the next stylelint run with --cache."
    );

    // -------------------------------------------------------------------------
    // Virtual Environments
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "python-venv",
        "Python .venv",
        Category::VirtualEnv,
        SafetyLevel::Cautious,
        DirName(".venv"),
        ["requirements.txt", "pyproject.toml", "setup.py"],
        "Python virtual environment. Recreated by `python -m venv .venv && pip install -r requirements.txt`."
    );

    rule!(
        rules,
        "python-venv-bare",
        "Python venv directory",
        Category::VirtualEnv,
        SafetyLevel::Cautious,
        DirName("venv"),
        ["requirements.txt", "pyproject.toml", "setup.py"],
        "Python virtual environment. Recreated by `python -m venv venv && pip install -r requirements.txt`."
    );

    rule!(
        rules,
        "python-env",
        "Python env directory",
        Category::VirtualEnv,
        SafetyLevel::Risky,
        DirName("env"),
        ["requirements.txt", "pyproject.toml", "setup.py"],
        "Python virtual environment with a generic name. Verify contents before deleting; the name `env` is ambiguous."
    );

    rule!(
        rules,
        "conda-env",
        "Conda environment",
        Category::VirtualEnv,
        SafetyLevel::Cautious,
        DirName(".conda"),
        "Conda environment directory. Recreatable, but custom packages may need reinstalling."
    );

    rule!(
        rules,
        "tox-env",
        "tox environments",
        Category::VirtualEnv,
        SafetyLevel::Safe,
        DirName(".tox"),
        ["tox.ini"],
        "tox test automation virtual environments. Recreated on the next `tox` run."
    );

    rule!(
        rules,
        "nox-env",
        "nox environments",
        Category::VirtualEnv,
        SafetyLevel::Safe,
        DirName(".nox"),
        ["noxfile.py"],
        "nox test automation virtual environments. Recreated on the next `nox` run."
    );

    // -------------------------------------------------------------------------
    // IDE Artifacts
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "jetbrains-idea",
        "JetBrains .idea",
        Category::IdeArtifact,
        SafetyLevel::Risky,
        DirName(".idea"),
        "JetBrains IDE project settings. Contains run configs and local preferences that are not easy to recreate."
    );

    rule!(
        rules,
        "vscode-dir",
        "VS Code .vscode",
        Category::IdeArtifact,
        SafetyLevel::Risky,
        DirName(".vscode"),
        "VS Code workspace settings and launch configurations. May contain project-specific settings that are hard to recreate."
    );

    rule!(
        rules,
        "vim-swp",
        "Vim swap files (*.swp)",
        Category::IdeArtifact,
        SafetyLevel::Safe,
        FileGlob("*.swp"),
        "Vim swap files left over from editing sessions. Safe to delete when no editor has the file open."
    );

    rule!(
        rules,
        "vim-swo",
        "Vim swap files (*.swo)",
        Category::IdeArtifact,
        SafetyLevel::Safe,
        FileGlob("*.swo"),
        "Vim swap files left over from editing sessions. Safe to delete when no editor has the file open."
    );

    rule!(
        rules,
        "visual-studio-vs",
        "Visual Studio .vs",
        Category::IdeArtifact,
        SafetyLevel::Cautious,
        DirName(".vs"),
        "Visual Studio local settings and IntelliSense cache. Usually regenerated, but local run configs may be lost."
    );

    // -------------------------------------------------------------------------
    // Coverage
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "coverage-dir",
        "coverage directory",
        Category::Coverage,
        SafetyLevel::Safe,
        DirName("coverage"),
        ["package.json", "pytest.ini", "setup.cfg", ".coveragerc"],
        "Test coverage report output. Recreated by re-running the test suite with coverage enabled."
    );

    rule!(
        rules,
        "htmlcov",
        "Python htmlcov",
        Category::Coverage,
        SafetyLevel::Safe,
        DirName("htmlcov"),
        "Python coverage HTML report. Recreated by `coverage html`."
    );

    rule!(
        rules,
        "nyc-output",
        "nyc / Istanbul .nyc_output",
        Category::Coverage,
        SafetyLevel::Safe,
        DirName(".nyc_output"),
        "nyc (Istanbul) raw coverage data. Recreated by re-running tests with nyc."
    );

    rule!(
        rules,
        "coverage-file",
        "Python .coverage file",
        Category::Coverage,
        SafetyLevel::Safe,
        FileName(".coverage"),
        "Python coverage data file. Recreated by re-running the test suite with coverage."
    );

    // -------------------------------------------------------------------------
    // Logs
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "npm-debug-log",
        "npm debug log",
        Category::Logs,
        SafetyLevel::Safe,
        FileGlob("npm-debug.log*"),
        "npm debug log files. Safe to delete; they are written fresh on each error."
    );

    rule!(
        rules,
        "yarn-debug-log",
        "Yarn debug log",
        Category::Logs,
        SafetyLevel::Safe,
        FileGlob("yarn-debug.log*"),
        "Yarn debug log files. Safe to delete."
    );

    rule!(
        rules,
        "yarn-error-log",
        "Yarn error log",
        Category::Logs,
        SafetyLevel::Safe,
        FileGlob("yarn-error.log*"),
        "Yarn error log files. Safe to delete."
    );

    // -------------------------------------------------------------------------
    // Compiled / Generated
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "python-eggs",
        "Python .eggs",
        Category::CompiledGenerated,
        SafetyLevel::Safe,
        DirName(".eggs"),
        "Python eggs directory created by setuptools. Recreated by running the build."
    );

    rule!(
        rules,
        "python-egg-info",
        "Python *.egg-info",
        Category::CompiledGenerated,
        SafetyLevel::Safe,
        DirGlob("*.egg-info"),
        "Python package metadata directories created by setuptools. Recreated by `pip install -e .`."
    );

    rule!(
        rules,
        "typescript-buildinfo",
        "TypeScript *.tsbuildinfo",
        Category::CompiledGenerated,
        SafetyLevel::Safe,
        FileGlob("*.tsbuildinfo"),
        "TypeScript incremental compilation info. Recreated on the next `tsc` run."
    );

    rule!(
        rules,
        "terraform",
        "Terraform .terraform",
        Category::CompiledGenerated,
        SafetyLevel::Cautious,
        DirName(".terraform"),
        "Terraform provider plugins and modules. Recreated by `terraform init`, but may require re-running plan."
    );

    rule!(
        rules,
        "serverless",
        "Serverless Framework .serverless",
        Category::CompiledGenerated,
        SafetyLevel::Cautious,
        DirName(".serverless"),
        "Serverless Framework deployment package. Recreated by `serverless package`, but review before deploying."
    );

    // -------------------------------------------------------------------------
    // Miscellaneous
    // -------------------------------------------------------------------------
    rule!(
        rules,
        "docusaurus",
        "Docusaurus cache",
        Category::Misc,
        SafetyLevel::Safe,
        DirName(".docusaurus"),
        "Docusaurus static site generator cache. Recreated on the next build."
    );

    rule!(
        rules,
        "expo",
        "Expo cache",
        Category::Misc,
        SafetyLevel::Safe,
        DirName(".expo"),
        "Expo React Native project cache. Recreated on the next expo start."
    );

    rule!(
        rules,
        "meteor-local",
        "Meteor local build",
        Category::Misc,
        SafetyLevel::Safe,
        DirName("local"),
        [".meteor"],
        "Meteor local build directory. Recreated on the next `meteor` run."
    );

    rule!(
        rules,
        "stack-work",
        "Haskell Stack .stack-work",
        Category::Misc,
        SafetyLevel::Safe,
        DirName(".stack-work"),
        "Haskell Stack build artefacts. Recreated by `stack build`."
    );

    rule!(
        rules,
        "cabal-sandbox",
        "Haskell Cabal sandbox",
        Category::Misc,
        SafetyLevel::Safe,
        DirName(".cabal-sandbox"),
        "Haskell Cabal sandbox (legacy). Recreated by `cabal sandbox init`."
    );

    rule!(
        rules,
        "elixir-deps",
        "Elixir _deps",
        Category::Misc,
        SafetyLevel::Safe,
        DirName("_deps"),
        "Elixir / Phoenix Mix dependency directory. Recreated by `mix deps.get`."
    );

    rule!(
        rules,
        "zig-cache",
        "Zig zig-cache",
        Category::Misc,
        SafetyLevel::Safe,
        DirName("zig-cache"),
        "Zig compiler cache. Recreated on the next `zig build`."
    );

    rule!(
        rules,
        "zig-out",
        "Zig zig-out",
        Category::Misc,
        SafetyLevel::Safe,
        DirName("zig-out"),
        "Zig build output directory. Recreated on the next `zig build`."
    );

    rule!(
        rules,
        "zig-cache-dot",
        "Zig .zig-cache",
        Category::Misc,
        SafetyLevel::Safe,
        DirName(".zig-cache"),
        "Zig compiler cache (hidden variant). Recreated on the next `zig build`."
    );

    rules
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::rules::types::Category;

    #[test]
    fn catalog_is_not_empty() {
        let rules = builtin_rules();
        assert!(!rules.is_empty(), "built-in rule catalog must not be empty");
    }

    #[test]
    fn all_rule_ids_are_unique() {
        let rules = builtin_rules();
        let mut seen = HashSet::new();
        for rule in &rules {
            assert!(
                seen.insert(rule.id.as_str()),
                "duplicate rule id: {}",
                rule.id
            );
        }
    }

    #[test]
    fn all_rules_have_required_fields() {
        let rules = builtin_rules();
        for rule in &rules {
            assert!(!rule.id.is_empty(), "rule has empty id");
            assert!(!rule.name.is_empty(), "rule '{}' has empty name", rule.id);
            assert!(
                !rule.description.is_empty(),
                "rule '{}' has empty description",
                rule.id
            );
            assert!(
                !rule.id.contains('/'),
                "rule id '{}' must not contain slashes",
                rule.id
            );
        }
    }

    #[test]
    fn all_categories_have_at_least_one_rule() {
        let rules = builtin_rules();
        for category in Category::all() {
            let count = rules.iter().filter(|r| &r.category == category).count();
            assert!(
                count > 0,
                "category {:?} has no rules in the catalog",
                category
            );
        }
    }

    #[test]
    fn context_marker_rules_have_markers() {
        // Generic names that are meaningless without context should always
        // carry context markers.
        let ambiguous_names = ["build", "dist", "out", "vendor", "coverage", "env", "local"];
        let rules = builtin_rules();
        for rule in &rules {
            let name = match &rule.match_condition {
                MatchCondition::DirName(n) => n.as_str(),
                _ => continue,
            };
            if ambiguous_names.contains(&name) {
                assert!(
                    !rule.context_markers.is_empty(),
                    "rule '{}' matches ambiguous name '{}' but has no context markers",
                    rule.id,
                    name
                );
            }
        }
    }

    #[test]
    fn all_rules_are_enabled_by_default() {
        let rules = builtin_rules();
        for rule in &rules {
            assert!(rule.enabled, "rule '{}' is disabled by default", rule.id);
        }
    }
}
