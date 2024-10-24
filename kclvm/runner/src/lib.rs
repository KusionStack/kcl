use std::{collections::HashMap, ffi::OsStr, path::Path, sync::Arc};

use anyhow::{anyhow, bail, Result};
use assembler::KclvmLibAssembler;
use kclvm_ast::{
    ast::{Module, Program},
    MAIN_PKG,
};
use kclvm_config::cache::KCL_CACHE_PATH_ENV_VAR;
use kclvm_parser::{load_program, KCLModuleCache, ParseSessionRef};
use kclvm_query::apply_overrides;
use kclvm_sema::resolver::{
    resolve_program, resolve_program_with_opts, scope::ProgramScope, Options,
};
use kclvm_utils::fslock::open_lock_file;
use linker::Command;
pub use runner::{Artifact, ExecProgramArgs, ExecProgramResult, MapErrorResult};
use runner::{FastRunner, RunnerOptions};
#[cfg(feature = "llvm")]
use runner::{LibRunner, ProgramRunner};
use tempfile::tempdir;

pub mod assembler;
pub mod linker;
pub mod runner;

#[cfg(test)]
pub mod tests;

pub const KCL_FAST_EVAL_ENV_VAR: &str = "KCL_FAST_EVAL";

/// After the kcl program passed through kclvm-parser in the compiler frontend,
/// KCL needs to resolve ast, generate corresponding LLVM IR, dynamic link library or
/// executable file for kcl program in the compiler backend.
///
/// Method “execute” is the entry point for the compiler backend.
///
/// It returns the KCL program executing result as Result<a_json_string, an_err_string>,
/// and mainly takes "program" (ast.Program returned by kclvm-parser) as input.
///
/// "args" is the items selected by the user in the KCL CLI.
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
/// At last, KclLibRunner will be constructed and call method "run" to execute the kcl program.
///
/// **Note that it is not thread safe.**
///
/// # Examples
///
/// ```
/// use kclvm_runner::{exec_program, ExecProgramArgs};
/// use kclvm_parser::ParseSession;
/// use std::sync::Arc;
///
/// // Create sessions
/// let sess = Arc::new(ParseSession::default());
/// // Get default args
/// let mut args = ExecProgramArgs::default();
/// args.k_filename_list = vec!["./src/test_datas/init_check_order_0/main.k".to_string()];
///
/// // Resolve ast, generate libs, link libs and execute.
/// // Result is the kcl in json format.
/// let result = exec_program(sess, &args).unwrap();
/// ```
pub fn exec_program(sess: ParseSessionRef, args: &ExecProgramArgs) -> Result<ExecProgramResult> {
    // parse args from json string
    let opts = args.get_load_program_options();
    let kcl_paths_str = args
        .k_filename_list
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>();
    let module_cache = KCLModuleCache::default();
    let mut program = load_program(
        sess.clone(),
        kcl_paths_str.as_slice(),
        Some(opts),
        Some(module_cache),
    )?
    .program;
    apply_overrides(
        &mut program,
        &args.overrides,
        &[],
        args.print_override_ast || args.debug > 0,
    )?;
    execute(sess, program, args)
}

/// Execute the KCL artifact with args.
pub fn exec_artifact<P: AsRef<OsStr>>(
    path: P,
    args: &ExecProgramArgs,
) -> Result<ExecProgramResult> {
    #[cfg(feature = "llvm")]
    {
        Artifact::from_path(path)?.run(args)
    }
    #[cfg(not(feature = "llvm"))]
    {
        let _ = path;
        let _ = args;
        Err(anyhow::anyhow!("error: llvm feature is not enabled. Note: Set KCL_FAST_EVAL=1 or rebuild the crate with the llvm feature."))
    }
}

/// After the kcl program passed through kclvm-parser in the compiler frontend,
/// KCL needs to resolve ast, generate corresponding LLVM IR, dynamic link library or
/// executable file for kcl program in the compiler backend.
///
/// Method “execute” is the entry point for the compiler backend.
///
/// It returns the KCL program executing result as Result<a_json_string, an_err_string>,
/// and mainly takes "program" (ast.Program returned by kclvm-parser) as input.
///
/// "args" is the items selected by the user in the KCL CLI.
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
/// At last, KclLibRunner will be constructed and call method "run" to execute the kcl program.
///
/// **Note that it is not thread safe.**
///
/// # Examples
///
/// ```
/// use kclvm_runner::{execute, runner::ExecProgramArgs};
/// use kclvm_parser::{load_program, ParseSession};
/// use kclvm_ast::ast::Program;
/// use std::sync::Arc;
///
/// // Create sessions
/// let sess = Arc::new(ParseSession::default());
/// // Get default args
/// let args = ExecProgramArgs::default();
/// let opts = args.get_load_program_options();
///
/// // Parse kcl file
/// let kcl_path = "./src/test_datas/init_check_order_0/main.k";
/// let prog = load_program(sess.clone(), &[kcl_path], Some(opts), None).unwrap().program;
///     
/// // Resolve ast, generate libs, link libs and execute.
/// // Result is the kcl in json format.
/// let result = execute(sess, prog, &args).unwrap();
/// ```
pub fn execute(
    sess: ParseSessionRef,
    mut program: Program,
    args: &ExecProgramArgs,
) -> Result<ExecProgramResult> {
    // If the user only wants to compile the kcl program, the following code will only resolve ast.
    if args.compile_only {
        let mut resolve_opts = Options::default();
        resolve_opts.merge_program = false;
        // Resolve ast
        let scope = resolve_program_with_opts(&mut program, resolve_opts, None);
        emit_compile_diag_to_string(sess, &scope, args.compile_only)?;
        return Ok(ExecProgramResult::default());
    }
    // Resolve ast
    let scope = resolve_program(&mut program);
    // Emit parse and resolve errors if exists.
    emit_compile_diag_to_string(sess, &scope, false)?;
    Ok(
        // Use the fast evaluator to run the kcl program.
        if args.fast_eval || std::env::var(KCL_FAST_EVAL_ENV_VAR).is_ok() {
            FastRunner::new(Some(RunnerOptions {
                plugin_agent_ptr: args.plugin_agent,
            }))
            .run(&program, args)?
        } else {
            // Compile the kcl program to native lib and run it.
            #[cfg(feature = "llvm")]
            {
                // Create a temp entry file and the temp dir will be delete automatically
                let temp_dir = tempdir()?;
                let temp_dir_path = temp_dir.path().to_str().ok_or(anyhow!(
                    "Internal error: {}: No such file or directory",
                    temp_dir.path().display()
                ))?;
                let temp_entry_file = temp_file(temp_dir_path)?;

                // Generate libs
                let lib_paths = assembler::KclvmAssembler::new(
                    program,
                    scope,
                    temp_entry_file.clone(),
                    KclvmLibAssembler::LLVM,
                    args.get_package_maps_from_external_pkg(),
                )
                .gen_libs(args)?;

                // Link libs into one library
                let lib_suffix = Command::get_lib_suffix();
                let temp_out_lib_file = format!("{}{}", temp_entry_file, lib_suffix);
                let lib_path = linker::KclvmLinker::link_all_libs(lib_paths, temp_out_lib_file)?;

                // Run the library
                let runner = LibRunner::new(Some(RunnerOptions {
                    plugin_agent_ptr: args.plugin_agent,
                }));
                let result = runner.run(&lib_path, args)?;

                remove_file(&lib_path)?;
                clean_tmp_files(&temp_entry_file, &lib_suffix)?;
                result
            }
            // If we don't enable llvm feature, the default running path is through the evaluator.
            #[cfg(not(feature = "llvm"))]
            {
                FastRunner::new(Some(RunnerOptions {
                    plugin_agent_ptr: args.plugin_agent,
                }))
                .run(&program, args)?
            }
        },
    )
}

/// `execute_module` can directly execute the ast `Module`.
/// `execute_module` constructs `Program` with default pkg name `MAIN_PKG`,
/// and calls method `execute` with default `plugin_agent` and `ExecProgramArgs`.
/// For more information, see doc above method `execute`.
///
/// **Note that it is not thread safe.**
pub fn execute_module(m: Module) -> Result<ExecProgramResult> {
    let mut pkgs = HashMap::new();
    pkgs.insert(MAIN_PKG.to_string(), vec![Arc::new(m)]);

    let prog = Program {
        root: MAIN_PKG.to_string(),
        pkgs,
    };

    execute(
        ParseSessionRef::default(),
        prog,
        &ExecProgramArgs::default(),
    )
}

/// Build a KCL program and generate a library artifact.
pub fn build_program<P: AsRef<Path>>(
    sess: ParseSessionRef,
    args: &ExecProgramArgs,
    output: Option<P>,
) -> Result<Artifact> {
    // Parse program.
    let opts = args.get_load_program_options();
    let kcl_paths_str = args
        .k_filename_list
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<&str>>();
    let mut program =
        load_program(sess.clone(), kcl_paths_str.as_slice(), Some(opts), None)?.program;
    // Resolve program.
    let scope = resolve_program(&mut program);
    // Emit parse and resolve errors if exists.
    emit_compile_diag_to_string(sess, &scope, false)?;
    // When set the common package cache path, lock the package to prevent the
    // data competition during compilation of different modules.
    if let Ok(cache_path) = std::env::var(KCL_CACHE_PATH_ENV_VAR) {
        build_with_lock(args, program, scope, &cache_path, output)
    } else {
        let temp_dir = std::env::temp_dir();
        build_with_lock(args, program, scope, &temp_dir.to_string_lossy(), output)
    }
}

fn build_with_lock<P: AsRef<Path>>(
    args: &ExecProgramArgs,
    program: Program,
    scope: ProgramScope,
    cache_path: &str,
    output: Option<P>,
) -> Result<Artifact> {
    let lock_file = Path::new(&cache_path)
        .join(format!("pkg.lock"))
        .display()
        .to_string();
    let mut lock_file = open_lock_file(&lock_file)?;
    lock_file.lock()?;
    let artifact = build(args, program, scope, output);
    lock_file.unlock()?;
    artifact
}

fn build<P: AsRef<Path>>(
    args: &ExecProgramArgs,
    program: Program,
    scope: ProgramScope,
    output: Option<P>,
) -> Result<Artifact> {
    // Create a temp entry file and the temp dir will be delete automatically.
    let temp_dir = tempdir()?;
    let temp_dir_path = temp_dir.path().to_str().ok_or(anyhow!(
        "Internal error: {}: No such file or directory",
        temp_dir.path().display()
    ))?;
    let temp_entry_file = temp_file(temp_dir_path)?;

    // Link libs into one library.
    let lib_suffix = Command::get_lib_suffix();
    // Temporary output of linker
    let temp_out_lib_file = if let Some(output) = output {
        output
            .as_ref()
            .to_str()
            .ok_or(anyhow!("build output path is not found"))?
            .to_string()
    } else {
        format!("{}{}", temp_entry_file, lib_suffix)
    };
    // Generate native libs.
    let lib_paths = assembler::KclvmAssembler::new(
        program,
        scope,
        temp_entry_file.clone(),
        KclvmLibAssembler::LLVM,
        args.get_package_maps_from_external_pkg(),
    )
    .gen_libs(args)?;
    let lib_path = linker::KclvmLinker::link_all_libs(lib_paths, temp_out_lib_file)?;

    // Return the library artifact.
    Artifact::from_path(lib_path)
}

/// Clean all the tmp files generated during lib generating and linking.
#[inline]
#[cfg(feature = "llvm")]
fn clean_tmp_files(temp_entry_file: &String, lib_suffix: &String) -> Result<()> {
    let temp_entry_lib_file = format!("{}{}", temp_entry_file, lib_suffix);
    remove_file(&temp_entry_lib_file)
}

#[inline]
#[cfg(feature = "llvm")]
fn remove_file(file: &str) -> Result<()> {
    if Path::new(&file).exists() {
        std::fs::remove_file(file)?;
    }
    Ok(())
}

/// Returns a temporary file name consisting of timestamp and process id.
fn temp_file(dir: &str) -> Result<String> {
    let timestamp = chrono::Local::now()
        .timestamp_nanos_opt()
        .unwrap_or_default();
    let id = std::process::id();
    let file = format!("{}_{}", id, timestamp);
    std::fs::create_dir_all(dir)?;
    Ok(Path::new(dir)
        .join(file)
        .to_str()
        .ok_or(anyhow::anyhow!("{dir} not found"))?
        .to_string())
}

// [`emit_compile_diag_to_string`] will emit compile diagnostics to string, including parsing and resolving diagnostics.
fn emit_compile_diag_to_string(
    sess: ParseSessionRef,
    scope: &ProgramScope,
    include_warnings: bool,
) -> Result<()> {
    let mut res_str = sess.1.write().emit_to_string()?;
    let sema_err = scope.emit_diagnostics_to_string(sess.0.clone(), include_warnings);
    if let Err(err) = &sema_err {
        #[cfg(not(target_os = "windows"))]
        res_str.push('\n');
        #[cfg(target_os = "windows")]
        res_str.push_str("\r\n");
        res_str.push_str(err);
    }

    res_str
        .is_empty()
        .then(|| Ok(()))
        .unwrap_or_else(|| bail!(res_str))
}
