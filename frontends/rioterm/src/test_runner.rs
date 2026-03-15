//! Intelligent test runner — auto-detects project type and runs tests.
//! Supports: cargo test, npm test, pytest, go test, rspec, gradle test

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum TestRunner {
    CargoTest,
    NpmTest,
    Pytest,
    GoTest,
    RSpec,
    GradleTest,
    MavenTest,
    Jest,
}

impl TestRunner {
    pub fn name(&self) -> &str {
        match self {
            TestRunner::CargoTest => "cargo test",
            TestRunner::NpmTest => "npm test",
            TestRunner::Pytest => "pytest",
            TestRunner::GoTest => "go test",
            TestRunner::RSpec => "bundle exec rspec",
            TestRunner::GradleTest => "gradle test",
            TestRunner::MavenTest => "mvn test",
            TestRunner::Jest => "npx jest",
        }
    }

    pub fn command(&self) -> (&str, Vec<&str>) {
        match self {
            TestRunner::CargoTest => ("cargo", vec!["test"]),
            TestRunner::NpmTest => ("npm", vec!["test"]),
            TestRunner::Pytest => ("pytest", vec!["-v"]),
            TestRunner::GoTest => ("go", vec!["test", "./..."]),
            TestRunner::RSpec => ("bundle", vec!["exec", "rspec"]),
            TestRunner::GradleTest => ("gradle", vec!["test"]),
            TestRunner::MavenTest => ("mvn", vec!["test"]),
            TestRunner::Jest => ("npx", vec!["jest", "--verbose"]),
        }
    }

    pub fn failed_command(&self) -> (&str, Vec<&str>) {
        match self {
            TestRunner::CargoTest => ("cargo", vec!["test", "--", "--failed"]),
            TestRunner::Pytest => ("pytest", vec!["--lf", "-v"]),
            TestRunner::Jest => ("npx", vec!["jest", "--onlyFailures"]),
            TestRunner::RSpec => ("bundle", vec!["exec", "rspec", "--only-failures"]),
            _ => self.command(), // Fallback to full run
        }
    }
}

/// Detect which test runner to use based on project files
pub fn detect_test_runner(dir: &Path) -> Option<TestRunner> {
    if dir.join("Cargo.toml").exists() {
        return Some(TestRunner::CargoTest);
    }
    if dir.join("package.json").exists() {
        // Check if Jest is configured
        if dir.join("jest.config.js").exists()
            || dir.join("jest.config.ts").exists()
            || dir.join("jest.config.mjs").exists()
        {
            return Some(TestRunner::Jest);
        }
        // Check package.json for jest in dependencies
        if let Ok(content) = std::fs::read_to_string(dir.join("package.json")) {
            if content.contains("\"jest\"") {
                return Some(TestRunner::Jest);
            }
        }
        return Some(TestRunner::NpmTest);
    }
    if dir.join("go.mod").exists() {
        return Some(TestRunner::GoTest);
    }
    if dir.join("Gemfile").exists() && dir.join("spec").exists() {
        return Some(TestRunner::RSpec);
    }
    if dir.join("pyproject.toml").exists()
        || dir.join("setup.py").exists()
        || dir.join("pytest.ini").exists()
        || dir.join("conftest.py").exists()
    {
        return Some(TestRunner::Pytest);
    }
    if dir.join("build.gradle").exists() || dir.join("build.gradle.kts").exists() {
        return Some(TestRunner::GradleTest);
    }
    if dir.join("pom.xml").exists() {
        return Some(TestRunner::MavenTest);
    }
    None
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub runner: TestRunner,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
    pub failed_tests: Vec<String>,
    pub output: String,
}

/// Parse test output to extract results (basic parsing)
pub fn parse_test_output(runner: &TestRunner, output: &str) -> TestResult {
    let mut result = TestResult {
        runner: runner.clone(),
        passed: 0,
        failed: 0,
        skipped: 0,
        duration_ms: 0,
        failed_tests: Vec::new(),
        output: output.to_string(),
    };

    match runner {
        TestRunner::CargoTest => {
            // Parse "test result: ok. X passed; Y failed; Z ignored"
            for line in output.lines() {
                if line.starts_with("test result:") {
                    if let Some(rest) = line.strip_prefix("test result: ") {
                        for part in rest.split(';') {
                            let part = part.trim();
                            // Find the number immediately before the keyword
                            let extract_num = || -> usize {
                                part.split_whitespace()
                                    .find_map(|w| w.parse::<usize>().ok())
                                    .unwrap_or(0)
                            };
                            if part.ends_with("passed") {
                                result.passed = extract_num();
                            } else if part.ends_with("failed") {
                                result.failed = extract_num();
                            } else if part.ends_with("ignored") {
                                result.skipped = extract_num();
                            }
                        }
                    }
                }
                // Track failed test names
                if line.contains("FAILED") && line.starts_with("test ") {
                    let name = line
                        .strip_prefix("test ")
                        .and_then(|s| s.split(" ...").next())
                        .unwrap_or(line);
                    result.failed_tests.push(name.to_string());
                }
            }
        }
        TestRunner::Pytest => {
            // Parse "X passed, Y failed, Z skipped"
            for line in output.lines().rev() {
                if line.contains("passed")
                    || line.contains("failed")
                    || line.contains("error")
                {
                    for part in line.split(',') {
                        let part = part.trim();
                        // Extract the first numeric token from the part
                        let num = part
                            .split_whitespace()
                            .find_map(|w| w.parse::<usize>().ok())
                            .unwrap_or(0);
                        if part.contains("passed") {
                            result.passed = num;
                        } else if part.contains("failed") {
                            result.failed = num;
                        } else if part.contains("skipped") {
                            result.skipped = num;
                        }
                    }
                    break;
                }
            }
        }
        _ => {
            // Generic: count "PASS"/"FAIL" lines
            for line in output.lines() {
                let upper = line.to_uppercase();
                if upper.contains("PASS") {
                    result.passed += 1;
                }
                if upper.contains("FAIL") {
                    result.failed += 1;
                }
                if upper.contains("SKIP") || upper.contains("PENDING") {
                    result.skipped += 1;
                }
            }
        }
    }

    result
}

/// Test history for flaky test detection
#[derive(Debug, Default)]
pub struct TestHistory {
    runs: Vec<TestResult>,
    max_runs: usize,
}

impl TestHistory {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            max_runs: 100,
        }
    }

    pub fn record(&mut self, result: TestResult) {
        self.runs.push(result);
        if self.runs.len() > self.max_runs {
            self.runs.remove(0);
        }
    }

    /// Find flaky tests — tests that pass sometimes and fail sometimes
    pub fn flaky_tests(&self) -> Vec<String> {
        let mut fail_count: HashMap<String, usize> = HashMap::new();

        for run in &self.runs {
            for name in &run.failed_tests {
                *fail_count.entry(name.clone()).or_default() += 1;
            }
        }

        // A test is flaky if it failed in some runs but not all
        let total_runs = self.runs.len();
        fail_count
            .into_iter()
            .filter(|(_, count)| *count > 0 && *count < total_runs)
            .map(|(name, _)| name)
            .collect()
    }

    pub fn last_result(&self) -> Option<&TestResult> {
        self.runs.last()
    }

    pub fn total_runs(&self) -> usize {
        self.runs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cargo() {
        let dir = std::env::current_dir().unwrap();
        // We're in a Rust project, should detect cargo
        let runner = detect_test_runner(&dir);
        assert_eq!(runner, Some(TestRunner::CargoTest));
    }

    #[test]
    fn test_parse_cargo_output() {
        let output = "test foo::bar ... ok\ntest foo::baz ... FAILED\n\ntest result: ok. 5 passed; 1 failed; 2 ignored; 0 measured";
        let result = parse_test_output(&TestRunner::CargoTest, output);
        assert_eq!(result.passed, 5);
        assert_eq!(result.failed, 1);
        assert_eq!(result.skipped, 2);
        assert!(result.failed_tests.contains(&"foo::baz".to_string()));
    }

    #[test]
    fn test_parse_pytest_output() {
        let output = "====== 10 passed, 2 failed, 1 skipped in 3.45s ======";
        let result = parse_test_output(&TestRunner::Pytest, output);
        assert_eq!(result.passed, 10);
        assert_eq!(result.failed, 2);
        assert_eq!(result.skipped, 1);
    }
}
