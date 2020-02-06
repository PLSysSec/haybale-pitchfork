//! This module contains helper functions that may be useful in writing function hooks.

use crate::{allocation, secret, AbstractData, StructDescriptions};
use either::Either;
use haybale::{Error, Project, Result, State};
use haybale::backend::*;
use llvm_ir::*;

/// This utility function fills a buffer with unconstrained data, and also outputs the number of bytes written.
///
/// The entire `max_buffer_len_bytes`-byte buffer will be written, but the output number of bytes will be constrained to be any number between 0 and `max_buffer_len_bytes`.
///
/// For the buffer and length pointer, this accepts either an `Operand` or a `BV`.
/// If an `Operand` is provided, it will be type-checked and converted to `BV`.
/// If a `BV` is provided, we'll assume the caller has done all appropriate typechecking.
pub fn fill_unconstrained_with_length<B: Backend>(
    state: &mut State<B>,
    out_buffer: Either<&Operand, B::BV>,  // address of output buffer
    out_len_ptr: Either<&Operand, B::BV>,  // address of a 64-bit integer which gets the number of bytes written
    max_buffer_len_bytes: u32,
    buffer_name: String,
) -> Result<()> {
    let out_len_bitwidth: u32 = 64;  // we assume that the out_len is a 64-bit integer

    let out_buffer: B::BV = match out_buffer {
        Either::Left(op) => {
            // sanity-check the type
            match op.get_type() {
                Type::PointerType { .. } => {},
                ty => return Err(Error::OtherError(format!("fill_unconstrained_with_length: expected out_buffer to be some pointer type, got {:?}", ty))),
            };
            state.operand_to_bv(op)?
        },
        Either::Right(bv) => bv,
    };
    let out_len_ptr: B::BV = match out_len_ptr {
        Either::Left(op) => {
            // sanity-check the type
            match op.get_type() {
                Type::PointerType { pointee_type, .. } => match *pointee_type {
                    Type::IntegerType { bits } if bits == out_len_bitwidth => {},
                    _ => return Err(Error::OtherError(format!("fill_unconstrained_with_length: expected out_len_ptr to be pointer-to-64-bit-integer type, got pointer to {:?}", pointee_type))),
                },
                ty => return Err(Error::OtherError(format!("fill_unconstrained_with_length: expected out_len_ptr to be some pointer type, got {:?}", ty))),
            };
            state.operand_to_bv(op)?
        },
        Either::Right(bv) => bv,
    };

    // write the output length
    let out_len = state.new_bv_with_name(Name::from(format!("{}_length", buffer_name)), out_len_bitwidth)?;
    out_len.ulte(&state.bv_from_u32(max_buffer_len_bytes, out_len_bitwidth)).assert()?;
    state.write(&out_len_ptr, out_len)?;

    // write the buffer contents
    let unconstrained_bytes = state.new_bv_with_name(Name::from(buffer_name), max_buffer_len_bytes * 8)?;
    state.write(&out_buffer, unconstrained_bytes)?;

    Ok(())
}

/// This utility function fills a buffer with secret data, and also outputs the number of bytes written.
///
/// The entire `max_buffer_len_bytes`-byte buffer will be written, but the output number of bytes will be constrained to be any number between 0 and `max_buffer_len_bytes`.
///
/// For the buffer and length pointer, this accepts either an `Operand` or a `BV`.
/// If an `Operand` is provided, it will be type-checked and converted to `BV`.
/// If a `BV` is provided, we'll assume the caller has done all appropriate typechecking.
pub fn fill_secret_with_length(
    state: &mut State<secret::Backend>,
    out_buffer: Either<&Operand, secret::BV>,  // address of output buffer
    out_len_ptr: Either<&Operand, secret::BV>,  // address of a 64-bit integer which gets the number of bytes written
    max_buffer_len_bytes: u32,
    buffer_name: String,
) -> Result<()> {
    let out_len_bitwidth: u32 = 64;  // we assume that the out_len is a 64-bit integer

    let out_buffer: secret::BV = match out_buffer {
        Either::Left(op) => {
            // sanity-check the type
            match op.get_type() {
                Type::PointerType { .. } => {},
                ty => return Err(Error::OtherError(format!("fill_secret_with_length: expected out_buffer to be some pointer type, got {:?}", ty))),
            };
            state.operand_to_bv(op)?
        },
        Either::Right(bv) => bv,
    };
    let out_len_ptr: secret::BV = match out_len_ptr {
        Either::Left(op) => {
            // sanity-check the type
            match op.get_type() {
                Type::PointerType { pointee_type, .. } => match *pointee_type {
                    Type::IntegerType { bits } if bits == out_len_bitwidth => {},
                    _ => return Err(Error::OtherError(format!("fill_secret_with_length: expected out_len_ptr to be pointer-to-64-bit-integer type, got pointer to {:?}", pointee_type))),
                },
                ty => return Err(Error::OtherError(format!("fill_secret_with_length: expected out_len_ptr to be some pointer type, got {:?}", ty))),
            };
            state.operand_to_bv(op)?
        },
        Either::Right(bv) => bv,
    };

    // write the output length
    let out_len = state.new_bv_with_name(Name::from(format!("{}_length", buffer_name)), out_len_bitwidth)?;
    out_len.ulte(&state.bv_from_u32(max_buffer_len_bytes, out_len_bitwidth)).assert()?;
    state.write(&out_len_ptr, out_len)?;

    // write the buffer contents
    let secret_bytes = secret::BV::Secret { btor: state.solver.clone(), width: max_buffer_len_bytes * 8, symbol: Some(buffer_name) };
    state.write(&out_buffer, secret_bytes)?;

    Ok(())
}

/// This helper function allocates space for the given `AbstractData`,
/// initializes it, and returns a pointer to the newly-allocated space.
pub fn allocate_and_init_abstractdata<'p>(
    proj: &'p Project,
    state: &mut State<'p, secret::Backend>,
    ad: AbstractData,
    ty: &Type,  // Type of the AbstractData
    sd: &'p StructDescriptions,
) -> Result<secret::BV> {
    let ad = ad.to_complete(ty, proj, sd);
    let ptr = state.allocate(ad.size_in_bits() as u64);
    let mut allocationctx = allocation::Context::new(proj, state, sd);
    allocation::InitializationContext::blank().initialize_cad_in_memory(&mut allocationctx, &ptr, &ad, Some(ty))?;
    Ok(ptr)
}

/// This helper function reinitializes whatever is pointed to by the given
/// pointer, according to the given `AbstractData`.
pub fn reinitialize_pointee<'p>(
    proj: &'p Project,
    state: &mut State<'p, secret::Backend>,
    pointer: &Operand,  // we'll reinitialize the [struct, array, whatever] that this points to
    ad: AbstractData,  // `AbstractData` describing the _pointee_ (not the pointer) and how to reinitialize it
    sd: &'p StructDescriptions,
) -> Result<()> {
    let ptr = state.operand_to_bv(pointer)?;
    let pointee_ty = match pointer.get_type() {
        Type::PointerType { pointee_type, .. } => pointee_type,
        ty => return Err(Error::OtherError(format!("reinitialize_pointee: expected `pointer` to be a pointer, got {:?}", ty))),
    };
    let mut allocationctx = allocation::Context::new(proj, state, sd);
    allocation::InitializationContext::blank().initialize_data_in_memory(&mut allocationctx, &ptr, ad, &pointee_ty, proj, sd)?;
    Ok(())
}
