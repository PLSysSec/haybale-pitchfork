//! This module contains helper functions that may be useful in writing function hooks.

use crate::secret;
use haybale::{Error, Result, State};
use haybale::backend::*;
use llvm_ir::*;

/// This utility function fills a buffer with unconstrained data, and also outputs the number of bytes written.
///
/// The entire `max_buffer_len_bytes`-byte buffer will be written, but the output number of bytes will be constrained to be any number between 0 and `max_buffer_len_bytes`.
pub fn fill_unconstrained_with_length<B: Backend>(
    state: &mut State<B>,
    out_buffer: &Operand,  // address of output buffer
    out_len_ptr: &Operand,  // address of a 64-bit integer which gets the number of bytes written
    max_buffer_len_bytes: u32,
    buffer_name: String,
) -> Result<()> {
    let out_len_bitwidth: u32 = 64;  // we assume that the out_len is a 64-bit integer

    // sanity-check some argument types
    match out_buffer.get_type() {
        Type::PointerType { .. } => {},
        ty => return Err(Error::OtherError(format!("fill_unconstrained_with_length: expected out_buffer to be some pointer type, got {:?}", ty))),
    };
    match out_len_ptr.get_type() {
        Type::PointerType { pointee_type, .. } => match *pointee_type {
            Type::IntegerType { bits } if bits == out_len_bitwidth => {},
            _ => return Err(Error::OtherError(format!("fill_unconstrained_with_length: expected out_len_ptr to be pointer-to-64-bit-integer type, got pointer to {:?}", pointee_type))),
        },
        ty => return Err(Error::OtherError(format!("fill_unconstrained_with_length: expected out_len_ptr to be some pointer type, got {:?}", ty))),
    };

    let out_buffer = state.operand_to_bv(out_buffer)?;
    let out_len_ptr = state.operand_to_bv(out_len_ptr)?;

    // write the output length
    let out_len = state.new_bv_with_name(Name::from(format!("{}_length", buffer_name)), out_len_bitwidth)?;
    out_len.ulte(&B::BV::from_u32(state.solver.clone(), max_buffer_len_bytes, out_len_bitwidth)).assert()?;
    state.write(&out_len_ptr, out_len)?;

    // write the buffer contents
    let unconstrained_bytes = state.new_bv_with_name(Name::from(buffer_name), max_buffer_len_bytes * 8)?;
    state.write(&out_buffer, unconstrained_bytes)?;

    Ok(())
}

/// This utility function fills a buffer with secret data, and also outputs the number of bytes written.
///
/// The entire `max_buffer_len_bytes`-byte buffer will be written, but the output number of bytes will be constrained to be any number between 0 and `max_buffer_len_bytes`.
pub fn fill_secret_with_length(
    state: &mut State<secret::Backend>,
    out_buffer: &Operand,  // address of output buffer
    out_len_ptr: &Operand,  // address of a 64-bit integer which gets the number of bytes written
    max_buffer_len_bytes: u32,
    buffer_name: String,
) -> Result<()> {
    let out_len_bitwidth: u32 = 64;  // we assume that the out_len is a 64-bit integer

    // sanity-check some argument types
    match out_buffer.get_type() {
        Type::PointerType { .. } => {},
        ty => return Err(Error::OtherError(format!("fill_secret_with_length: expected out_buffer to be some pointer type, got {:?}", ty))),
    };
    match out_len_ptr.get_type() {
        Type::PointerType { pointee_type, .. } => match *pointee_type {
            Type::IntegerType { bits } if bits == out_len_bitwidth => {},
            _ => return Err(Error::OtherError(format!("fill_secret_with_length: expected out_len_ptr to be pointer-to-64-bit-integer type, got pointer to {:?}", pointee_type))),
        },
        ty => return Err(Error::OtherError(format!("fill_secret_with_length: expected out_len_ptr to be some pointer type, got {:?}", ty))),
    };

    let out_buffer = state.operand_to_bv(out_buffer)?;
    let out_len_ptr = state.operand_to_bv(out_len_ptr)?;

    // write the output length
    let out_len = state.new_bv_with_name(Name::from(format!("{}_length", buffer_name)), out_len_bitwidth)?;
    out_len.ulte(&secret::BV::from_u32(state.solver.clone(), max_buffer_len_bytes, out_len_bitwidth)).assert()?;
    state.write(&out_len_ptr, out_len)?;

    // write the buffer contents
    let secret_bytes = secret::BV::Secret { btor: state.solver.clone(), width: max_buffer_len_bytes * 8, symbol: Some(buffer_name) };
    state.write(&out_buffer, secret_bytes)?;

    Ok(())
}
