use std::{collections::HashMap, path::Path, time::SystemTime};

use assembler::KclvmLibAssembler;
use command::Command;
use kclvm_ast::{
    ast::{Module, Program},
    MAIN_PKG,
};
use kclvm_parser::load_program;
use kclvm_query::apply_overrides;
use kclvm_runtime::ValueRef;
use kclvm_sema::resolver::resolve_program;
pub use runner::ExecProgramArgs;
use runner::{ExecProgramResult, KclvmRunner, KclvmRunnerOptions};
use tempfile::tempdir;

pub mod assembler;
pub mod command;
pub mod linker;
pub mod runner;

#[cfg(test)]
pub mod tests;

/// After the kcl program passed through kclvm-parser in the compiler frontend,
/// KCLVM needs to resolve ast, generate corresponding LLVM IR, dynamic link library or
/// executable file for kcl program in the compiler backend.
///
/// Method “execute” is the entry point for the compiler backend.
///
/// It returns the KCL program executing result as Result<a_json_string, an_err_string>,
/// and mainly takes "program" (ast.Program returned by kclvm-parser) as input.
///
/// "args" is the items selected by the user in the KCLVM CLI.
///
/// This method will first resolve “program” (ast.Program) and save the result to the "scope" (ProgramScope).
///
/// Then, dynamic link libraries is generated by KclvmAssembler, and method "KclvmAssembler::gen_libs"
/// will return dynamic link library paths in a "Vec<String>";
///
/// KclvmAssembler is mainly responsible for concurrent compilation of multiple files.
/// Single-file compilation in each thread in concurrent compilation is the responsibility of KclvmLibAssembler.
/// In the future, it may support the dynamic link library generation of multiple intermediate language.
/// KclvmLibAssembler currently only supports LLVM IR.
///
/// After linking all dynamic link libraries by KclvmLinker, method "KclvmLinker::link_all_libs" will return a path
/// for dynamic link library after linking.
///
/// At last, KclvmRunner will be constructed and call method "run" to execute the kcl program.
///
/// # Examples
///
/// ```
/// use kclvm_runner::{exec_program, ExecProgramArgs};
///
/// // Get default args
/// let mut args = ExecProgramArgs::default();
/// args.k_filename_list = vec!["./src/test_datas/init_check_order_0/main.k".to_string()];
///
/// // Resolve ast, generate libs, link libs and execute.
/// // Result is the kcl in json format.
/// let result = exec_program(&args, 0).unwrap();
/// ```
pub fn exec_program(
    args: &ExecProgramArgs,
    plugin_agent: u64,
) -> Result<ExecProgramResult, String> {
    // parse args from json string
    let opts = args.get_load_program_options();
    let k_files = &args.k_filename_list;
    let mut kcl_paths = Vec::<String>::new();
    let work_dir = args.work_dir.clone().unwrap_or_default();

    // join work_path with k_file_path
    for (_, file) in k_files.iter().enumerate() {
        match Path::new(&work_dir).join(file).to_str() {
            Some(str) => kcl_paths.push(String::from(str)),
            None => (),
        }
    }

    let kcl_paths_str = kcl_paths.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

    let mut program = load_program(kcl_paths_str.as_slice(), Some(opts))?;

    if let Err(err) = apply_overrides(&mut program, &args.overrides, &[], args.print_override_ast) {
        return Err(err.to_string());
    }

    let start_time = SystemTime::now();
    let exec_result = execute(program, plugin_agent, args);
    let escape_time = match SystemTime::now().duration_since(start_time) {
        Ok(dur) => dur.as_secs_f32(),
        Err(err) => return Err(err.to_string()),
    };
    let mut result = ExecProgramResult::default();
    result.escaped_time = escape_time.to_string();
    // Exec result is a JSON or YAML string.
    let exec_result = match exec_result {
        Ok(res) => res,
        Err(res) => {
            if res.is_empty() {
                return Ok(result);
            } else {
                return Err(res);
            }
        }
    };
    let kcl_val = match ValueRef::from_yaml_stream(&exec_result) {
        Ok(v) => v,
        Err(err) => return Err(err.to_string()),
    };
    let (json_result, yaml_result) = kcl_val.plan();
    result.json_result = json_result;
    if !args.disable_yaml_result {
        result.yaml_result = yaml_result;
    }
    Ok(result)
}

/// After the kcl program passed through kclvm-parser in the compiler frontend,
/// KCLVM needs to resolve ast, generate corresponding LLVM IR, dynamic link library or
/// executable file for kcl program in the compiler backend.
///
/// Method “execute” is the entry point for the compiler backend.
///
/// It returns the KCL program executing result as Result<a_json_string, an_err_string>,
/// and mainly takes "program" (ast.Program returned by kclvm-parser) as input.
///
/// "args" is the items selected by the user in the KCLVM CLI.
///
/// This method will first resolve “program” (ast.Program) and save the result to the "scope" (ProgramScope).
///
/// Then, dynamic link libraries is generated by KclvmAssembler, and method "KclvmAssembler::gen_libs"
/// will return dynamic link library paths in a "Vec<String>";
///
/// KclvmAssembler is mainly responsible for concurrent compilation of multiple files.
/// Single-file compilation in each thread in concurrent compilation is the responsibility of KclvmLibAssembler.
/// In the future, it may support the dynamic link library generation of multiple intermediate language.
/// KclvmLibAssembler currently only supports LLVM IR.
///
/// After linking all dynamic link libraries by KclvmLinker, method "KclvmLinker::link_all_libs" will return a path
/// for dynamic link library after linking.
///
/// At last, KclvmRunner will be constructed and call method "run" to execute the kcl program.
///
/// # Examples
///
/// ```
/// use kclvm_runner::{execute, runner::ExecProgramArgs};
/// use kclvm_parser::load_program;
/// use kclvm_ast::ast::Program;
/// // plugin_agent is the address of plugin.
/// let plugin_agent = 0;
/// // Get default args
/// let args = ExecProgramArgs::default();
/// let opts = args.get_load_program_options();
///
/// // Parse kcl file
/// let kcl_path = "./src/test_datas/init_check_order_0/main.k";
/// let prog = load_program(&[kcl_path], Some(opts)).unwrap();
///     
/// // Resolve ast, generate libs, link libs and execute.
/// // Result is the kcl in json format.
/// let result = execute(prog, plugin_agent, &args).unwrap();
/// ```
pub fn execute(
    mut program: Program,
    plugin_agent: u64,
    args: &ExecProgramArgs,
) -> Result<String, String> {
    // Resolve ast
    let scope = resolve_program(&mut program);
    scope.check_scope_diagnostics();

    // Create a temp entry file and the temp dir will be delete automatically
    let temp_dir = tempdir().unwrap();
    let temp_dir_path = temp_dir.path().to_str().unwrap();
    let temp_entry_file = temp_file(temp_dir_path);

    // Generate libs
    let lib_paths = assembler::KclvmAssembler::new(
        program,
        scope,
        temp_entry_file.clone(),
        KclvmLibAssembler::LLVM,
    )
    .gen_libs();

    // Link libs
    let lib_suffix = Command::get_lib_suffix();
    let temp_out_lib_file = format!("{}.out{}", temp_entry_file, lib_suffix);
    let lib_path = linker::KclvmLinker::link_all_libs(lib_paths, temp_out_lib_file);

    // Run
    let runner = KclvmRunner::new(
        lib_path.as_str(),
        Some(KclvmRunnerOptions {
            plugin_agent_ptr: plugin_agent,
        }),
    );
    let result = runner.run(args);

    // Clean temp files
    remove_file(&lib_path);
    clean_tmp_files(&temp_entry_file, &lib_suffix);
    result
}

/// `execute_module` can directly execute the ast `Module`.
/// `execute_module` constructs `Program` with default pkg name `MAIN_PKG`,
/// and calls method `execute` with default `plugin_agent` and `ExecProgramArgs`.
/// For more information, see doc above method `execute`.
pub fn execute_module(mut m: Module) -> Result<String, String> {
    m.pkg = MAIN_PKG.to_string();

    let mut pkgs = HashMap::new();
    pkgs.insert(MAIN_PKG.to_string(), vec![m]);

    let prog = Program {
        root: MAIN_PKG.to_string(),
        main: MAIN_PKG.to_string(),
        pkgs,
        cmd_args: vec![],
        cmd_overrides: vec![],
    };

    execute(prog, 0, &ExecProgramArgs::default())
}

/// Clean all the tmp files generated during lib generating and linking.
#[inline]
fn clean_tmp_files(temp_entry_file: &String, lib_suffix: &String) {
    let temp_entry_lib_file = format!("{}{}", temp_entry_file, lib_suffix);
    remove_file(&temp_entry_lib_file);
}

#[inline]
fn remove_file(file: &str) {
    if Path::new(&file).exists() {
        std::fs::remove_file(&file).unwrap_or_else(|_| panic!("{} not found", file));
    }
}

/// Returns a temporary file name consisting of timestamp and process id.
fn temp_file(dir: &str) -> String {
    let timestamp = chrono::Local::now().timestamp_nanos();
    let id = std::process::id();
    let file = format!("{}_{}", id, timestamp);
    std::fs::create_dir_all(dir).unwrap_or_else(|_| panic!("{} not found", dir));
    Path::new(dir).join(file).to_str().unwrap().to_string()
}
