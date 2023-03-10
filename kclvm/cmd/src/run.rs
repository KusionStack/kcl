use anyhow::Result;
use clap::ArgMatches;
use compiler_base_session::Session;
use kclvm_error::Diagnostic;
use kclvm_runner::exec_program;
use kclvm_runtime::PanicInfo;
use std::sync::Arc;

use crate::settings::must_build_settings;

/// Run the KCL main command.
pub fn run_command(matches: &ArgMatches) -> Result<()> {
    // Config settings building
    let settings = must_build_settings(matches);
    let output = settings.output();
    let sess = Arc::new(Session::default());
    match exec_program(sess.clone(), &settings.into(), 1) {
        Ok(result) => match output {
            Some(o) => {
                std::fs::write(o, result.yaml_result).unwrap();
            }
            None => println!("{}", result.yaml_result),
        },
        Err(msg) => {
            sess.add_err(<PanicInfo as Into<Diagnostic>>::into(PanicInfo::from(msg)))?;
            sess.emit_stashed_diagnostics_and_abort()?;
        }
    }
    Ok(())
}
