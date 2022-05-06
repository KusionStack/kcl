// Copyright 2021 The KCL Authors. All rights reserved.

use indexmap::IndexMap;
use inkwell::module::Module;
use inkwell::{context::Context, memory_buffer::MemoryBuffer};
use kclvm_ast::ast;
use std::error;

use crate::codegen::{EmitOptions, MODULE_NAME};

use super::context::LLVMCodeGenContext;

static RUNTIME_LLVM_BC: &[u8] = include_bytes!("../../../../runtime/src/_kclvm.bc");

/// Load runtime libraries and parse it to a module.
fn load_runtime(context: &'_ Context) -> Module<'_> {
    let memory = MemoryBuffer::create_from_memory_range(RUNTIME_LLVM_BC, MODULE_NAME);
    Module::parse_bitcode_from_buffer(&memory, context).unwrap()
}

/// Generate LLVM IR of KCL ast module.
pub fn emit_code(
    program: &ast::Program,
    import_names: IndexMap<String, IndexMap<String, String>>,
    opt: &EmitOptions,
) -> Result<(), Box<dyn error::Error>> {
    // Create a LLVM context
    let context = Context::create();
    // Create a LLVM module using an exist LLVM bitcode file
    let module = if let Some(path) = &opt.from_path {
        Module::parse_bitcode_from_path(std::path::Path::new(path), &context).unwrap()
    } else {
        load_runtime(&context)
    };
    // Create a KCL LLVM code generator using the KCL AST and the LLVM module
    let ctx = LLVMCodeGenContext::new(&context, module, program, import_names, opt.no_link);
    // Generate user KCL code LLVM IR
    crate::codegen::emit_code(ctx, opt)
}
