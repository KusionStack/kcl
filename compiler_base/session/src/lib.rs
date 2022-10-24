use anyhow::{Context, Result};
use compiler_base_error::{diagnostic_handler::DiagnosticHandler, Diagnostic, DiagnosticStyle};
use compiler_base_span::{FilePathMapping, SourceMap};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(test)]
mod tests;

/// Represents the data associated with a compilation
/// session for a single crate.
///
/// Note: TODO(zongz): This is a WIP structure.
/// Currently only contains the part related to error diagnostic displaying.
pub struct Session {
    pub sm: Arc<SourceMap>,
    pub diag_handler: Arc<DiagnosticHandler>,
}

impl Session {
    /// Construct a `Session`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use compiler_base_session::Session;
    /// # use compiler_base_error::diagnostic_handler::DiagnosticHandler;
    /// # use std::path::PathBuf;
    /// # use compiler_base_span::FilePathMapping;
    /// # use compiler_base_span::SourceMap;
    /// # use std::sync::Arc;
    /// # use std::fs;
    ///
    /// // 1. You should create a new `SourceMap` wrapped with `Arc`.
    /// let filename = fs::canonicalize(&PathBuf::from("./src/test_datas/code_snippet")).unwrap().display().to_string();
    /// let src = std::fs::read_to_string(filename.clone()).unwrap();
    /// let sm = Arc::new(SourceMap::new(FilePathMapping::empty()));
    /// sm.new_source_file(PathBuf::from(filename.clone()).into(), src.to_string());
    ///
    /// // 2. You should create a new `DiagnosticHandler` wrapped with `Arc`.
    /// let diag_handler = Arc::new(DiagnosticHandler::new_with_template_dir("./src/test_datas/locales/en-US").unwrap());
    ///
    /// // 3. Create `Session`
    /// let sess = Session::new(sm, diag_handler);
    ///
    /// ```
    #[inline]
    pub fn new(sm: Arc<SourceMap>, diag_handler: Arc<DiagnosticHandler>) -> Self {
        Self { sm, diag_handler }
    }

    /// Construct a `Session` with file name and optional source code.
    ///
    /// In the method, a `SourceMap` with a `SourceFile` will be created from `filename` and the optional source code `code`.
    ///
    /// Note: `code` has higher priority than `filename`,
    /// If `code` is not None and the content in file `filename` is not the same as `code`,
    /// then the content in `code` will be used as the source code.
    ///
    /// If `code` is None, the session will use the content of file `filename` as source code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use compiler_base_session::Session;
    /// # use std::path::PathBuf;
    /// const CARGO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
    /// let mut cargo_file_path = PathBuf::from(CARGO_ROOT);
    /// cargo_file_path.push("src/test_datas/code_snippet");
    /// let abs_path = cargo_file_path.to_str().unwrap();
    ///
    /// let sess = Session::new_with_file_and_code(abs_path, None);
    /// ```
    /// The `sess` will take the content of file `abs_path` as source code.
    ///
    /// ```rust
    /// # use compiler_base_session::Session;
    /// # use std::path::PathBuf;
    /// const CARGO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
    /// let mut cargo_file_path = PathBuf::from(CARGO_ROOT);
    /// cargo_file_path.push("src/test_datas/code_snippet");
    /// let abs_path = cargo_file_path.to_str().unwrap();
    ///
    /// let sess = Session::new_with_file_and_code(abs_path, Some("This is tmp source code"));
    /// ```
    /// The `sess` will take "This is tmp source code" as source code.
    pub fn new_with_file_and_code(filename: &str, code: Option<&str>) -> Result<Self> {
        let sm = SourceMap::new(FilePathMapping::empty());
        match code {
            Some(c) => {
                sm.new_source_file(PathBuf::from(filename).into(), c.to_string());
            }
            None => {
                sm.load_file(&Path::new(&filename))
                    .with_context(|| "Failed to load source file")?;
            }
        }
        let diag = DiagnosticHandler::default()
            .with_context(|| "Internal bug: Failed to create session")?;
        Ok(Self {
            sm: Arc::new(sm),
            diag_handler: Arc::new(diag),
        })
    }

    /// Construct a `Session` with source code.
    ///
    /// In the method, a `SourceMap` with a `SourceFile` will be created from an empty path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use compiler_base_session::Session;
    /// let sess = Session::new_with_src_code("This is the source code");
    /// ```
    #[inline]
    pub fn new_with_src_code(code: &str) -> Result<Self> {
        let sm = SourceMap::new(FilePathMapping::empty());
        sm.new_source_file(PathBuf::from("").into(), code.to_string());
        let diag = DiagnosticHandler::default()
            .with_context(|| "Internal bug: Failed to create session")?;

        Ok(Self {
            sm: Arc::new(sm),
            diag_handler: Arc::new(diag),
        })
    }

    /// Emit error diagnostic to terminal.
    ///
    /// # Panics
    ///
    /// After emitting the error diagnositc, the program will panic.
    ///
    /// # Examples
    ///
    /// If you want to emit an error diagnostic.
    /// ```rust
    /// # use compiler_base_session::Session;
    /// # use compiler_base_error::components::Label;
    /// # use compiler_base_error::DiagnosticStyle;
    /// # use compiler_base_error::Diagnostic;
    /// # use compiler_base_session::SessionDiagnostic;
    /// # use anyhow::Result;
    ///
    /// // 1. Create your own error type.
    /// struct MyError;
    ///
    /// // 2. Implement trait `SessionDiagnostic` manually.
    /// impl SessionDiagnostic for MyError {
    ///     fn into_diagnostic(self, sess: &Session) -> Result<Diagnostic<DiagnosticStyle>> {
    ///         let mut diag = Diagnostic::<DiagnosticStyle>::new();
    ///         // 1. Label Component
    ///         let label_component = Box::new(Label::Error("error".to_string()));
    ///         diag.append_component(label_component);
    ///         Ok(diag)
    ///     }
    /// }
    ///
    /// let result = std::panic::catch_unwind(|| {
    ///    // 3. Create a Session.
    ///    let sess = Session::new_with_src_code("test code").unwrap();
    ///    // 4. Emit the error diagnostic.
    ///    sess.emit_err(MyError {}).unwrap();
    /// });
    /// assert!(result.is_err());
    ///
    /// ```
    pub fn emit_err(&self, err: impl SessionDiagnostic) -> Result<bool> {
        self.diag_handler
            .add_err_diagnostic(err.into_diagnostic(self)?)?
            .abort_if_errors()
            .with_context(|| "Internale Bug: Fail to display error diagnostic")?;
        Ok(true)
    }
}

/// Trait implemented by error types.
///
/// You can implement manually for error types as below.
///
/// # Example
///
/// ```rust
/// use anyhow::Result;
/// use compiler_base_error::components::Label;
/// use compiler_base_error::DiagnosticStyle;
/// use compiler_base_error::Diagnostic;
/// use compiler_base_session::Session;
/// use compiler_base_session::SessionDiagnostic;
///
/// // 1. Create your own error type.
/// struct MyError;
///
/// // 2. Implement trait `SessionDiagnostic` manually.
/// impl SessionDiagnostic for MyError {
///     fn into_diagnostic(self, sess: &Session) -> Result<Diagnostic<DiagnosticStyle>> {
///         let mut diag = Diagnostic::<DiagnosticStyle>::new();
///         // 1. Label Component
///         let label_component = Box::new(Label::Error("error".to_string()));
///         diag.append_component(label_component);
///         Ok(diag)
///     }
/// }
///
/// // 3. The diagnostic of MyError will display "error" on terminal.
/// // For more information about diagnositc displaying, see doc in `compiler_base_error`.
/// ```
///
/// Note:
/// TODO(zongz): `#[derive(SessionDiagnostic)]` is WIP, before that you need to manually implement this trait.
/// This should not be implemented manually. Instead, use `#[derive(SessionDiagnostic)]` in the future.
pub trait SessionDiagnostic {
    fn into_diagnostic(self, sess: &Session) -> Result<Diagnostic<DiagnosticStyle>>;
}
