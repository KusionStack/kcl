//! KCL collection system module
//!
//! Copyright 2021 The KCL Authors. All rights reserved.

use crate::*;

#[allow(non_camel_case_types)]
type kclvm_value_ref_t = ValueRef;

#[no_mangle]
#[runtime_fn]
pub extern "C" fn kclvm_value_union_all(
    _ctx: *mut kclvm_context_t,
    args: *const kclvm_value_ref_t,
    _kwargs: *const kclvm_value_ref_t,
) -> *const kclvm_value_ref_t {
    let args = ptr_as_ref(args);
    if let Some(arg) = args.arg_0() {
        if !arg.is_truthy() || !arg.is_list() {
            return ValueRef::dict(None).into_raw();
        }
        let value = arg.as_list_ref();
        if value.values.is_empty() {
            return ValueRef::dict(None).into_raw();
        }
        let mut result = value.values[0].deep_copy();
        for (i, v) in value.values.iter().enumerate() {
            if i > 0 {
                result.union_with(v);
            }
        }
        return result.into_raw();
    }
    panic!("union_all() takes at least 1 argument (0 given)")
}
