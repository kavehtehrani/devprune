use super::types::{Category, MatchCondition, Rule, SafetyLevel};

/// Returns all built-in detection rules.
pub fn builtin_rules() -> Vec<Rule> {
    let mut rules = Vec::new();

    // -------------------------------------------------------------------------
    // Dependencies
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "npm-node-modules".to_string(),
        name: "npm / Node.js node_modules".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("node_modules".to_string()),
        context_markers: vec![],
        description: "npm / Yarn / pnpm dependency cache. Recreated by running the package manager install command.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "bower-components".to_string(),
        name: "Bower bower_components".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("bower_components".to_string()),
        context_markers: vec![],
        description: "Bower front-end package directory. Recreated by running `bower install`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "php-vendor".to_string(),
        name: "Composer vendor (PHP)".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("vendor".to_string()),
        context_markers: vec!["composer.json".to_string()],
        description: "PHP Composer dependency directory. Recreated by running `composer install`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "go-vendor".to_string(),
        name: "Go vendor".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("vendor".to_string()),
        context_markers: vec!["go.mod".to_string()],
        description: "Go module vendor directory. Recreated by running `go mod vendor`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "ruby-bundle".to_string(),
        name: "Bundler .bundle".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".bundle".to_string()),
        context_markers: vec![],
        description: "Ruby Bundler local gem cache. Recreated by running `bundle install`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "cocoapods".to_string(),
        name: "CocoaPods Pods".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("Pods".to_string()),
        context_markers: vec!["Podfile".to_string()],
        description: "CocoaPods dependency directory. Recreated by running `pod install`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "dart-pub-cache".to_string(),
        name: "Dart / Flutter .pub-cache".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".pub-cache".to_string()),
        context_markers: vec![],
        description: "Dart / Flutter pub package cache. Recreated automatically by pub.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "elm-stuff".to_string(),
        name: "Elm elm-stuff".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("elm-stuff".to_string()),
        context_markers: vec![],
        description: "Elm package cache and compiled artefacts. Recreated by `elm make`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "jspm-packages".to_string(),
        name: "jspm jspm_packages".to_string(),
        category: Category::Dependencies,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("jspm_packages".to_string()),
        context_markers: vec![],
        description: "jspm package directory. Recreated by `jspm install`.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // Build Outputs
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "cargo-target".to_string(),
        name: "Cargo target (Rust)".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("target".to_string()),
        context_markers: vec!["Cargo.toml".to_string()],
        description: "Rust Cargo build output. Recreated by `cargo build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "maven-target".to_string(),
        name: "Maven target (Java)".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("target".to_string()),
        context_markers: vec!["pom.xml".to_string()],
        description: "Maven build output directory. Recreated by `mvn compile`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "sbt-target".to_string(),
        name: "sbt target (Scala)".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("target".to_string()),
        context_markers: vec!["build.sbt".to_string()],
        description: "sbt / Scala build output directory. Recreated by running `sbt compile`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "gradle-build".to_string(),
        name: "Gradle build output".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("build".to_string()),
        context_markers: vec!["build.gradle".to_string()],
        description: "Gradle build output directory. Recreated by running the Gradle build task.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "cmake-build".to_string(),
        name: "CMake build output".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("build".to_string()),
        context_markers: vec!["CMakeLists.txt".to_string()],
        description: "CMake build output directory. Recreated by running `cmake --build .`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "python-build".to_string(),
        name: "Python build output".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("build".to_string()),
        context_markers: vec!["setup.py".to_string()],
        description: "Python setuptools build directory. Recreated by `python setup.py build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "npm-build".to_string(),
        name: "npm / Node.js build output".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("build".to_string()),
        context_markers: vec!["package.json".to_string()],
        description: "Node.js / React build output directory. Recreated by running the project's build script.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "npm-dist".to_string(),
        name: "npm dist output".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("dist".to_string()),
        context_markers: vec!["package.json".to_string()],
        description: "Node.js distribution / bundle output. Recreated by running the build script.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "python-dist".to_string(),
        name: "Python dist output".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("dist".to_string()),
        context_markers: vec!["setup.py".to_string(), "pyproject.toml".to_string()],
        description: "Python distribution packages (wheels, sdists). Recreated by `python -m build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "gradle-out".to_string(),
        name: "Gradle out directory".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName("out".to_string()),
        context_markers: vec!["build.gradle".to_string()],
        description: "Gradle / IntelliJ out directory. Usually safe to delete, but verify if it contains generated sources.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "intellij-out".to_string(),
        name: "IntelliJ out directory".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName("out".to_string()),
        context_markers: vec![".classpath".to_string()],
        description: "IntelliJ IDEA compiled class output directory. Usually recreatable via a project rebuild.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "elixir-build".to_string(),
        name: "Elixir _build".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("_build".to_string()),
        context_markers: vec!["mix.exs".to_string()],
        description: "Elixir / Erlang Mix build artefacts. Recreated by `mix compile`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "swift-build".to_string(),
        name: "Swift Package Manager .build".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".build".to_string()),
        context_markers: vec!["Package.swift".to_string()],
        description: "Swift Package Manager build directory. Recreated by `swift build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "cmake-build-glob".to_string(),
        name: "CMake cmake-build-* directories".to_string(),
        category: Category::BuildOutput,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirGlob("cmake-build-*".to_string()),
        context_markers: vec![],
        description: "CMake build directories created by IDEs such as CLion. Safe to delete and regenerate.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // Caches
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "python-pycache".to_string(),
        name: "Python __pycache__".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("__pycache__".to_string()),
        context_markers: vec![],
        description: "Python bytecode cache. Automatically recreated on next import.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "pytest-cache".to_string(),
        name: "pytest cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".pytest_cache".to_string()),
        context_markers: vec![],
        description: "pytest results and cache. Recreated on the next test run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "mypy-cache".to_string(),
        name: "mypy cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".mypy_cache".to_string()),
        context_markers: vec![],
        description: "mypy type-checker cache. Recreated on the next mypy run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "ruff-cache".to_string(),
        name: "Ruff cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".ruff_cache".to_string()),
        context_markers: vec![],
        description: "Ruff linter cache. Recreated on the next ruff run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "parcel-cache".to_string(),
        name: "Parcel cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".parcel-cache".to_string()),
        context_markers: vec![],
        description: "Parcel bundler cache. Recreated on the next build.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "turbo-cache".to_string(),
        name: "Turborepo cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".turbo".to_string()),
        context_markers: vec![],
        description: "Turborepo build cache. Recreated on the next turbo run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "next-cache".to_string(),
        name: "Next.js cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".next".to_string()),
        context_markers: vec!["package.json".to_string()],
        description: "Next.js build cache and compiled output. Recreated by `next build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "nuxt-cache".to_string(),
        name: "Nuxt.js cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".nuxt".to_string()),
        context_markers: vec!["package.json".to_string()],
        description: "Nuxt.js build cache. Recreated by `nuxt build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "angular-cache".to_string(),
        name: "Angular cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".angular".to_string()),
        context_markers: vec![],
        description: "Angular CLI build cache. Recreated on the next build.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "svelte-kit-cache".to_string(),
        name: "SvelteKit cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".svelte-kit".to_string()),
        context_markers: vec![],
        description: "SvelteKit generated files and cache. Recreated by running the dev/build command.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "gradle-cache".to_string(),
        name: "Gradle wrapper cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".gradle".to_string()),
        context_markers: vec![],
        description: "Gradle build cache and wrapper files. Recreated on the next Gradle run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "sass-cache".to_string(),
        name: "Sass cache".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".sass-cache".to_string()),
        context_markers: vec![],
        description: "Sass/SCSS compiler cache. Recreated on the next compilation.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "eslint-cache".to_string(),
        name: "ESLint cache file".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileName(".eslintcache".to_string()),
        context_markers: vec![],
        description: "ESLint lint result cache. Recreated on the next eslint run with --cache.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "stylelint-cache".to_string(),
        name: "Stylelint cache file".to_string(),
        category: Category::Cache,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileName(".stylelintcache".to_string()),
        context_markers: vec![],
        description: "Stylelint lint result cache. Recreated on the next stylelint run with --cache.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // Virtual Environments
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "python-venv".to_string(),
        name: "Python .venv".to_string(),
        category: Category::VirtualEnv,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName(".venv".to_string()),
        context_markers: vec![
            "requirements.txt".to_string(),
            "pyproject.toml".to_string(),
            "setup.py".to_string(),
        ],
        description: "Python virtual environment. Recreated by `python -m venv .venv && pip install -r requirements.txt`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "python-venv-bare".to_string(),
        name: "Python venv directory".to_string(),
        category: Category::VirtualEnv,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName("venv".to_string()),
        context_markers: vec![
            "requirements.txt".to_string(),
            "pyproject.toml".to_string(),
            "setup.py".to_string(),
        ],
        description: "Python virtual environment. Recreated by `python -m venv venv && pip install -r requirements.txt`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "python-env".to_string(),
        name: "Python env directory".to_string(),
        category: Category::VirtualEnv,
        safety: SafetyLevel::Risky,
        match_condition: MatchCondition::DirName("env".to_string()),
        context_markers: vec![
            "requirements.txt".to_string(),
            "pyproject.toml".to_string(),
            "setup.py".to_string(),
        ],
        description: "Python virtual environment with a generic name. Verify contents before deleting; the name `env` is ambiguous.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "conda-env".to_string(),
        name: "Conda environment".to_string(),
        category: Category::VirtualEnv,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName(".conda".to_string()),
        context_markers: vec![],
        description: "Conda environment directory. Recreatable, but custom packages may need reinstalling.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "tox-env".to_string(),
        name: "tox environments".to_string(),
        category: Category::VirtualEnv,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".tox".to_string()),
        context_markers: vec!["tox.ini".to_string()],
        description: "tox test automation virtual environments. Recreated on the next `tox` run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "nox-env".to_string(),
        name: "nox environments".to_string(),
        category: Category::VirtualEnv,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".nox".to_string()),
        context_markers: vec!["noxfile.py".to_string()],
        description: "nox test automation virtual environments. Recreated on the next `nox` run.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // IDE Artifacts
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "jetbrains-idea".to_string(),
        name: "JetBrains .idea".to_string(),
        category: Category::IdeArtifact,
        safety: SafetyLevel::Risky,
        match_condition: MatchCondition::DirName(".idea".to_string()),
        context_markers: vec![],
        description: "JetBrains IDE project settings. Contains run configs and local preferences that are not easy to recreate.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "vscode-dir".to_string(),
        name: "VS Code .vscode".to_string(),
        category: Category::IdeArtifact,
        safety: SafetyLevel::Risky,
        match_condition: MatchCondition::DirName(".vscode".to_string()),
        context_markers: vec![],
        description: "VS Code workspace settings and launch configurations. May contain project-specific settings that are hard to recreate.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "vim-swp".to_string(),
        name: "Vim swap files (*.swp)".to_string(),
        category: Category::IdeArtifact,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileGlob("*.swp".to_string()),
        context_markers: vec![],
        description: "Vim swap files left over from editing sessions. Safe to delete when no editor has the file open.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "vim-swo".to_string(),
        name: "Vim swap files (*.swo)".to_string(),
        category: Category::IdeArtifact,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileGlob("*.swo".to_string()),
        context_markers: vec![],
        description: "Vim swap files left over from editing sessions. Safe to delete when no editor has the file open.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "visual-studio-vs".to_string(),
        name: "Visual Studio .vs".to_string(),
        category: Category::IdeArtifact,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName(".vs".to_string()),
        context_markers: vec![],
        description: "Visual Studio local settings and IntelliSense cache. Usually regenerated, but local run configs may be lost.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // Coverage
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "coverage-dir".to_string(),
        name: "coverage directory".to_string(),
        category: Category::Coverage,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("coverage".to_string()),
        context_markers: vec![
            "package.json".to_string(),
            "pytest.ini".to_string(),
            "setup.cfg".to_string(),
            ".coveragerc".to_string(),
        ],
        description: "Test coverage report output. Recreated by re-running the test suite with coverage enabled.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "htmlcov".to_string(),
        name: "Python htmlcov".to_string(),
        category: Category::Coverage,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("htmlcov".to_string()),
        context_markers: vec![],
        description: "Python coverage HTML report. Recreated by `coverage html`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "nyc-output".to_string(),
        name: "nyc / Istanbul .nyc_output".to_string(),
        category: Category::Coverage,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".nyc_output".to_string()),
        context_markers: vec![],
        description: "nyc (Istanbul) raw coverage data. Recreated by re-running tests with nyc.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "coverage-file".to_string(),
        name: "Python .coverage file".to_string(),
        category: Category::Coverage,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileName(".coverage".to_string()),
        context_markers: vec![],
        description: "Python coverage data file. Recreated by re-running the test suite with coverage.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // Logs
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "npm-debug-log".to_string(),
        name: "npm debug log".to_string(),
        category: Category::Logs,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileGlob("npm-debug.log*".to_string()),
        context_markers: vec![],
        description: "npm debug log files. Safe to delete; they are written fresh on each error.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "yarn-debug-log".to_string(),
        name: "Yarn debug log".to_string(),
        category: Category::Logs,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileGlob("yarn-debug.log*".to_string()),
        context_markers: vec![],
        description: "Yarn debug log files. Safe to delete.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "yarn-error-log".to_string(),
        name: "Yarn error log".to_string(),
        category: Category::Logs,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileGlob("yarn-error.log*".to_string()),
        context_markers: vec![],
        description: "Yarn error log files. Safe to delete.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // Compiled / Generated
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "python-eggs".to_string(),
        name: "Python .eggs".to_string(),
        category: Category::CompiledGenerated,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".eggs".to_string()),
        context_markers: vec![],
        description: "Python eggs directory created by setuptools. Recreated by running the build.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "python-egg-info".to_string(),
        name: "Python *.egg-info".to_string(),
        category: Category::CompiledGenerated,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirGlob("*.egg-info".to_string()),
        context_markers: vec![],
        description: "Python package metadata directories created by setuptools. Recreated by `pip install -e .`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "typescript-buildinfo".to_string(),
        name: "TypeScript *.tsbuildinfo".to_string(),
        category: Category::CompiledGenerated,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::FileGlob("*.tsbuildinfo".to_string()),
        context_markers: vec![],
        description: "TypeScript incremental compilation info. Recreated on the next `tsc` run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "terraform".to_string(),
        name: "Terraform .terraform".to_string(),
        category: Category::CompiledGenerated,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName(".terraform".to_string()),
        context_markers: vec![],
        description: "Terraform provider plugins and modules. Recreated by `terraform init`, but may require re-running plan.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "serverless".to_string(),
        name: "Serverless Framework .serverless".to_string(),
        category: Category::CompiledGenerated,
        safety: SafetyLevel::Cautious,
        match_condition: MatchCondition::DirName(".serverless".to_string()),
        context_markers: vec![],
        description: "Serverless Framework deployment package. Recreated by `serverless package`, but review before deploying.".to_string(),
        enabled: true,
    });

    // -------------------------------------------------------------------------
    // Miscellaneous
    // -------------------------------------------------------------------------
    rules.push(Rule {
        id: "docusaurus".to_string(),
        name: "Docusaurus cache".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".docusaurus".to_string()),
        context_markers: vec![],
        description: "Docusaurus static site generator cache. Recreated on the next build.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "expo".to_string(),
        name: "Expo cache".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".expo".to_string()),
        context_markers: vec![],
        description: "Expo React Native project cache. Recreated on the next expo start.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "meteor-local".to_string(),
        name: "Meteor local build".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("local".to_string()),
        context_markers: vec![".meteor".to_string()],
        description: "Meteor local build directory. Recreated on the next `meteor` run.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "stack-work".to_string(),
        name: "Haskell Stack .stack-work".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".stack-work".to_string()),
        context_markers: vec![],
        description: "Haskell Stack build artefacts. Recreated by `stack build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "cabal-sandbox".to_string(),
        name: "Haskell Cabal sandbox".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".cabal-sandbox".to_string()),
        context_markers: vec![],
        description: "Haskell Cabal sandbox (legacy). Recreated by `cabal sandbox init`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "elixir-deps".to_string(),
        name: "Elixir _deps".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("_deps".to_string()),
        context_markers: vec![],
        description: "Elixir / Phoenix Mix dependency directory. Recreated by `mix deps.get`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "zig-cache".to_string(),
        name: "Zig zig-cache".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("zig-cache".to_string()),
        context_markers: vec![],
        description: "Zig compiler cache. Recreated on the next `zig build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "zig-out".to_string(),
        name: "Zig zig-out".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName("zig-out".to_string()),
        context_markers: vec![],
        description: "Zig build output directory. Recreated on the next `zig build`.".to_string(),
        enabled: true,
    });

    rules.push(Rule {
        id: "zig-cache-dot".to_string(),
        name: "Zig .zig-cache".to_string(),
        category: Category::Misc,
        safety: SafetyLevel::Safe,
        match_condition: MatchCondition::DirName(".zig-cache".to_string()),
        context_markers: vec![],
        description: "Zig compiler cache (hidden variant). Recreated on the next `zig build`.".to_string(),
        enabled: true,
    });

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
