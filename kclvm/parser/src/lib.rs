//! Copyright The KCL Authors. All rights reserved.

pub mod entry;
pub mod file_graph;
mod lexer;
mod parser;
mod session;

#[cfg(test)]
mod tests;

extern crate kclvm_error;

use crate::entry::get_compile_entries_from_paths;
pub use crate::session::{ParseSession, ParseSessionRef};
use compiler_base_macros::bug;
use compiler_base_session::Session;
use compiler_base_span::span::new_byte_pos;
use file_graph::{toposort, Pkg, PkgFile, PkgFileGraph, PkgMap};
use indexmap::IndexMap;
use kclvm_ast::ast::Module;
use kclvm_ast::{ast, MAIN_PKG};
use kclvm_config::modfile::{get_vendor_home, KCL_FILE_EXTENSION, KCL_FILE_SUFFIX, KCL_MOD_FILE};
use kclvm_error::diagnostic::{Errors, Range};
use kclvm_error::{ErrorKind, Message, Position, Style};
use kclvm_sema::plugin::PLUGIN_MODULE_PREFIX;
use kclvm_utils::path::PathPrefix;
use kclvm_utils::pkgpath::parse_external_pkg_name;
use kclvm_utils::pkgpath::rm_external_pkg_name;

use anyhow::Result;
use lexer::parse_token_streams;
use parser::Parser;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use kclvm_span::create_session_globals_then;

#[derive(Default, Debug)]
/// [`PkgInfo`] is some basic information about a kcl package.
pub(crate) struct PkgInfo {
    /// the name of the kcl package.
    pkg_name: String,
    /// path to save the package locally. e.g. /usr/xxx
    pkg_root: String,
    /// package path. e.g. konfig.base.xxx
    pkg_path: String,
    /// The kcl files that need to be compiled in this package.
    k_files: Vec<String>,
}

impl PkgInfo {
    /// New a [`PkgInfo`].
    pub(crate) fn new(
        pkg_name: String,
        pkg_root: String,
        pkg_path: String,
        k_files: Vec<String>,
    ) -> Self {
        PkgInfo {
            pkg_name,
            pkg_root,
            pkg_path,
            k_files,
        }
    }
}

/// parser mode
#[derive(Debug, Clone)]
pub enum ParseMode {
    Null,
    ParseComments,
}

/// LoadProgramResult denotes the result of the whole program and a topological
/// ordering of all known files,
#[derive(Debug, Clone)]
pub struct LoadProgramResult {
    /// Program AST
    pub program: ast::Program,
    /// Parse errors
    pub errors: Errors,
    /// The topological ordering of all known files.
    pub paths: Vec<PathBuf>,
}

/// ParseFileResult denotes the result of a single file including AST,
/// errors and import dependencies.
#[derive(Debug, Clone)]
pub struct ParseFileResult {
    /// Module AST
    pub module: ast::Module,
    /// Parse errors
    pub errors: Errors,
    /// Dependency paths.
    pub deps: Vec<PkgFile>,
}

/// Parse a KCL file to the AST module with parse errors.
pub fn parse_single_file(filename: &str, code: Option<String>) -> Result<ParseFileResult> {
    let filename = filename.adjust_canonicalization();
    let sess = Arc::new(ParseSession::default());
    let mut loader = Loader::new(
        sess,
        &[&filename],
        Some(LoadProgramOptions {
            load_packages: false,
            k_code_list: if let Some(code) = code {
                vec![code]
            } else {
                vec![]
            },
            ..Default::default()
        }),
        None,
    );
    let result = loader.load_main()?;
    let module = match result.program.get_main_package_first_module() {
        Some(module) => module.clone(),
        None => ast::Module::default(),
    };
    let file_graph = match loader.file_graph.read() {
        Ok(file_graph) => file_graph,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Failed to read KCL file graph. Because '{e}'"
            ))
        }
    };
    let file = PkgFile {
        path: PathBuf::from(filename),
        pkg_path: MAIN_PKG.to_string(),
    };
    let deps = if file_graph.contains_file(&file) {
        file_graph.dependencies_of(&file).into_iter().collect()
    } else {
        vec![]
    };
    Ok(ParseFileResult {
        module,
        errors: result.errors.clone(),
        deps,
    })
}

/// Parse a KCL file to the AST module and return errors when meets parse errors as result.
pub fn parse_file_force_errors(filename: &str, code: Option<String>) -> Result<ast::Module> {
    let sess = Arc::new(ParseSession::default());
    let result = parse_file_with_global_session(sess.clone(), filename, code);
    if sess.0.diag_handler.has_errors()? {
        let err = sess
            .0
            .emit_nth_diag_into_string(0)?
            .unwrap_or(Ok(ErrorKind::InvalidSyntax.name()))?;
        Err(anyhow::anyhow!(err))
    } else {
        result
    }
}

/// Parse a KCL file to the AST module with the parse session .
pub fn parse_file_with_session(
    sess: ParseSessionRef,
    filename: &str,
    code: Option<String>,
) -> Result<ast::Module> {
    // Code source.
    let src = if let Some(s) = code {
        s
    } else {
        match std::fs::read_to_string(filename) {
            Ok(src) => src,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to load KCL file '{filename}'. Because '{err}'"
                ));
            }
        }
    };

    // Build a source map to store file sources.
    let sf = sess
        .0
        .sm
        .new_source_file(PathBuf::from(filename).into(), src);

    let src_from_sf = match sf.src.as_ref() {
        Some(src) => src,
        None => {
            return Err(anyhow::anyhow!(
                "Internal Bug: Failed to load KCL file '{filename}'."
            ));
        }
    };

    // Lexer
    let stream = lexer::parse_token_streams(&sess, src_from_sf.as_str(), sf.start_pos);
    // Parser
    let mut p = parser::Parser::new(&sess, stream);
    let mut m = p.parse_module();
    m.filename = filename.to_string().adjust_canonicalization();

    Ok(m)
}

/// Parse a KCL file to the AST module with the parse session and the global session
#[inline]
pub fn parse_file_with_global_session(
    sess: ParseSessionRef,
    filename: &str,
    code: Option<String>,
) -> Result<ast::Module> {
    create_session_globals_then(move || parse_file_with_session(sess, filename, code))
}

/// Parse a source string to a expression. When input empty string, it will return [None].
///
/// # Examples
/// ```
/// use kclvm_ast::ast;
/// use kclvm_parser::parse_expr;
///
/// let expr = parse_expr("'alice'").unwrap();
/// assert!(matches!(expr.node, ast::Expr::StringLit(_)));
/// let expr = parse_expr("");
/// assert!(matches!(expr, None));
/// ```
pub fn parse_expr(src: &str) -> Option<ast::NodeRef<ast::Expr>> {
    if src.is_empty() {
        None
    } else {
        let sess = Arc::new(Session::default());
        let sf = sess
            .sm
            .new_source_file(PathBuf::from("").into(), src.to_string());
        let src_from_sf = match sf.src.as_ref() {
            Some(src) => src,
            None => {
                bug!("Internal Bug: Failed to load KCL file.");
            }
        };

        let sess = &&ParseSession::with_session(sess);

        let expr: Option<ast::NodeRef<ast::Expr>> = Some(create_session_globals_then(|| {
            let stream = parse_token_streams(sess, src_from_sf.as_str(), new_byte_pos(0));
            let mut parser = Parser::new(sess, stream);
            parser.parse_expr()
        }));
        expr
    }
}

#[derive(Debug, Clone)]
pub struct LoadProgramOptions {
    pub work_dir: String,
    pub k_code_list: Vec<String>,
    pub vendor_dirs: Vec<String>,
    pub package_maps: HashMap<String, String>,
    /// The parser mode.
    pub mode: ParseMode,
    /// Whether to load packages.
    pub load_packages: bool,
    /// Whether to load plugins
    pub load_plugins: bool,
}

impl Default for LoadProgramOptions {
    fn default() -> Self {
        Self {
            work_dir: Default::default(),
            k_code_list: Default::default(),
            vendor_dirs: vec![get_vendor_home()],
            package_maps: Default::default(),
            mode: ParseMode::ParseComments,
            load_packages: true,
            load_plugins: false,
        }
    }
}

/// Load the KCL program by paths and options,
/// "module_cache" is used to cache parsed asts to support incremental parse,
/// if it is None, module caching will be disabled
///
/// # Examples
///
/// ```
/// use kclvm_parser::{load_program, ParseSession};
/// use kclvm_parser::KCLModuleCache;
/// use kclvm_ast::ast::Program;
/// use std::sync::Arc;
///
/// // Create sessions
/// let sess = Arc::new(ParseSession::default());
/// // Create module cache
/// let module_cache = KCLModuleCache::default();
///
/// // Parse kcl file
/// let kcl_path = "./testdata/import-01.k";
/// let prog = load_program(sess.clone(), &[kcl_path], None, Some(module_cache.clone())).unwrap();
///     
/// ```
pub fn load_program(
    sess: ParseSessionRef,
    paths: &[&str],
    opts: Option<LoadProgramOptions>,
    module_cache: Option<KCLModuleCache>,
) -> Result<LoadProgramResult> {
    Loader::new(sess, paths, opts, module_cache).load_main()
}

pub type KCLModuleCache = Arc<RwLock<ModuleCache>>;

#[derive(Default, Debug)]
pub struct ModuleCache {
    pub ast_cache: IndexMap<PathBuf, Arc<ast::Module>>,
    pub dep_cache: IndexMap<PkgFile, (Vec<PkgFile>, PkgMap)>,
}
struct Loader {
    sess: ParseSessionRef,
    paths: Vec<String>,
    opts: LoadProgramOptions,
    module_cache: KCLModuleCache,
    file_graph: FileGraphCache,
}

impl Loader {
    fn new(
        sess: ParseSessionRef,
        paths: &[&str],
        opts: Option<LoadProgramOptions>,
        module_cache: Option<KCLModuleCache>,
    ) -> Self {
        Self {
            sess,
            paths: paths
                .iter()
                .map(|s| kclvm_utils::path::convert_windows_drive_letter(s))
                .collect(),
            opts: opts.unwrap_or_default(),
            module_cache: module_cache.unwrap_or_default(),
            file_graph: FileGraphCache::default(),
        }
    }

    #[inline]
    fn load_main(&mut self) -> Result<LoadProgramResult> {
        create_session_globals_then(move || self._load_main())
    }

    fn _load_main(&mut self) -> Result<LoadProgramResult> {
        parse_program(
            self.sess.clone(),
            self.paths.clone(),
            self.module_cache.clone(),
            self.file_graph.clone(),
            &self.opts,
        )
    }
}

fn fix_rel_import_path_with_file(
    pkgroot: &str,
    m: &mut ast::Module,
    file: &PkgFile,
    pkgmap: &PkgMap,
    opts: LoadProgramOptions,
    sess: ParseSessionRef,
) {
    for stmt in &mut m.body {
        let pos = stmt.pos().clone();
        if let ast::Stmt::Import(ref mut import_spec) = &mut stmt.node {
            let fix_path = kclvm_config::vfs::fix_import_path(
                pkgroot,
                &m.filename,
                import_spec.path.node.as_str(),
            );
            import_spec.path.node = fix_path.clone();

            let pkg = pkgmap.get(&file).expect("file not in pkgmap").clone();
            import_spec.pkg_name = pkg.pkg_name.clone();
            // Load the import package source code and compile.
            let pkg_info = find_packages(
                pos.into(),
                &pkg.pkg_name,
                &pkg.pkg_root,
                &fix_path,
                opts.clone(),
                sess.clone(),
            )
            .unwrap_or(None);
            if let Some(pkg_info) = &pkg_info {
                // Add the external package name as prefix of the [`kclvm_ast::ImportStmt`]'s member [`path`].
                import_spec.path.node = pkg_info.pkg_path.to_string();
                import_spec.pkg_name = pkg_info.pkg_name.clone();
            }
        }
    }
}

fn is_plugin_pkg(pkgpath: &str) -> bool {
    pkgpath.starts_with(PLUGIN_MODULE_PREFIX)
}

fn is_builtin_pkg(pkgpath: &str) -> bool {
    let system_modules = kclvm_sema::builtin::system_module::STANDARD_SYSTEM_MODULES;
    system_modules.contains(&pkgpath)
}

fn find_packages(
    pos: ast::Pos,
    pkg_name: &str,
    pkg_root: &str,
    pkg_path: &str,
    opts: LoadProgramOptions,
    sess: ParseSessionRef,
) -> Result<Option<PkgInfo>> {
    if pkg_path.is_empty() {
        return Ok(None);
    }

    // plugin pkgs
    if is_plugin_pkg(pkg_path) {
        if !opts.load_plugins {
            sess.1.write().add_error(
                ErrorKind::CannotFindModule,
                &[Message {
                    range: Into::<Range>::into(pos),
                    style: Style::Line,
                    message: format!("the plugin package `{}` is not found, please confirm if plugin mode is enabled", pkg_path),
                    note: None,
                    suggested_replacement: None,
                }],
            );
        }
        return Ok(None);
    }

    // builtin pkgs
    if is_builtin_pkg(pkg_path) {
        return Ok(None);
    }

    // 1. Look for in the current package's directory.
    let is_internal = is_internal_pkg(pkg_name, pkg_root, pkg_path)?;
    // 2. Look for in the vendor path.
    let is_external = is_external_pkg(pkg_path, opts)?;

    // 3. Internal and external packages cannot be duplicated
    if is_external.is_some() && is_internal.is_some() {
        sess.1.write().add_error(
            ErrorKind::CannotFindModule,
            &[Message {
                range: Into::<Range>::into(pos),
                style: Style::Line,
                message: format!(
                    "the `{}` is found multiple times in the current package and vendor package",
                    pkg_path
                ),
                note: None,
                suggested_replacement: None,
            }],
        );
        return Ok(None);
    }

    // 4. Get package information based on whether the package is internal or external.

    match is_internal.or(is_external) {
        Some(pkg_info) => Ok(Some(pkg_info)),
        None => {
            sess.1.write().add_error(
                ErrorKind::CannotFindModule,
                &[Message {
                    range: Into::<Range>::into(pos),
                    style: Style::Line,
                    message: format!("pkgpath {} not found in the program", pkg_path),
                    note: None,
                    suggested_replacement: None,
                }],
            );
            let mut suggestions = vec![format!("browse more packages at 'https://artifacthub.io'")];

            if let Ok(pkg_name) = parse_external_pkg_name(pkg_path) {
                suggestions.insert(
                    0,
                    format!(
                        "try 'kcl mod add {}' to download the missing package",
                        pkg_name
                    ),
                );
            }
            sess.1.write().add_suggestions(suggestions);
            Ok(None)
        }
    }
}

/// Search [`pkgpath`] among all the paths in [`pkgroots`].
///
/// # Notes
///
/// All paths in [`pkgpath`] must contain the kcl.mod file.
/// It returns the parent directory of kcl.mod if present, or none if not.
fn pkg_exists(pkgroots: &[String], pkgpath: &str) -> Option<String> {
    pkgroots
        .into_iter()
        .find(|root| pkg_exists_in_path(root, pkgpath))
        .cloned()
}

/// Search for [`pkgpath`] under [`path`].
/// It only returns [`true`] if [`path`]/[`pkgpath`] or [`path`]/[`pkgpath.k`] exists.
fn pkg_exists_in_path(path: &str, pkgpath: &str) -> bool {
    let mut pathbuf = PathBuf::from(path);
    pkgpath.split('.').for_each(|s| pathbuf.push(s));
    pathbuf.exists() || pathbuf.with_extension(KCL_FILE_EXTENSION).exists()
}

/// Look for [`pkgpath`] in the current package's [`pkgroot`].
/// If found, return to the [`PkgInfo`]， else return [`None`]
///
/// # Error
///
/// [`is_internal_pkg`] will return an error if the package's source files cannot be found.
fn is_internal_pkg(pkg_name: &str, pkg_root: &str, pkg_path: &str) -> Result<Option<PkgInfo>> {
    match pkg_exists(&[pkg_root.to_string()], pkg_path) {
        Some(internal_pkg_root) => {
            let fullpath = if pkg_name == kclvm_ast::MAIN_PKG {
                pkg_path.to_string()
            } else {
                format!("{}.{}", pkg_name, pkg_path)
            };
            let k_files = get_pkg_kfile_list(pkg_root, pkg_path)?;
            Ok(Some(PkgInfo::new(
                pkg_name.to_string(),
                internal_pkg_root,
                fullpath,
                k_files,
            )))
        }
        None => Ok(None),
    }
}

fn get_pkg_kfile_list(pkgroot: &str, pkgpath: &str) -> Result<Vec<String>> {
    // plugin pkgs
    if is_plugin_pkg(pkgpath) {
        return Ok(Vec::new());
    }

    // builtin pkgs
    if is_builtin_pkg(pkgpath) {
        return Ok(Vec::new());
    }

    if pkgroot.is_empty() {
        return Err(anyhow::anyhow!("pkgroot not found"));
    }

    let mut pathbuf = std::path::PathBuf::new();
    pathbuf.push(pkgroot);

    for s in pkgpath.split('.') {
        pathbuf.push(s);
    }

    let abspath = match pathbuf.canonicalize() {
        Ok(p) => p.to_str().unwrap().to_string(),
        Err(_) => pathbuf.as_path().to_str().unwrap().to_string(),
    };
    if std::path::Path::new(abspath.as_str()).exists() {
        return get_dir_files(abspath.as_str());
    }

    let as_k_path = abspath + KCL_FILE_SUFFIX;
    if std::path::Path::new((as_k_path).as_str()).exists() {
        return Ok(vec![as_k_path]);
    }

    Ok(Vec::new())
}

/// Get file list in the directory.
fn get_dir_files(dir: &str) -> Result<Vec<String>> {
    if !std::path::Path::new(dir).exists() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    for path in std::fs::read_dir(dir)? {
        let path = path?;
        if !path
            .file_name()
            .to_str()
            .unwrap()
            .ends_with(KCL_FILE_SUFFIX)
        {
            continue;
        }
        if path.file_name().to_str().unwrap().ends_with("_test.k") {
            continue;
        }
        if path.file_name().to_str().unwrap().starts_with('_') {
            continue;
        }

        let s = format!("{}", path.path().display());
        list.push(s);
    }

    list.sort();
    Ok(list)
}

/// Look for [`pkgpath`] in the external package's home.
/// If found, return to the [`PkgInfo`]， else return [`None`]
///
/// # Error
///
/// - [`is_external_pkg`] will return an error if the package's source files cannot be found.
/// - The name of the external package could not be resolved from [`pkg_path`].
fn is_external_pkg(pkg_path: &str, opts: LoadProgramOptions) -> Result<Option<PkgInfo>> {
    let pkg_name = parse_external_pkg_name(pkg_path)?;
    let external_pkg_root = if let Some(root) = opts.package_maps.get(&pkg_name) {
        PathBuf::from(root).join(KCL_MOD_FILE)
    } else {
        match pkg_exists(&opts.vendor_dirs, pkg_path) {
            Some(path) => PathBuf::from(path).join(&pkg_name).join(KCL_MOD_FILE),
            None => return Ok(None),
        }
    };

    if external_pkg_root.exists() {
        return Ok(Some(match external_pkg_root.parent() {
            Some(root) => {
                let abs_root: String = match root.canonicalize() {
                    Ok(p) => p.to_str().unwrap().to_string(),
                    Err(_) => root.display().to_string(),
                };
                let k_files = get_pkg_kfile_list(&abs_root, &rm_external_pkg_name(pkg_path)?)?;
                PkgInfo::new(
                    pkg_name.to_string(),
                    abs_root,
                    pkg_path.to_string(),
                    k_files,
                )
            }
            None => return Ok(None),
        }));
    } else {
        Ok(None)
    }
}

pub type ASTCache = Arc<RwLock<IndexMap<PathBuf, Arc<ast::Module>>>>;
pub type FileGraphCache = Arc<RwLock<PkgFileGraph>>;

pub fn parse_file(
    sess: ParseSessionRef,
    file: PkgFile,
    src: Option<String>,
    module_cache: KCLModuleCache,
    pkgs: &mut HashMap<String, Vec<Module>>,
    pkgmap: &mut PkgMap,
    file_graph: FileGraphCache,
    opts: &LoadProgramOptions,
) -> Result<Vec<PkgFile>> {
    let m = Arc::new(parse_file_with_session(
        sess.clone(),
        file.path.to_str().unwrap(),
        src,
    )?);

    let (deps, new_pkgmap) = get_deps(&file, m.as_ref(), pkgs, pkgmap, opts, sess)?;
    pkgmap.extend(new_pkgmap.clone());
    match &mut module_cache.write() {
        Ok(module_cache) => {
            module_cache
                .ast_cache
                .insert(file.canonicalize(), m.clone());
            module_cache
                .dep_cache
                .insert(file.clone(), (deps.clone(), new_pkgmap));
        }
        Err(e) => return Err(anyhow::anyhow!("Parse file failed: {e}")),
    }

    match &mut file_graph.write() {
        Ok(file_graph) => {
            file_graph.update_file(&file, &deps);
        }
        Err(e) => return Err(anyhow::anyhow!("Parse file failed: {e}")),
    }
    Ok(deps)
}

pub fn get_deps(
    file: &PkgFile,
    m: &Module,
    modules: &mut HashMap<String, Vec<Module>>,
    pkgmap: &mut PkgMap,
    opts: &LoadProgramOptions,
    sess: ParseSessionRef,
) -> Result<(Vec<PkgFile>, PkgMap)> {
    let mut deps: Vec<PkgFile> = vec![];
    let mut new_pkgmap = PkgMap::default();
    for stmt in &m.body {
        let pos = stmt.pos().clone();
        let pkg = pkgmap.get(file).expect("file not in pkgmap").clone();
        if let ast::Stmt::Import(import_spec) = &stmt.node {
            let fix_path = kclvm_config::vfs::fix_import_path(
                &pkg.pkg_root,
                &m.filename,
                import_spec.path.node.as_str(),
            );
            let pkg_info = find_packages(
                pos.into(),
                &pkg.pkg_name,
                &pkg.pkg_root,
                &fix_path,
                opts.clone(),
                sess.clone(),
            )?;
            if let Some(pkg_info) = &pkg_info {
                // If k_files is empty, the pkg information will not be found in the file graph.
                // Record the empty pkg to prevent loss. After the parse file is completed, fill in the modules
                if pkg_info.k_files.is_empty() {
                    modules.insert(pkg_info.pkg_path.clone(), vec![]);
                }
                // Add file dependencies.
                let mut paths: Vec<PkgFile> = pkg_info
                    .k_files
                    .iter()
                    .map(|p| {
                        let file = PkgFile {
                            path: p.into(),
                            pkg_path: pkg_info.pkg_path.clone(),
                        };
                        new_pkgmap.insert(
                            file.clone(),
                            file_graph::Pkg {
                                pkg_name: pkg_info.pkg_name.clone(),
                                pkg_root: pkg_info.pkg_root.clone().into(),
                            },
                        );
                        file
                    })
                    .collect();
                deps.append(&mut paths);
            }
        }
    }
    Ok((deps, new_pkgmap))
}

pub fn parse_pkg(
    sess: ParseSessionRef,
    files: Vec<(PkgFile, Option<String>)>,
    module_cache: KCLModuleCache,
    pkgs: &mut HashMap<String, Vec<Module>>,
    pkgmap: &mut PkgMap,
    file_graph: FileGraphCache,
    opts: &LoadProgramOptions,
) -> Result<Vec<PkgFile>> {
    let mut dependent = vec![];
    for (file, src) in files {
        let deps = parse_file(
            sess.clone(),
            file.clone(),
            src,
            module_cache.clone(),
            pkgs,
            pkgmap,
            file_graph.clone(),
            opts,
        )?;
        dependent.extend(deps);
    }
    Ok(dependent)
}

pub fn parse_entry(
    sess: ParseSessionRef,
    entry: &entry::Entry,
    module_cache: KCLModuleCache,
    pkgs: &mut HashMap<String, Vec<Module>>,
    pkgmap: &mut PkgMap,
    file_graph: FileGraphCache,
    opts: &LoadProgramOptions,
) -> Result<()> {
    let k_files = entry.get_k_files();
    let maybe_k_codes = entry.get_k_codes();
    let mut files = vec![];
    for (i, f) in k_files.iter().enumerate() {
        let file = PkgFile {
            path: f.adjust_canonicalization().into(),
            pkg_path: MAIN_PKG.to_string(),
        };
        files.push((file.clone(), maybe_k_codes.get(i).unwrap_or(&None).clone()));
        pkgmap.insert(
            file,
            Pkg {
                pkg_name: entry.name().clone(),
                pkg_root: entry.path().into(),
            },
        );
    }
    let dependent_paths = parse_pkg(
        sess.clone(),
        files,
        module_cache.clone(),
        pkgs,
        pkgmap,
        file_graph.clone(),
        opts,
    )?;
    let mut unparsed_file: VecDeque<PkgFile> = dependent_paths.into();
    let mut parsed_file: HashSet<PkgFile> = HashSet::new();
    while let Some(file) = unparsed_file.pop_front() {
        if parsed_file.insert(file.clone()) {
            let module_cache_read = module_cache.read();
            match &module_cache_read {
                Ok(m_cache) => match m_cache.ast_cache.get(&file.canonicalize()) {
                    Some(m) => {
                        let (deps, new_pkgmap) =
                            m_cache.dep_cache.get(&file).cloned().unwrap_or_else(|| {
                                get_deps(&file, m.as_ref(), pkgs, pkgmap, opts, sess.clone())
                                    .unwrap()
                            });
                        pkgmap.extend(new_pkgmap.clone());

                        match &mut file_graph.write() {
                            Ok(file_graph) => {
                                file_graph.update_file(&file, &deps);

                                for dep in deps {
                                    if !parsed_file.contains(&dep) {
                                        unparsed_file.push_back(dep.clone());
                                    }
                                }

                                continue;
                            }
                            Err(e) => return Err(anyhow::anyhow!("Parse entry failed: {e}")),
                        }
                    }
                    None => {
                        drop(module_cache_read);
                        let deps = parse_file(
                            sess.clone(),
                            file,
                            None,
                            module_cache.clone(),
                            pkgs,
                            pkgmap,
                            file_graph.clone(),
                            &opts,
                        )?;
                        for dep in deps {
                            if !parsed_file.contains(&dep) {
                                unparsed_file.push_back(dep.clone());
                            }
                        }
                    }
                },
                Err(e) => return Err(anyhow::anyhow!("Parse entry failed: {e}")),
            };
        }
    }
    Ok(())
}

pub fn parse_program(
    sess: ParseSessionRef,
    paths: Vec<String>,
    module_cache: KCLModuleCache,
    file_graph: FileGraphCache,
    opts: &LoadProgramOptions,
) -> Result<LoadProgramResult> {
    let compile_entries = get_compile_entries_from_paths(&paths, &opts)?;
    let workdir = compile_entries.get_root_path().to_string();
    let mut pkgs: HashMap<String, Vec<Module>> = HashMap::new();
    let mut pkgmap = PkgMap::new();
    for entry in compile_entries.iter() {
        parse_entry(
            sess.clone(),
            entry,
            module_cache.clone(),
            &mut pkgs,
            &mut pkgmap,
            file_graph.clone(),
            &opts,
        )?;
    }

    let files = match file_graph.read() {
        Ok(file_graph) => {
            let files = match file_graph.toposort() {
                Ok(files) => files,
                Err(_) => file_graph.paths(),
            };

            let file_path_graph = file_graph.file_path_graph().0;
            if let Err(cycle) = toposort(&file_path_graph) {
                let formatted_cycle = cycle
                    .iter()
                    .map(|file| format!("- {}\n", file.to_string_lossy()))
                    .collect::<String>();

                sess.1.write().add_error(
                    ErrorKind::RecursiveLoad,
                    &[Message {
                        range: (Position::dummy_pos(), Position::dummy_pos()),
                        style: Style::Line,
                        message: format!(
                            "Could not compiles due to cyclic import statements\n{}",
                            formatted_cycle.trim_end()
                        ),
                        note: None,
                        suggested_replacement: None,
                    }],
                );
            }
            files
        }
        Err(e) => return Err(anyhow::anyhow!("Parse program failed: {e}")),
    };

    for file in files.iter() {
        let mut m = match module_cache.read() {
            Ok(module_cache) => module_cache
                .ast_cache
                .get(&file.canonicalize())
                .expect(&format!(
                    "Module not found in module: {:?}",
                    file.canonicalize()
                ))
                .as_ref()
                .clone(),
            Err(e) => return Err(anyhow::anyhow!("Parse program failed: {e}")),
        };
        let pkg = pkgmap.get(file).expect("file not in pkgmap");
        fix_rel_import_path_with_file(
            &pkg.pkg_root,
            &mut m,
            file,
            &pkgmap,
            opts.clone(),
            sess.clone(),
        );

        match pkgs.get_mut(&file.pkg_path) {
            Some(modules) => {
                modules.push(m);
            }
            None => {
                pkgs.insert(file.pkg_path.clone(), vec![m]);
            }
        }
    }
    let program = ast::Program {
        root: workdir,
        pkgs,
    };
    Ok(LoadProgramResult {
        program,
        errors: sess.1.read().diagnostics.clone(),
        paths: files.iter().map(|file| file.path.clone()).collect(),
    })
}
