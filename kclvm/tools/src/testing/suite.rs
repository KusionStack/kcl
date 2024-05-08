use std::{fs::remove_file, path::Path};

use crate::testing::{TestCaseInfo, TestOptions, TestResult, TestRun};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use kclvm_ast::ast;
use kclvm_driver::{get_kcl_files, get_pkg_list};
use kclvm_parser::{parse_file_force_errors, ParseSessionRef};
#[cfg(feature = "llvm")]
use kclvm_runner::build_program;
#[cfg(not(feature = "llvm"))]
use kclvm_runner::exec_program;
#[cfg(feature = "llvm")]
use kclvm_runner::runner::ProgramRunner;
use kclvm_runner::ExecProgramArgs;
use std::time::Instant;

/// File suffix for test files.
pub const TEST_FILE_SUFFIX: &str = "_test.k";
/// Prefix for test suite names.
pub const TEST_SUITE_PREFIX: &str = "test_";

const TEST_MAIN_FILE: &str = "_kcl_test.k";
const TEST_CASE_RUN_OPTION: &str = "_kcl_test_case_run";
const TEST_MAIN_FILE_PREFIX: &str = r#"
# Auto generated by the kcl test tool; DO NOT EDIT!

_kcl_test_case_run = option("_kcl_test_case_run", type="str", default="")

"#;

pub struct TestSuite {
    /// Package path of the test suite. e.g. ./path/to/pkg
    pub pkg: String,
    /// List of normal files in the package.
    pub normal_files: Vec<String>,
    /// List of normal files without the `_test.k` suffix in the package.
    pub test_files: Vec<String>,
    // Map of test cases in the test suite.
    pub cases: IndexMap<String, TestCase>,
    // Flag indicating whether the test suite should be skipped.
    pub skip: bool,
}

impl TestRun for TestSuite {
    type Options = TestOptions;
    type Result = TestResult;

    /// Run the test suite with the given options and return the result.
    fn run(&self, opts: &Self::Options) -> Result<Self::Result> {
        let mut result = TestResult::default();
        // Skip test suite if marked as skipped or if there are no test cases.
        if self.skip || self.cases.is_empty() {
            return Ok(result);
        }
        // Generate the test main entry file.
        let main_file = self.gen_test_main_file()?;
        // Set up execution arguments.
        let mut args = ExecProgramArgs {
            k_filename_list: self.get_input_files(&main_file),
            overrides: vec![],
            disable_yaml_result: true,
            ..opts.exec_args.clone()
        };
        // Build the program.
        #[cfg(feature = "llvm")]
        let artifact = build_program::<String>(ParseSessionRef::default(), &args, None)?;
        // Test every case in the suite.
        for (name, _) in &self.cases {
            args.args = vec![ast::CmdArgSpec {
                name: TEST_CASE_RUN_OPTION.into(),
                value: format!("{:?}", name),
            }];
            let start = Instant::now();
            #[cfg(feature = "llvm")]
            let exec_result = artifact.run(&args)?;
            #[cfg(not(feature = "llvm"))]
            let exec_result = exec_program(ParseSessionRef::default(), &args)?;
            // Check if there was an error.
            let error = if exec_result.err_message.is_empty() {
                None
            } else {
                Some(anyhow!("{}", exec_result.err_message))
            };
            // Check if the fail_fast option is enabled and there was an error.
            let fail_fast = error.is_some() && opts.fail_fast;
            // Add test case information to the result.
            result.info.insert(
                name.clone(),
                TestCaseInfo {
                    log_message: exec_result.log_message.clone(),
                    duration: Instant::now() - start,
                    error,
                },
            );
            if fail_fast {
                break;
            }
        }
        // Remove the temp test main file
        if opts.exec_args.debug == 0 {
            remove_file(main_file)?;
        }
        Ok(result)
    }
}

impl TestSuite {
    fn gen_test_main_file(&self) -> Result<String> {
        let test_codes = self
            .cases
            .keys()
            .map(|c| format!("if {} == '{}': {}()", TEST_CASE_RUN_OPTION, c, c))
            .collect::<Vec<String>>();
        let code = format!("{}{}", TEST_MAIN_FILE_PREFIX, test_codes.join("\n"));
        let path = Path::new(&self.pkg).join(TEST_MAIN_FILE);
        let test_main_file = path
            .to_str()
            .ok_or(anyhow!("{} is not found", TEST_MAIN_FILE))?;
        std::fs::write(test_main_file, code)?;
        Ok(test_main_file.into())
    }

    fn get_input_files(&self, main_file: &str) -> Vec<String> {
        // Construct test package files.
        let mut files = vec![];
        let mut normal_files = self.normal_files.clone();
        let mut test_files = self.test_files.clone();
        files.append(&mut normal_files);
        files.append(&mut test_files);
        files.push(main_file.into());
        files
    }
}

pub struct TestCase;

/// Load test suite from path
pub fn load_test_suites<P: AsRef<str>>(path: P, opts: &TestOptions) -> Result<Vec<TestSuite>> {
    let pkg_list = get_pkg_list(path.as_ref())?;
    let mut suites = vec![];
    for pkg in &pkg_list {
        let (normal_files, test_files) = get_test_files(pkg)?;
        let mut cases = IndexMap::new();
        for file in &test_files {
            let module = parse_file_force_errors(file, None)?;
            for stmt in &module.body {
                if let ast::Stmt::Assign(assign_stmt) = &stmt.node {
                    if let ast::Expr::Lambda(_lambda_expr) = &assign_stmt.value.node {
                        for target in &assign_stmt.targets {
                            let func_name = target.node.get_name();
                            if is_test_suite(&func_name) && should_run(&opts.run_regexp, &func_name)
                            {
                                cases.insert(func_name.clone(), TestCase {});
                            }
                        }
                    }
                }
            }
        }
        suites.push(TestSuite {
            pkg: pkg.clone(),
            cases,
            normal_files,
            test_files,
            skip: false,
        });
    }
    Ok(suites)
}

#[inline]
fn get_test_files<P: AsRef<Path>>(pkg: P) -> Result<(Vec<String>, Vec<String>)> {
    let files = get_kcl_files(pkg, false)?;
    let normal_files = files
        .iter()
        .filter(|x| !x.starts_with('_') && !x.ends_with(TEST_FILE_SUFFIX))
        .cloned()
        .collect::<Vec<String>>();
    let test_files = files
        .iter()
        .filter(|x| !x.starts_with('_') && x.ends_with(TEST_FILE_SUFFIX))
        .cloned()
        .collect::<Vec<String>>();
    Ok((normal_files, test_files))
}

#[inline]
fn is_test_suite(name: &str) -> bool {
    name.starts_with(TEST_SUITE_PREFIX)
}

#[inline]
fn should_run(run_regexp: &str, name: &str) -> bool {
    if !run_regexp.is_empty() {
        regex::Regex::new(run_regexp)
            .map(|re| re.is_match(name))
            .unwrap_or_default()
    } else {
        true
    }
}
