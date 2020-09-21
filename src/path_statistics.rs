use colored::*;
use crate::ConstantTimeResultForPath;
use haybale::Error;
use std::fmt;

/// Some statistics which can be computed from a
/// [`ConstantTimeResultForFunction`](struct.ConstantTimeResultForFunction.html).
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PathStatistics {
    /// How many paths "passed", that is, had no error or constant-time violation
    pub num_ct_paths: usize,
    /// How many constant-time violations did we find
    pub num_ct_violations: usize,
    /// How many Unsat errors did we find
    pub num_unsats: usize,
    /// How many LoopBoundExceeded errors did we find
    pub num_loop_bound_exceeded: usize,
    /// How many NullPointerDereference errors did we find
    pub num_null_ptr_deref: usize,
    /// How many FunctionNotFound errors did we find
    pub num_function_not_found: usize,
    /// How many solver errors (including timeouts) did we find
    pub num_solver_errors: usize,
    /// How many UnsupportedInstruction errors did we find
    pub num_unsupported_instruction: usize,
    /// How many MalformedInstruction errors did we find
    pub num_malformed_instruction: usize,
    /// How many UnreachableInstruction errors did we find
    pub num_unreachable_instruction: usize,
    /// How many FailedToResolveFunctionPointer errors did we find
    pub num_failed_resolve_fptr: usize,
    /// How many HookReturnValueMismatch errors did we find
    pub num_hook_retval_mismatch: usize,
    /// How many other errors did we find
    pub num_other_errors: usize,
}

impl PathStatistics {
    /// A fresh `PathStatistics` with all zeroes
    pub(crate) fn new() -> Self {
        Self {
            num_ct_paths: 0,
            num_ct_violations: 0,
            num_unsats: 0,
            num_loop_bound_exceeded: 0,
            num_null_ptr_deref: 0,
            num_function_not_found: 0,
            num_solver_errors: 0,
            num_unsupported_instruction: 0,
            num_malformed_instruction: 0,
            num_unreachable_instruction: 0,
            num_failed_resolve_fptr: 0,
            num_hook_retval_mismatch: 0,
            num_other_errors: 0,
        }
    }

    pub(crate) fn add_path_result(&mut self, path_result: &ConstantTimeResultForPath) {
        match path_result {
            ConstantTimeResultForPath::IsConstantTime => self.num_ct_paths += 1,
            ConstantTimeResultForPath::NotConstantTime { .. } => self.num_ct_violations += 1,
            ConstantTimeResultForPath::OtherError { error: Error::Unsat, .. } => self.num_unsats += 1,
            ConstantTimeResultForPath::OtherError { error: Error::LoopBoundExceeded(_), .. } => self.num_loop_bound_exceeded += 1,
            ConstantTimeResultForPath::OtherError { error: Error::NullPointerDereference, .. } => self.num_null_ptr_deref += 1,
            ConstantTimeResultForPath::OtherError { error: Error::FunctionNotFound(_), .. } => self.num_function_not_found += 1,
            ConstantTimeResultForPath::OtherError { error: Error::SolverError(_), .. } => self.num_solver_errors += 1,
            ConstantTimeResultForPath::OtherError { error: Error::UnsupportedInstruction(_), .. } => self.num_unsupported_instruction += 1,
            ConstantTimeResultForPath::OtherError { error: Error::MalformedInstruction(_), .. } => self.num_malformed_instruction += 1,
            ConstantTimeResultForPath::OtherError { error: Error::UnreachableInstruction, .. } => self.num_unreachable_instruction += 1,
            ConstantTimeResultForPath::OtherError { error: Error::FailedToResolveFunctionPointer(_), .. } => self.num_failed_resolve_fptr += 1,
            ConstantTimeResultForPath::OtherError { error: Error::HookReturnValueMismatch(_), .. } => self.num_hook_retval_mismatch += 1,
            ConstantTimeResultForPath::OtherError { error: Error::OtherError(_), .. } => self.num_other_errors += 1,
        }
    }
}

impl fmt::Display for PathStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // We always show "verified paths" and "constant-time violations found"
        writeln!(f, "verified paths: {}",
            if self.num_ct_paths > 0 {
                self.num_ct_paths.to_string().green()
            } else {
                self.num_ct_paths.to_string().normal()
            }
        )?;
        writeln!(f, "constant-time violations found: {}",
            if self.num_ct_violations > 0 {
                self.num_ct_violations.to_string().red()
            } else {
                self.num_ct_violations.to_string().normal()
            }
        )?;

        // For the other error types, we only show the entry if it's > 0
        if self.num_null_ptr_deref > 0 {
            writeln!(f, "null-pointer dereferences found: {}",
                self.num_null_ptr_deref.to_string().red()
            )?;
        }
        if self.num_function_not_found > 0 {
            writeln!(f, "function-not-found errors: {}",
                self.num_function_not_found.to_string().red()
            )?;
        }
        if self.num_unsupported_instruction > 0 {
            writeln!(f, "unsupported-instruction errors: {}",
                self.num_unsupported_instruction.to_string().red()
            )?;
        }
        if self.num_malformed_instruction > 0 {
            writeln!(f, "malformed-instruction errors: {}",
                self.num_malformed_instruction.to_string().red()
            )?;
        }
        if self.num_unsats > 0 {
            writeln!(f, "unsat errors: {}",
                self.num_unsats.to_string().red()
            )?;
        }
        if self.num_loop_bound_exceeded > 0 {
            writeln!(f, "paths exceeding the loop bound: {}",
                self.num_loop_bound_exceeded.to_string().red()
            )?;
        }
        if self.num_unreachable_instruction > 0 {
            writeln!(f, "unreachable-instruction errors: {}",
                self.num_unreachable_instruction.to_string().red()
            )?;
        }
        if self.num_failed_resolve_fptr > 0 {
            writeln!(f, "failed-function-pointer-resolution errors: {}",
                self.num_failed_resolve_fptr.to_string().red()
            )?;
        }
        if self.num_hook_retval_mismatch > 0 {
            writeln!(f, "hook-retval-mismatch errors: {}",
                self.num_hook_retval_mismatch.to_string().red()
            )?;
        }
        if self.num_solver_errors > 0 {
            writeln!(f, "solver errors, including timeouts: {}",
                self.num_solver_errors.to_string().red()
            )?;
        }
        if self.num_other_errors > 0 {
            writeln!(f, "other errors: {}",
                self.num_other_errors.to_string().red()
            )?;
        }
        Ok(())
    }
}
