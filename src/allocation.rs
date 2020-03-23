use crate::abstractdata::*;
use crate::secret;
use haybale::{layout, Project, State};
use haybale::backend::*;
use haybale::Result;
use haybale::watchpoints::Watchpoint;
use llvm_ir::*;
use log::debug;
use std::collections::HashMap;
use std::collections::hash_map::Entry::*;
use std::fmt;
use std::sync::{Arc, RwLock};

/// Allocate the function parameters given in `params` with their corresponding `AbstractData` descriptions.
///
/// Returns a vector of the `secret::BV`s representing the parameters. Many callers won't need this, though.
pub fn allocate_args<'p>(
    proj: &'p Project,
    state: &mut State<'p, secret::Backend>,
    sd: &StructDescriptions,
    params: impl IntoIterator<Item = (&'p function::Parameter, AbstractData)>,
) -> Result<Vec<secret::BV>> {
    let mut ctx = Context::new(proj, state, sd);
    params.into_iter().map(|(param, arg)| ctx.allocate_arg(param, arg)).collect()
}

/// This `Context` serves two purposes:
/// first, simply collecting some objects together so we can pass them around as a unit;
/// but second, allowing some state to persist across invocations of `allocate_arg`
/// (particularly, tracking `AbstractValue::Named` values, thus allowing names used for one arg to reference values defined for another)
pub struct Context<'p, 's> {
    proj: &'p Project,
    state: &'s mut State<'p, secret::Backend>,
    sd: &'s StructDescriptions,
    namedvals: HashMap<String, secret::BV>,
}

impl<'p, 's> Context<'p, 's> {
    pub fn new(proj: &'p Project, state: &'s mut State<'p, secret::Backend>, sd: &'s StructDescriptions) -> Self {
        Self {
            proj,
            state,
            sd,
            namedvals: HashMap::new(),
        }
    }

    /// Returns the `secret::BV` representing the argument. Many callers won't need this, though.
    fn allocate_arg(&mut self, param: &'p function::Parameter, arg: AbstractData) -> Result<secret::BV> {
        debug!("Allocating function parameter {:?}", &param.name);
        let arg = arg.to_complete(&param.ty, &self.proj, &self.sd);
        self.allocate_arg_from_cad(param, arg, false)
    }

    /// Same as above, but takes a `CompleteAbstractData` instead of an `AbstractData`.
    ///
    /// `type_override`: If `true`, then the parameter type will not be checked against the `CompleteAbstractData`.
    fn allocate_arg_from_cad(
        &mut self,
        param: &'p function::Parameter,
        arg: CompleteAbstractData,
        type_override: bool,
    ) -> Result<secret::BV> {
        let arg_size = arg.size_in_bits();
        match layout::size_opaque_aware(&param.ty, self.proj) {
            Some(param_size) => assert_eq!(arg_size, param_size, "Parameter size mismatch for parameter {:?}: parameter is {} bits but CompleteAbstractData is {} bits", &param.name, param_size, arg_size),
            None => {},  // can't determine the parameter size: skip performing this check
        };
        match arg {
            CompleteAbstractData::Secret { bits } => {
                debug!("Parameter is marked secret");
                let bv = secret::BV::Secret { btor: self.state.solver.clone(), width: bits as u32, symbol: None };
                self.state.overwrite_latest_version_of_bv(&param.name, bv.clone());
                Ok(bv)
            },
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
                debug!("Parameter is marked public, equal to {}", value);
                let bv = self.state.bv_from_u64(value, bits as u32);
                self.state.overwrite_latest_version_of_bv(&param.name, bv.clone());
                Ok(bv)
            },
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
                debug!("Parameter is marked public, in the range ({}, {}) inclusive", min, max);
                let parambv = self.state.new_bv_with_name(param.name.clone(), bits as u32).unwrap();
                parambv.ugte(&self.state.bv_from_u64(min, bits as u32)).assert()?;
                parambv.ulte(&self.state.bv_from_u64(max, bits as u32)).assert()?;
                self.state.overwrite_latest_version_of_bv(&param.name, parambv.clone());
                Ok(parambv)
            }
            CompleteAbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
                debug!("Parameter is marked public, unconstrained value");
                // nothing to do, just return the BV representing that parameter
                let op = Operand::LocalOperand { name: param.name.clone(), ty: param.ty.clone() };
                self.state.operand_to_bv(&op)
            },
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::Named { name, value } } => {
                let unwrapped_arg = CompleteAbstractData::pub_integer(bits, *value);
                let bv = self.allocate_arg_from_cad(param, unwrapped_arg, type_override)?;
                match self.namedvals.entry(name.to_owned()) {
                    Vacant(v) => {
                        v.insert(bv.clone());
                    },
                    Occupied(bv_for_name) => {
                        let bv_for_name = bv_for_name.get();
                        let width = bv_for_name.get_width();
                        assert_eq!(width, bits as u32, "AbstractValue::Named {:?}: multiple values with different bitwidths given this name: one with width {} bits, another with width {} bits", name, width, bits);
                        bv._eq(&bv_for_name).assert()?;
                    },
                };
                self.state.overwrite_latest_version_of_bv(&param.name, bv.clone());
                Ok(bv)
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::EqualTo(name) } => {
                match self.namedvals.get(&name) {
                    None => panic!("AbstractValue::Named {:?} not found", name),
                    Some(bv) => {
                        let width = bv.get_width();
                        assert_eq!(width, bits as u32, "AbstractValue::EqualTo {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        self.state.overwrite_latest_version_of_bv(&param.name, bv.clone());
                        Ok(bv.clone())
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedLessThan(name) } => {
                match self.namedvals.get(&name) {
                    None => panic!("AbstractValue::Named {:?} not found", name),
                    Some(bv) => {
                        let width = bv.get_width();
                        assert_eq!(width, bits as u32, "AbstractValue::SignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        let new_bv = self.state.new_bv_with_name(Name::from(format!("SignedLessThan{}:", name)), width)?;
                        new_bv.slt(&bv).assert()?;
                        self.state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                        Ok(new_bv)
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedGreaterThan(name) } => {
                match self.namedvals.get(&name) {
                    None => panic!("AbstractValue::Named {:?} not found", name),
                    Some(bv) => {
                        let width = bv.get_width();
                        assert_eq!(width, bits as u32, "AbstractValue::SignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        let new_bv = self.state.new_bv_with_name(Name::from(format!("SignedGreaterThan:{}", name)), width)?;
                        new_bv.sgt(&bv).assert()?;
                        self.state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                        Ok(new_bv)
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedLessThan(name) } => {
                match self.namedvals.get(&name) {
                    None => panic!("AbstractValue::Named {:?} not found", name),
                    Some(bv) => {
                        let width = bv.get_width();
                        assert_eq!(width, bits as u32, "AbstractValue::UnsignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        let new_bv = self.state.new_bv_with_name(Name::from(format!("UnsignedLessThan:{}", name)), width)?;
                        new_bv.ult(&bv).assert()?;
                        self.state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                        Ok(new_bv)
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedGreaterThan(name) } => {
                match self.namedvals.get(&name) {
                    None => panic!("AbstractValue::Named {:?} not found", name),
                    Some(bv) => {
                        let width = bv.get_width();
                        assert_eq!(width, bits as u32, "AbstractValue::UnsignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        let new_bv = self.state.new_bv_with_name(Name::from(format!("UnsignedGreaterThan:{}", name)), width)?;
                        new_bv.ugt(&bv).assert()?;
                        self.state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                        Ok(new_bv)
                    }
                }
            }
            CompleteAbstractData::PublicPointerTo { pointee, maybe_null } => {
                debug!("Parameter is marked as a public pointer which {} be null", if maybe_null { "may" } else { "cannot" });
                let ptr = self.state.allocate(pointee.size_in_bits() as u64);
                debug!("Allocated the parameter at {:?}", ptr);
                if maybe_null {
                    let ptr_width = ptr.get_width();
                    let condition = self.state.new_bv_with_name(Name::from("pointer_is_null"), 1)?;
                    let maybe_null_ptr = condition.cond_bv(&self.state.zero(ptr_width), &ptr);
                    self.state.overwrite_latest_version_of_bv(&param.name, maybe_null_ptr);
                } else {
                    self.state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
                }
                // in either case, initialize the concrete pointer, not the maybe-null location
                if type_override {
                    InitializationContext::blank().initialize_cad_in_memory(self, &ptr, &*pointee, None)?;
                } else {
                    let pointee_ty = match &param.ty {
                        Type::PointerType { pointee_type, .. } => pointee_type,
                        ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
                    };
                    InitializationContext::blank().initialize_cad_in_memory(self, &ptr, &*pointee, Some(pointee_ty))?;
                }
                Ok(ptr)
            },
            CompleteAbstractData::PublicPointerToFunction(funcname) => {
                debug!("Parameter is marked as a public pointer to the function {:?}", funcname);
                if type_override {
                    match &param.ty {
                        Type::PointerType { .. } => {},
                        ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
                    };
                }
                let ptr = self.state.get_pointer_to_function(funcname.clone())
                    .unwrap_or_else(|| panic!("Failed to find function {:?}", &funcname))
                    .clone();
                self.state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
                Ok(ptr)
            }
            CompleteAbstractData::PublicPointerToHook(funcname) => {
                debug!("Parameter is marked as a public pointer to the active hook for function {:?}", funcname);
                if type_override {
                    match &param.ty {
                        Type::PointerType { .. } => {},
                        ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
                    };
                }
                let ptr = self.state.get_pointer_to_function_hook(&funcname)
                    .unwrap_or_else(|| panic!("Failed to find hook for function {:?}", &funcname))
                    .clone();
                self.state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
                Ok(ptr)
            }
            CompleteAbstractData::PublicPointerToSelf => panic!("Pointer-to-self is not supported for toplevel parameter (requires support for struct-passed-by-value, which at the time of this writing is also unimplemented)"),
            CompleteAbstractData::PublicPointerToParentOr(_) => panic!("Pointer-to-parent is not supported for toplevel parameter; we have no way to know what struct it is contained in"),
            CompleteAbstractData::Array { .. } => unimplemented!("Array passed by value"),
            CompleteAbstractData::Struct { .. } => unimplemented!("Struct passed by value"),
            CompleteAbstractData::VoidOverride { .. } => unimplemented!("VoidOverride used as an argument directly.  You probably meant to use a pointer to a VoidOverride"),
            CompleteAbstractData::PointerOverride { llvm_struct_name, data } => {
                debug!("Parameter is marked as a public pointer to {}, overriding LLVM type", data);
                let ptr = self.state.allocate(data.size_in_bits() as u64);
                debug!("Allocated the parameter at {:?}", ptr);
                self.state.overwrite_latest_version_of_bv(&param.name, ptr.clone());

                if !type_override {
                    // typecheck that the parameter is at least a pointer
                    match &param.ty {
                        Type::PointerType { .. } => (),
                        ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
                    };
                }

                match llvm_struct_name {
                    None => {
                        InitializationContext::blank().initialize_cad_in_memory(self, &ptr, &data, None)?;
                    },
                    Some(llvm_struct_name) => {
                        let (llvm_ty, _) = self.proj.get_named_struct_type_by_name(&llvm_struct_name)
                            .unwrap_or_else(|| { panic!("PointerOverride: llvm_struct_name {:?} not found in Project", llvm_struct_name) });
                        let arc = llvm_ty.as_ref().unwrap_or_else(|| { panic!("PointerOverride: llvm_struct_name {:?} is an opaque type", llvm_struct_name) });
                        let llvm_ty: &Type = &arc.read().unwrap();
                        InitializationContext::blank().initialize_cad_in_memory(self, &ptr, &data, Some(llvm_ty))?;
                    },
                }

                Ok(ptr)
            },
            CompleteAbstractData::SameSizeOverride { data } => {
                // we already checked above that the param size == the data size; and we will again on the recursive call, actually
                self.allocate_arg_from_cad(param, *data, true)
            },
            CompleteAbstractData::WithWatchpoint { .. } => unimplemented!("WithWatchpoint is not supported for toplevel parameter (its value usually does not reside in memory). You may want a pointer to a WithWatchpoint instead"),
        }
    }
}

/// As opposed to the `Context`, which contains global-ish state preserved across
/// all allocations (even of different function args), this
/// `InitializationContext` contains more immediate information about where we
/// are and what we're doing.
///
/// It is not preserved across different invocations of
/// `initialize_data_in_memory` - note that `initialize_data_in_memory` takes an
/// _owned_ `self`, so it will consume and destroy the `InitializationContext`
/// when the initialization is done. Outside callers need a fresh
/// `InitializationContext` to start each `initialize_data_in_memory`.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct InitializationContext<'a> {
    /// If present, this is a pointer to the struct containing the element we're
    /// initializing, as well as the type of that struct.
    cur_struct: Option<(&'a secret::BV, &'a Type)>,

    /// If present, this is a pointer to the struct containing `cur_struct`,
    /// as well as the type of that parent struct.
    parent: Option<(&'a secret::BV, &'a Type)>,

    /// Description of the struct we are currently within (and the struct that is
    /// within, etc), purely for debugging purposes. First in the vec is the
    /// top-level struct, last is the most immediate struct.
    within_structs: Vec<WithinStruct>,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
struct WithinStruct {
    /// Name of the struct we are within
    name: String,
    /// Index of the element in that struct which we are within (0-indexed)
    element_index: usize,
}

impl fmt::Display for WithinStruct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "struct {:?}, element {}", self.name, self.element_index)
    }
}

impl<'a> InitializationContext<'a> {
    /// A default/blank initialization context. If you're not doing anything fancy,
    /// this is what you're looking for.
    pub fn blank() -> Self {
        Self {
            cur_struct: None,
            parent: None,
            within_structs: Vec::new(),
        }
    }

    fn error_backtrace(&self) {
        eprintln!();
        for w in &self.within_structs {
            eprintln!("within {}:", w);
        }
    }

    /// Check that `ty` represents a value of `bits` bits, panicking if not
    fn size_check_ty(&self, ctx: &Context, ty: &'a Type, bits: usize) {
        match layout::size_opaque_aware(ty, ctx.proj) {
            Some(ty_size_bits) => {
                if bits != ty_size_bits {
                    self.error_backtrace();
                    panic!("Size mismatch: type {:?} is {} bits but CompleteAbstractData is {} bits", ty, ty_size_bits, bits);
                }
            },
            None => {},  // can't determine the size of `ty`; skip performing the check
        }
    }

    /// Initialize the data in memory at `addr` according to the given `AbstractData`.
    ///
    /// `ty` should be the type of the pointed-to object (i.e., the type of the
    /// `AbstractData`), not the type of `addr`.
    ///
    /// Returns the number of _bits_ (not bytes) the initialized data takes. Many callers won't need this, though.
    pub fn initialize_data_in_memory(
        self,
        ctx: &mut Context,
        addr: &'a secret::BV,
        data: AbstractData,
        ty: &'a Type,
    ) -> Result<usize> {
        self.initialize_cad_in_memory(ctx, addr, &data.to_complete(ty, ctx.proj, ctx.sd), Some(ty))
    }

    /// Like `initialize_data_in_memory`, but takes a `CompleteAbstractData`
    /// instead of an `AbstractData`.
    ///
    /// Also, `ty` is optional here. As before, `ty` represents the type of the
    /// pointed-to object, not the type of `addr`. In this function, `ty` is used
    /// only for type-checking, to ensure that the `CompleteAbstractData`
    /// actually matches the intended LLVM type.
    /// Setting `ty` to `None` disables this type-checking.
    pub(crate) fn initialize_cad_in_memory(
        mut self,
        ctx: &mut Context,
        addr: &'a secret::BV,
        data: &CompleteAbstractData,
        ty: Option<&'a Type>,
    ) -> Result<usize> {
        // First we handle the case where the LLVM type is array-of-one-element
        if let Some(Type::ArrayType { num_elements: 1, element_type }) | Some(Type::VectorType { num_elements: 1, element_type }) = ty {
            match data {
                CompleteAbstractData::Array { num_elements: 1, element_type: element_abstractdata } => {
                    // both LLVM and CAD type are array-of-one-element.  Unwrap and call recursively
                    return self.initialize_cad_in_memory(ctx, addr, element_abstractdata, Some(element_type));
                },
                data => {
                    // LLVM type is array-of-one-element but CAD type is not.  Unwrap the LLVM type and call recursively
                    return self.initialize_cad_in_memory(ctx, addr, data, Some(element_type));
                },
            }
        };

        // Then we handle the case where the LLVM type is struct-of-one-element
        match ty {
            Some(Type::StructType { element_types, .. }) if element_types.len() == 1 => {
                if !data.could_describe_a_struct_of_one_element() {
                    // `data` specifies some incompatible type.  Unwrap the LLVM struct and try again.
                    return self.initialize_cad_in_memory(ctx, addr, data, Some(&element_types[0]));
                }
            },
            Some(ty@Type::NamedStructType { .. }) => {
                match ctx.proj.get_inner_struct_type_from_named(ty) {
                    None => {},  // we're looking for where LLVM type is a struct of one element. Opaque struct type is a different problem.
                    Some(arc) => {
                        let actual_ty: &Type = &arc.read().unwrap();
                        if let Type::StructType { element_types, .. } = actual_ty {
                            if element_types.len() == 1 {
                                // the LLVM type is struct of one element.  Proceed as in the above case
                                if !data.could_describe_a_struct_of_one_element() {
                                    // `data` specifies some incompatible type.  Unwrap the LLVM struct and try again.
                                    // we could consider pushing the named struct name to within_structs here
                                    return self.initialize_cad_in_memory(ctx, addr, data, Some(&element_types[0]));
                                }
                            }
                        }
                    }
                }
            },
            _ => {},  // LLVM type isn't struct of one element.  Continue.
        }

        // Otherwise, on to normal processing
        debug!("Initializing data in memory at address {:?}", addr);
        debug!("Memory contents are marked as {:?}", data.to_string());
        match data {
            CompleteAbstractData::Secret { bits } => {
                debug!("marking {} bits secret at address {:?}", bits, addr);
                let bv = secret::BV::Secret { btor: ctx.state.solver.clone(), width: *bits as u32, symbol: None };
                ctx.state.write(&addr, bv)?;
                Ok(*bits)
            },
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
                debug!("setting the memory contents equal to {}", value);
                if let Some(ty) = ty {
                    self.size_check_ty(ctx, ty, *bits);
                }
                let bv = ctx.state.bv_from_u64(*value, *bits as u32);
                ctx.state.write(&addr, bv)?;
                Ok(*bits)
            },
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
                debug!("constraining the memory contents to be in the range ({}, {}) inclusive", min, max);
                if let Some(ty) = ty {
                    self.size_check_ty(ctx, ty, *bits);
                }
                let bv = ctx.state.read(&addr, *bits as u32)?;
                bv.ugte(&ctx.state.bv_from_u64(*min, *bits as u32)).assert()?;
                bv.ulte(&ctx.state.bv_from_u64(*max, *bits as u32)).assert()?;
                Ok(*bits)
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::Unconstrained } => {
                // nothing to do, just check that the type matches
                if let Some(ty) = ty {
                    self.size_check_ty(ctx, ty, *bits);
                }
                Ok(*bits)
            },
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::Named { name, value } } => {
                let unwrapped_data = CompleteAbstractData::pub_integer(*bits, (**value).clone());
                let initialized_bits = self.clone().initialize_cad_in_memory(ctx, addr, &unwrapped_data, ty)?;
                if *bits != initialized_bits {
                    self.error_backtrace();
                    panic!("AbstractValue::Named {:?}: specified {} bits, but value is {} bits", name, bits, initialized_bits);
                }
                let bv = ctx.state.read(addr, *bits as u32)?;
                match ctx.namedvals.entry(name.to_owned()) {
                    Vacant(v) => {
                        v.insert(bv);
                    },
                    Occupied(bv_for_name) => {
                        let bv_for_name = bv_for_name.get();
                        let width = bv_for_name.get_width();
                        assert_eq!(width, *bits as u32, "AbstractValue::Named {:?}: multiple values with different bitwidths given this name: one with width {} bits, another with width {} bits", name, width, *bits);
                        bv._eq(&bv_for_name).assert()?;
                    },
                };
                Ok(*bits)
            },
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::EqualTo(name) } => {
                match ctx.namedvals.get(name) {
                    None => {
                        self.error_backtrace();
                        panic!("AbstractValue::Named {:?} not found", name)
                    },
                    Some(bv) => {
                        let width = bv.get_width();
                        if width != *bits as u32 {
                            self.error_backtrace();
                            panic!("AbstractValue::EqualTo {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        }
                        if let Some(ty) = ty {
                            self.size_check_ty(ctx, ty, *bits);
                        }
                        ctx.state.write(&addr, bv.clone())?;
                        Ok(*bits)
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedLessThan(name) } => {
                match ctx.namedvals.get(name) {
                    None => {
                        self.error_backtrace();
                        panic!("AbstractValue::Named {:?} not found", name)
                    },
                    Some(bv) => {
                        let width = bv.get_width();
                        if width != *bits as u32 {
                            self.error_backtrace();
                            panic!("AbstractValue::SignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        }
                        if let Some(ty) = ty {
                            self.size_check_ty(ctx, ty, *bits);
                        }
                        let new_bv = ctx.state.new_bv_with_name(Name::from(format!("SignedLessThan:{}", name)), width)?;
                        new_bv.slt(&bv).assert()?;
                        ctx.state.write(&addr, new_bv)?;
                        Ok(*bits)
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedGreaterThan(name) } => {
                match ctx.namedvals.get(name) {
                    None => {
                        self.error_backtrace();
                        panic!("AbstractValue::Named {:?} not found", name)
                    },
                    Some(bv) => {
                        let width = bv.get_width();
                        if width != *bits as u32 {
                            self.error_backtrace();
                            panic!("AbstractValue::SignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        }
                        if let Some(ty) = ty {
                            self.size_check_ty(ctx, ty, *bits);
                        }
                        let new_bv = ctx.state.new_bv_with_name(Name::from(format!("SignedGreaterThan:{}", name)), width)?;
                        new_bv.sgt(&bv).assert()?;
                        ctx.state.write(&addr, new_bv)?;
                        Ok(*bits)
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedLessThan(name) } => {
                match ctx.namedvals.get(name) {
                    None => {
                        self.error_backtrace();
                        panic!("AbstractValue::Named {:?} not found", name)
                    },
                    Some(bv) => {
                        let width = bv.get_width();
                        if width != *bits as u32 {
                            self.error_backtrace();
                            panic!("AbstractValue::UnsignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        }
                        if let Some(ty) = ty {
                            self.size_check_ty(ctx, ty, *bits);
                        }
                        let new_bv = ctx.state.new_bv_with_name(Name::from(format!("UnsignedLessThan:{}", name)), width)?;
                        new_bv.ult(&bv).assert()?;
                        ctx.state.write(&addr, new_bv)?;
                        Ok(*bits)
                    }
                }
            }
            CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedGreaterThan(name) } => {
                match ctx.namedvals.get(name) {
                    None => {
                        self.error_backtrace();
                        panic!("AbstractValue::Named {:?} not found", name)
                    },
                    Some(bv) => {
                        let width = bv.get_width();
                        if width != *bits as u32 {
                            self.error_backtrace();
                            panic!("AbstractValue::UnsignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                        }
                        if let Some(ty) = ty {
                            self.size_check_ty(ctx, ty, *bits);
                        }
                        let new_bv = ctx.state.new_bv_with_name(Name::from(format!("UnsignedGreaterThan:{}", name)), width)?;
                        new_bv.ugt(&bv).assert()?;
                        ctx.state.write(&addr, new_bv)?;
                        Ok(*bits)
                    }
                }
            }
            CompleteAbstractData::PublicPointerTo { pointee, maybe_null } => {
                debug!("memory contents are marked as a public pointer which {} be null", if *maybe_null { "may" } else { "cannot"});

                // type-check
                let pointee_ty = ty.map(|ty| match ty {
                    Type::PointerType { pointee_type, .. } => &**pointee_type,
                    _ => {
                        self.error_backtrace();
                        panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty)
                    },
                });

                // allocate memory for the pointee
                let inner_ptr = ctx.state.allocate(pointee.size_in_bits() as u64);
                let bits = inner_ptr.get_width();
                debug!("allocated memory for the pointee at {:?}, and will constrain the memory contents at {:?} to have that pointer value{}", inner_ptr, addr, if *maybe_null { " or null" } else { "" });

                // make `addr` point to a pointer to the newly allocated memory (or point to NULL if appropriate)
                if *maybe_null {
                    let condition = ctx.state.new_bv_with_name(Name::from("pointer_is_null"), 1)?;
                    let maybe_null_ptr = condition.cond_bv(&ctx.state.zero(bits), &inner_ptr);
                    ctx.state.write(&addr, maybe_null_ptr)?;
                } else {
                    ctx.state.write(&addr, inner_ptr.clone())?;
                };

                // in either case, initialize the pointee at the concrete address (not at the maybe-null location)
                self.initialize_cad_in_memory(ctx, &inner_ptr, &**pointee, pointee_ty)?;

                Ok(bits as usize)
            },
            CompleteAbstractData::PublicPointerToFunction(funcname) => {
                if let Some(ty) = ty {
                    match ty {
                        Type::PointerType { .. } => {},
                        _ => {
                            self.error_backtrace();
                            panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty)
                        },
                    };
                }
                let inner_ptr = ctx.state.get_pointer_to_function(funcname.clone())
                    .unwrap_or_else(|| { self.error_backtrace(); panic!("Failed to find function {:?}", &funcname) })
                    .clone();
                debug!("setting the memory contents equal to {:?}", inner_ptr);
                let bits = inner_ptr.get_width();
                ctx.state.write(&addr, inner_ptr)?; // make `addr` point to a pointer to the function
                Ok(bits as usize)
            }
            CompleteAbstractData::PublicPointerToHook(funcname) => {
                if let Some(ty) = ty {
                    match ty {
                        Type::PointerType { .. } => {},
                        _ => {
                            self.error_backtrace();
                            panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty)
                        },
                    };
                }
                let inner_ptr = ctx.state.get_pointer_to_function_hook(funcname)
                    .unwrap_or_else(|| { self.error_backtrace(); panic!("Failed to find hook for function {:?}", &funcname) })
                    .clone();
                debug!("setting the memory contents equal to {:?}", inner_ptr);
                let bits = inner_ptr.get_width();
                ctx.state.write(&addr, inner_ptr)?; // make `addr` point to a pointer to the hook
                Ok(bits as usize)
            }
            CompleteAbstractData::PublicPointerToSelf => {
                match self.cur_struct {
                    None => {
                        self.error_backtrace();
                        panic!("Pointer-to-self used but there is no current struct")
                    },
                    Some((cur_struct_ptr, cur_struct_ty)) => {
                        // first typecheck: is this actually a pointer to the correct struct type
                        match ty {
                            Some(Type::PointerType { pointee_type, .. }) => {
                                let pointee_ty = &**pointee_type;
                                if pointee_ty == cur_struct_ty {
                                    // typecheck passes, do nothing
                                } else if let Type::NamedStructType { name, .. } = pointee_ty {
                                    // LLVM type is pointer to a named struct type, try unwrapping it and see if that makes the types equal
                                    let arc = ctx.proj.get_inner_struct_type_from_named(pointee_ty).unwrap_or_else(|| {
                                        self.error_backtrace();
                                        panic!("CompleteAbstractData specifies pointer-to-self, but self type (struct named {:?}) is fully opaque and has no definition in this Project", name);
                                    });
                                    let actual_ty: &Type = &arc.read().unwrap();
                                    if actual_ty == cur_struct_ty {
                                        // typecheck passes, do nothing
                                    } else {
                                        self.error_backtrace();
                                        panic!("Type mismatch: CompleteAbstractData specifies pointer-to-self, but found pointer to a different type.\n  Self type: {:?}\n  Found type: struct named {:?}: {:?}\n", cur_struct_ty, name, actual_ty);
                                    }
                                } else {
                                    self.error_backtrace();
                                    panic!("Type mismatch: CompleteAbstractData specifies pointer-to-self, but found pointer to a different type.\n  Self type: {:?}\n  Found type: {:?}\n", cur_struct_ty, pointee_ty);
                                }
                            },
                            Some(_) => {
                                self.error_backtrace();
                                panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty)
                            },
                            None => {},
                        };
                        // typecheck passed, write the pointer
                        debug!("setting the memory contents equal to {:?}", cur_struct_ptr);
                        let bits = cur_struct_ptr.get_width();
                        ctx.state.write(&addr, cur_struct_ptr.clone())?;
                        Ok(bits as usize)
                    }
                }
            }
            CompleteAbstractData::PublicPointerToParentOr(pointee) => {
                // this parent_ptr will be None if the parent type mismatches
                let parent_ptr: Option<_> = match self.parent {
                    None => None,  // no parent, so it's not the correct type; we'll use the `pointee` data, if it was provided
                    Some((parent_ptr, parent_ty)) => match ty {
                        Some(Type::PointerType { pointee_type, .. }) => {
                            let pointee_ty = &**pointee_type;
                            if pointee_ty == parent_ty {
                                Some(parent_ptr)
                            } else if let Type::NamedStructType { name, .. } = pointee_ty {
                                // LLVM type is pointer to a named struct type, try unwrapping it and see if that makes the types equal
                                let arc = ctx.proj.get_inner_struct_type_from_named(pointee_ty).unwrap_or_else(|| {
                                    self.error_backtrace();
                                    panic!("CompleteAbstractData specifies pointer-to-parent, but parent type (struct named {:?}) is fully opaque and has no definition in this Project", name);
                                });
                                let actual_ty: &Type = &arc.read().unwrap();
                                if actual_ty == parent_ty {
                                    Some(parent_ptr)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        },
                        Some(ty) => {
                            self.error_backtrace();
                            panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty)
                        },
                        None => match pointee {
                            None => Some(parent_ptr),  // we can't determine if the parent type matches, but we weren't given a backup, so we assume the types match (somewhat dangerous)
                            _ => {
                                self.error_backtrace();
                                panic!("CompleteAbstractData specifies public-pointer-to-parent-or, but since we don't know the current LLVM type, we can't determine whether the parent type matches or not")
                            },
                        },
                    },
                };
                match (parent_ptr, pointee) {
                    (Some(parent_ptr), _) => {
                        // no need for the backup, since the parent type matches
                        debug!("setting the memory contents equal to {:?}", parent_ptr);
                        let bits = parent_ptr.get_width();
                        ctx.state.write(&addr, parent_ptr.clone())?;
                        Ok(bits as usize)
                    },
                    (None, Some(pointee)) => {
                        // parent type mismatches, or parent doesn't exist: use the backup
                        debug!("memory contents are marked as public-pointer-to-parent with a backup; since {}, using the backup", match self.parent {
                            Some(_) => "parent type doesn't match",
                            None => "there is no immediate parent",
                        });
                        self.initialize_cad_in_memory(ctx, addr, &CompleteAbstractData::pub_pointer_to((**pointee).to_owned()), ty)
                    },
                    (None, None) => {
                        // parent type mismatches, but we have no backup
                        self.error_backtrace();
                        match self.parent {
                            None => panic!("CompleteAbstractData specifies pointer-to-parent (with no backup), but there is no immediate parent"),
                            Some((_, parent_ty)) => match ty {
                                Some(Type::PointerType { pointee_type, .. }) =>
                                    panic!("Type mismatch: CompleteAbstractData specifies pointer-to-parent (with no backup), but found pointer to a different type.\n  Parent type: {:?}\n  Found type: {:?}\n", parent_ty, &**pointee_type),
                                _ => panic!("we should have already panicked above in this case"),
                            },
                        }
                    }
                }
            },
            CompleteAbstractData::Array { element_type: element_abstractdata, num_elements } => {
                let element_type = ty.map(|ty| match ty {
                    Type::ArrayType { element_type, num_elements: found_num_elements } => {
                        if *found_num_elements != 0 {
                            if num_elements != found_num_elements {
                                self.error_backtrace();
                                panic!( "Type mismatch: CompleteAbstractData specifies an array with {} elements, but found an array with {} elements", num_elements, found_num_elements);
                            }
                        } else {
                            // do nothing.  If it is a 0-element array in LLVM, that probably just means an array of unspecified length, so we don't compare with the CompleteAbstractData length
                        }
                        element_type
                    },
                    _ => ty,  // an array, but the LLVM type is just pointer.  E.g., *int instead of *{array of 16 ints}.
                });
                let element_size_bits = element_abstractdata.size_in_bits();
                if let Some(element_type) = element_type {
                    let element_type: Option<Arc<RwLock<Type>>> = match element_type {
                        Type::NamedStructType { ty: None, name: llvm_struct_name } => {
                            // This is an opaque struct definition. Try to find a non-opaque definition for the same struct.
                            let (ty, _) = ctx.proj.get_named_struct_type_by_name(&llvm_struct_name).unwrap_or_else(|| panic!("Struct name {:?} not found in the project", llvm_struct_name));
                            ty.clone()
                        },
                        _ => Some(Arc::new(RwLock::new(element_type.clone()))),
                    };
                    match element_type {
                        None => {},  // element_type is an opaque struct type, there's no size check we can make.
                        Some(element_type) => {
                           match layout::size_opaque_aware(&element_type.read().unwrap(), ctx.proj) {
                                Some(llvm_element_size_bits) if llvm_element_size_bits != 0 => {
                                    if element_size_bits != llvm_element_size_bits {
                                        self.error_backtrace();
                                        panic!( "CompleteAbstractData element size of {} bits does not match LLVM element size of {} bits", element_size_bits, llvm_element_size_bits);
                                    }
                                },
                                _ => {},  // skip performing the check: llvm element size is either 0 bits or can't be determined
                            }
                        }
                    }
                }
                match **element_abstractdata {
                    CompleteAbstractData::Secret { .. } => {
                        // special-case this, as we can initialize with one big write
                        let array_size_bits = element_size_bits * *num_elements;
                        debug!("initializing the entire array as {} secret bits", array_size_bits);
                        self.initialize_cad_in_memory(ctx, &addr, &CompleteAbstractData::sec_integer(array_size_bits), ty)
                    },
                    CompleteAbstractData::PublicValue { bits, value: AbstractValue::Unconstrained } => {
                        // special-case this, as no initialization is necessary for the entire array
                        debug!("array contents are entirely public unconstrained bits");
                        Ok(bits * *num_elements)
                    },
                    _ => {
                        // the general case. This would work in all cases, but would be slower than the optimized special-case above
                        if element_size_bits % 8 != 0 {
                            self.error_backtrace();
                            panic!("Array element size is not a multiple of 8 bits: {}", element_size_bits);
                        }
                        let element_size_bytes = element_size_bits / 8;
                        if *num_elements == 0 {
                            // it might seem like we could just do nothing here, but
                            // actually there's no way to return the correct value
                            // (Boolector doesn't support 0-width BVs), so we have to panic
                            self.error_backtrace();
                            panic!("Array with 0 elements (and element type {:?})", element_type);
                        }
                        for i in 0 .. *num_elements {
                            debug!("initializing element {} of the array", i);
                            let element_addr = addr.add(&ctx.state.bv_from_u64((i*element_size_bytes) as u64, addr.get_width()));
                            self.clone().initialize_cad_in_memory(ctx, &element_addr, element_abstractdata, element_type)?;
                        }
                        debug!("done initializing the array at {:?}", addr);
                        Ok(element_size_bits * *num_elements)
                    },
                }
            },
            CompleteAbstractData::Struct { name, elements } => {
                let mut cur_addr = addr.clone();
                let element_types = match ty {
                    Some(ty) => match ty {
                        Type::StructType { element_types, .. } => element_types.iter().cloned().map(Some).collect::<Vec<_>>(),
                        Type::NamedStructType { .. } => {
                            match ctx.proj.get_inner_struct_type_from_named(ty) {
                                Some(arc) => {
                                    let actual_ty: &Type = &arc.read().unwrap();
                                    match actual_ty {
                                        Type::StructType { element_types, .. } => element_types.iter().cloned().map(Some).collect::<Vec<_>>(),
                                        ty => {
                                            self.error_backtrace();
                                            panic!("NamedStructType referred to type {:?} which is not a StructType variant", ty)
                                        },
                                    }
                                },
                                None => {
                                    // This struct has only opaque definitions in the Project.
                                    // We also assume it isn't in the StructDescriptions, since that would have been caught in to_complete().
                                    // Just treat this as if `ty` was `None`
                                    itertools::repeat_n(None, elements.len()).collect()
                                },
                            }
                        },
                        _ => {
                            self.error_backtrace();
                            panic!("Type mismatch: CompleteAbstractData specifies a struct named {}, but found type {:?}", name, ty)
                        },
                    },
                    None => itertools::repeat_n(None, elements.len()).collect(),
                };
                if elements.len() != element_types.len() {
                    self.error_backtrace();
                    panic!("Have {} struct elements but {} element types", elements.len(), element_types.len());
                }
                self.within_structs.push(WithinStruct { name: name.clone(), element_index: 0 });
                self.parent = self.cur_struct;
                self.cur_struct = ty.map(|ty| (addr, ty));
                let mut total_bits = 0;
                for (element_idx, (element, element_ty)) in elements.iter().zip(element_types).enumerate() {
                    let within_structs_len = self.within_structs.len();
                    self.within_structs.get_mut(within_structs_len - 1).unwrap().element_index = element_idx;
                    let element_size_bits = element.size_in_bits();
                    if let Some(element_ty) = &element_ty {
                        match layout::size_opaque_aware(&element_ty, ctx.proj) {
                            Some(llvm_element_size_bits) if llvm_element_size_bits != 0 => {
                                if element_size_bits != llvm_element_size_bits {
                                    self.error_backtrace();
                                    panic!("CompleteAbstractData element size of {} bits does not match LLVM element size of {} bits", element_size_bits, llvm_element_size_bits);
                                }
                            },
                            _ => {},  // skip performing the check: llvm element size is either 0 bits or can't be determined
                        }
                    }
                    if element_size_bits % 8 != 0 {
                        self.error_backtrace();
                        panic!("Struct element size is not a multiple of 8 bits: {}", element_size_bits);
                    }
                    total_bits += element_size_bits;
                    let element_size_bytes = element_size_bits / 8;
                    debug!("initializing element {} of struct {}; element's address is {:?}", element_idx, name, &cur_addr);
                    let bits = self.clone().initialize_cad_in_memory(ctx, &cur_addr, element, element_ty.as_ref())?;
                    if bits != element_size_bits {
                        self.error_backtrace();
                        panic!("Element {} of struct {} should be {} bits based on its type, but we seem to have initialized {} bits", element_idx, name, element_size_bits, bits);
                    }
                    cur_addr = cur_addr.add(&ctx.state.bv_from_u64(element_size_bytes as u64, addr.get_width()));
                }
                debug!("done initializing struct {} at {:?}", name, addr);
                Ok(total_bits)
            }
            CompleteAbstractData::VoidOverride { llvm_struct_name, data } => {
                // first check that the type we're overriding is `i8`: LLVM seems to use `i8*` when C uses `void*`
                match ty {
                    Some(Type::IntegerType { bits: 8 }) => {},
                    Some(Type::PointerType { .. }) => {
                        self.error_backtrace();
                        panic!("attempt to use VoidOverride to override LLVM type {:?} rather than i8. You may want to use a pointer to a VoidOverride rather than a VoidOverride directly.", ty)
                    },
                    Some(ty) => {
                        self.error_backtrace();
                        panic!("attempt to use VoidOverride to override LLVM type {:?} rather than i8", ty)
                    },
                    None => {},  // could be a nested VoidOverride, for instance
                }
                match llvm_struct_name {
                    None => self.initialize_cad_in_memory(ctx, addr, &data, None),
                    Some(llvm_struct_name) => {
                        let (llvm_ty, _) = ctx.proj.get_named_struct_type_by_name(&llvm_struct_name)
                            .unwrap_or_else(|| { self.error_backtrace(); panic!("VoidOverride: llvm_struct_name {:?} not found in Project", llvm_struct_name) });
                        let arc = llvm_ty.as_ref().unwrap_or_else(|| { self.error_backtrace(); panic!("VoidOverride: llvm_struct_name {:?} is an opaque type", llvm_struct_name) });
                        let llvm_ty: &Type = &arc.read().unwrap();
                        self.initialize_cad_in_memory(ctx, addr, &data, Some(llvm_ty))
                    },
                }
            },
            CompleteAbstractData::PointerOverride { llvm_struct_name, data } => {
                // first check that we're overriding a pointer type
                if let Some(ty) = ty {
                    match ty {
                        Type::PointerType { .. } => {},
                        _ => {
                            self.error_backtrace();
                            panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty)
                        },
                    };
                }

                // allocate memory for the pointee, which is `data` (ignoring LLVM type)
                let inner_ptr = ctx.state.allocate(data.size_in_bits() as u64);
                debug!("allocated memory for the pointee at {:?}, and will constrain the memory contents at {:?} to have that pointer value", inner_ptr, addr);

                // make `addr` point to a pointer to the newly allocated memory
                let bits = inner_ptr.get_width();
                ctx.state.write(&addr, inner_ptr.clone())?;

                // initialize the pointee
                match llvm_struct_name {
                    None => {
                        self.initialize_cad_in_memory(ctx, &inner_ptr, data, None)?;
                    },
                    Some(llvm_struct_name) => {
                        let (llvm_ty, _) = ctx.proj.get_named_struct_type_by_name(&llvm_struct_name)
                            .unwrap_or_else(|| { self.error_backtrace(); panic!("PointerOverride: llvm_struct_name {:?} not found in Project", llvm_struct_name) });
                        let arc = llvm_ty.as_ref().unwrap_or_else(|| { self.error_backtrace(); panic!("PointerOverride: llvm_struct_name {:?} is an opaque type", llvm_struct_name) });
                        let llvm_ty: &Type = &arc.read().unwrap();
                        self.initialize_cad_in_memory(ctx, &inner_ptr, data, Some(llvm_ty))?;
                    },
                }

                Ok(bits as usize)
            },
            CompleteAbstractData::SameSizeOverride { data } => {
                // first check that the type we're overriding is the right size
                match ty {
                    Some(ty) => match layout::size_opaque_aware(ty, ctx.proj) {
                        Some(ty_size) => {
                            assert_eq!(data.size_in_bits(), ty_size, "same_size_override: size mismatch: specified something of size {} bits, but the LLVM type has size {} bits", data.size_in_bits(), ty_size);
                        },
                        None => {},  // can't determine the size, so skip performing the check
                    },
                    None => {},
                };
                self.initialize_cad_in_memory(ctx, addr, &**data, None)
            }
            CompleteAbstractData::WithWatchpoint { name, data } => {
                let watch_addr = addr.as_u64().expect("WithWatchpoint not compatible with a non-constant initialization address");
                let watch_size_in_bytes = data.size_in_bits() / 8;
                ctx.state.add_mem_watchpoint(name, Watchpoint::new(watch_addr, watch_size_in_bytes as u64));
                self.initialize_cad_in_memory(ctx, addr, &**data, ty)
            }
        }
    }

}
