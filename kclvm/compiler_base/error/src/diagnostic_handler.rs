use crate::{
    diagnostic::diagnostic_message::TemplateLoader, Diagnostic, DiagnosticStyle, Emitter,
    MessageArgs, TerminalEmitter,
};
use anyhow::{Context, Result};
use compiler_base_span::fatal_error::FatalError;
use std::sync::Arc;

pub(crate) struct DiagnosticHandlerInner {
    emitter: Box<dyn Emitter<DiagnosticStyle>>,
    diagnostics: Vec<Diagnostic<DiagnosticStyle>>,
    err_count: usize,
    warn_count: usize,
    template_loader: Arc<TemplateLoader>,
}

impl DiagnosticHandlerInner {
    /// Load all (*.ftl) template files under directory `template_dir`.
    pub(crate) fn new_with_template_dir(template_dir: &str) -> Result<Self> {
        let template_loader = TemplateLoader::new_with_template_dir(template_dir)
            .with_context(|| format!("Failed to init `TemplateLoader` from '{}'", template_dir))?;

        Ok(Self {
            err_count: 0,
            warn_count: 0,
            emitter: Box::new(TerminalEmitter::default()),
            diagnostics: vec![],
            template_loader: Arc::new(template_loader),
        })
    }

    /// Add a diagnostic generated from error to `DiagnosticHandler`.
    /// `DiagnosticHandler` contains a set of `Diagnostic<DiagnosticStyle>`
    pub(crate) fn add_err_diagnostic(&mut self, diag: Diagnostic<DiagnosticStyle>) {
        self.diagnostics.push(diag);
        self.err_count += 1;
    }

    /// Add a diagnostic generated from warning to `DiagnosticHandler`.
    /// `DiagnosticHandler` contains a set of `Diagnostic<DiagnosticStyle>`
    pub(crate) fn add_warn_diagnostic(&mut self, diag: Diagnostic<DiagnosticStyle>) {
        self.diagnostics.push(diag);
        self.warn_count += 1;
    }

    /// Get count of diagnostics in `DiagnosticHandler`.
    /// `DiagnosticHandler` contains a set of `Diagnostic<DiagnosticStyle>`
    #[inline]
    pub(crate) fn diagnostics_count(&self) -> usize {
        self.diagnostics.len()
    }

    /// Emit the diagnostic messages generated from error to to terminal stderr.
    pub(crate) fn emit_error_diagnostic(&mut self, diag: Diagnostic<DiagnosticStyle>) {
        self.emitter.emit_diagnostic(&diag);
        self.err_count += 1;
    }

    /// Emit the diagnostic messages generated from warning to to terminal stderr.
    pub(crate) fn emit_warn_diagnostic(&mut self, diag: Diagnostic<DiagnosticStyle>) {
        self.emitter.emit_diagnostic(&diag);
        self.warn_count += 1;
    }

    /// Emit all the diagnostics messages to to terminal stderr.
    /// `DiagnosticHandler` contains a set of `Diagnostic<DiagnosticStyle>`
    pub(crate) fn emit_stashed_diagnostics(&mut self) {
        for diag in &self.diagnostics {
            self.emitter.emit_diagnostic(&diag)
        }
    }

    /// If some diagnotsics generated by errors, `has_errors` returns `True`.
    #[inline]
    pub(crate) fn has_errors(&self) -> bool {
        self.err_count > 0
    }

    /// If some diagnotsics generated by warnings, `has_errors` returns `True`.
    #[inline]
    pub(crate) fn has_warns(&self) -> bool {
        self.warn_count > 0
    }

    /// After emitting all the diagnostics, it will panic.
    pub(crate) fn abort_if_errors(&mut self) {
        self.emit_stashed_diagnostics();

        if self.has_errors() {
            FatalError.raise();
        }
    }

    /// Get the message string from "*.ftl" file by `index`, `sub_index` and `MessageArgs`.
    /// "*.ftl" file looks like, e.g. './src/diagnostic/locales/en-US/default.ftl' :
    pub(crate) fn get_diagnostic_msg(
        &self,
        index: &str,
        sub_index: Option<&str>,
        args: &MessageArgs,
    ) -> Result<String> {
        self.template_loader.get_msg_to_str(index, sub_index, &args)
    }
}
