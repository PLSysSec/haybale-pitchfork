var N=null,E="",T="t",U="u",searchIndex={};
var R=["haybale_pitchfork","Holds information about the results of a constant-time…","haybale_pitchfork::ConstantTimeResultForPath","backend","project","config","constanttimeresultforfunction","abstractvalue","abstractdata","operand","structdescriptions","haybale_pitchfork::secret","haybale_pitchfork::secret::BV","constanttimeresultforpath","result","try_from","try_into","borrow_mut","to_owned","clone_into","type_id","to_string","borrow","typeid","btorref","formatter","option","string","bvsolution","get_solver","duration","Construct a new `Project` from a path to a directory…","ConstantTimeResultForFunction","PathStatistics","AbstractValue","ConstantTimeResultForPath","AbstractData"];

searchIndex["haybale_pitchfork"]={"doc":E,"i":[[3,"Config",R[0],"Various settings which affect how the symbolic execution…",N,N],[12,"loop_bound",E,"Maximum number of times to execute any given line of LLVM…",0,N],[12,"solver_query_timeout",E,"Maximum amount of time to allow for any single solver query.",0,N],[12,"null_detection",E,"If `true`, all memory accesses will be checked to ensure…",0,N],[12,"concretize_memcpy_lengths",E,"When encountering a `memcpy`, `memset`, or `memmove` with…",0,N],[12,"squash_unsats",E,"`Error::Unsat` is an error type which is used internally,…",0,N],[12,"trust_llvm_assumes",E,"When encountering the `llvm.assume()` intrinsic, should we…",0,N],[12,"function_hooks",E,"The set of currently active function hooks; see…",0,N],[12,"initial_mem_watchpoints",E,"The initial memory watchpoints when a `State` is created…",0,N],[12,"demangling",E,"Controls the (attempted) demangling of function names in…",0,N],[12,"print_source_info",E,"If `true`, then `haybale` will attempt to print source…",0,N],[12,"print_module_name",E,"If `true`, then `haybale` will include the module name…",0,N],[3,"Project",E,"A `Project` is a collection of LLVM code to be explored,…",N,N],[3,R[36],E,"An abstract description of a value: its size, whether it…",N,N],[3,R[32],E,R[1],N,N],[12,"funcname",E,"Name of the toplevel function we analyzed",1,N],[12,"path_results",E,"the `ConstantTimeResultForPath`s for each path in that…",1,N],[12,"block_coverage",E,"Map from function names to statistics on the block…",1,N],[3,R[33],E,"Some statistics which can be computed from a…",N,N],[12,"num_ct_paths",E,"How many paths \"passed\", that is, had no error or…",2,N],[12,"num_ct_violations",E,"How many constant-time violations did we find",2,N],[12,"num_unsats",E,"How many Unsat errors did we find",2,N],[12,"num_loop_bound_exceeded",E,"How many LoopBoundExceeded errors did we find",2,N],[12,"num_null_ptr_deref",E,"How many NullPointerDereference errors did we find",2,N],[12,"num_function_not_found",E,"How many FunctionNotFound errors did we find",2,N],[12,"num_solver_errors",E,"How many solver errors (including timeouts) did we find",2,N],[12,"num_unsupported_instruction",E,"How many UnsupportedInstruction errors did we find",2,N],[12,"num_malformed_instruction",E,"How many MalformedInstruction errors did we find",2,N],[12,"num_other_errors",E,"How many other errors (including solver timeouts) did we…",2,N],[4,R[34],E,"A variety of ways to specify a numerical value, from…",N,N],[13,"ExactValue",E,"This exact numerical value",3,N],[13,"Range",E,"Any numerical value in the range (inclusive)",3,N],[13,"Unconstrained",E,"Any value whatsoever",3,N],[13,"Named",E,"A value with a (unique) name, so that it can be referenced…",3,N],[12,"name","haybale_pitchfork::AbstractValue",E,3,N],[12,"value",E,E,3,N],[13,"EqualTo",R[0],"A value equal to the value with the given name",3,N],[13,"SignedLessThan",E,"A value signed-less-than the value with the given name",3,N],[13,"SignedGreaterThan",E,"A value signed-greater-than the value with the given name",3,N],[13,"UnsignedLessThan",E,"A value unsigned-less-than the value with the given name",3,N],[13,"UnsignedGreaterThan",E,"A value unsigned-greater-than the value with the given name",3,N],[4,R[35],E,R[1],N,N],[13,"IsConstantTime",E,E,4,N],[13,"NotConstantTime",E,E,4,N],[12,"violation_message",R[2],"A `String` describing the violation found on this path.",4,N],[13,"OtherError",R[0],E,4,N],[12,"error",R[2],"The `Error` encountered on this path.",4,N],[12,"full_message",E,"The full error message with \"rich context\" (backtrace,…",4,N],[5,"check_for_ct_violation_in_inputs",R[0],"Checks whether a function is \"constant-time\" in its…",N,[[["str"],[R[4]],[R[3]],[R[5],[R[3]]],["bool"]],[R[6]]]],[5,"check_for_ct_violation",E,"Checks whether a function is \"constant-time\" in the…",N,[[["str"],[R[4]],[R[10]],[R[3]],[R[5],[R[3]]],["bool"]],[R[6]]]],[11,"pub_i8",E,"an 8-bit public value",5,[[[R[7]]],["self"]]],[11,"pub_i16",E,"a 16-bit public value",5,[[[R[7]]],["self"]]],[11,"pub_i32",E,"a 32-bit public value",5,[[[R[7]]],["self"]]],[11,"pub_i64",E,"a 64-bit public value",5,[[[R[7]]],["self"]]],[11,"pub_integer",E,"a public value with the given number of bits",5,[[["usize"],[R[7]]],["self"]]],[11,"sec_i8",E,"an 8-bit secret value",5,[[],["self"]]],[11,"sec_i16",E,"a 16-bit secret value",5,[[],["self"]]],[11,"sec_i32",E,"a 32-bit secret value",5,[[],["self"]]],[11,"sec_i64",E,"a 64-bit secret value",5,[[],["self"]]],[11,"sec_integer",E,"a secret value with the given number of bits",5,[[["usize"]],["self"]]],[11,"pub_pointer_to",E,"A (public) pointer to something - another value, an array,…",5,[[],["self"]]],[11,"pub_maybe_null_pointer_to",E,"A (public) pointer which may either point to the given…",5,[[],["self"]]],[11,"pub_pointer_to_func",E,"a (public) pointer to the LLVM `Function` with the given…",5,[[],["self"]]],[11,"pub_pointer_to_hook",E,"a (public) pointer to the hook registered for the given name",5,[[],["self"]]],[11,"pub_pointer_to_self",E,"A (public) pointer to this struct itself. E.g., in the C…",5,[[],["self"]]],[11,"pub_pointer_to_parent",E,"A (public) pointer to this struct's parent. E.g., in the C…",5,[[],["self"]]],[11,"pub_pointer_to_parent_or",E,"Like `pub_pointer_to_parent()`, but if the parent is not…",5,[[],["self"]]],[11,"array_of",E,"A (first-class) array of values",5,[[["usize"]],["self"]]],[11,"_struct",E,"A (first-class) structure of values",5,[[],["self"]]],[11,"default",E,"Just use the default structure based on the LLVM type…",5,[[],["self"]]],[11,"default_for_llvm_struct_name",E,"Use the default structure for the given LLVM struct name.",5,[[],["self"]]],[11,"unconstrained_pointer",E,"A (public) pointer which may point anywhere, including…",5,[[],["self"]]],[11,"unconstrained",E,"Just fill with the appropriate number of unconstrained…",5,[[],["self"]]],[11,"secret",E,"Fill with the appropriate number of secret bytes based on…",5,[[],["self"]]],[11,"void_override",E,"When C code uses `void*`, this often becomes `i8*` in…",5,[[["str"],[R[26],["str"]],[R[8]]],["self"]]],[11,"same_size_override",E,"Use the given `data`, even though it may not match the…",5,[[[R[8]]],["self"]]],[11,"with_watchpoint",E,"Use the given `data`, but also (during initialization) add…",5,[[],["self"]]],[18,"DEFAULT_ARRAY_LENGTH",E,E,5,N],[18,"POINTER_SIZE_BITS",E,E,5,N],[18,"OPAQUE_STRUCT_SIZE_BYTES",E,E,5,N],[11,"named",E,E,3,[[["str"],[R[7]]],["self"]]],[0,"hook_helpers",E,"This module contains helper functions that may be useful…",N,N],[5,"fill_unconstrained_with_length","haybale_pitchfork::hook_helpers","Fills a buffer with unconstrained data, and also outputs…",N,[[["u32"],["state"],[R[9]],["either",[R[9]]],[R[27]]],[R[14]]]],[5,"fill_secret_with_length",E,"Fills a buffer with secret data, and also outputs the…",N,[[["either",[R[9],"bv"]],["u32"],[R[27]],[R[9]],["bv"],["state"]],[R[14]]]],[5,"allocate_and_init_abstractdata",E,"Allocates space for the given `AbstractData`, initializes…",N,[[[R[4]],[R[8]],[R[10]],["state"],["type"]],[["bv"],[R[14],["bv"]]]]],[5,"reinitialize_pointee",E,"Reinitializes whatever is pointed to by the given pointer,…",N,[[[R[4]],[R[8]],[R[9]],["state"],[R[10]]],[R[14]]]],[0,"secret",R[0],"This module contains the dynamic taint-tracking layer…",N,N],[3,"BtorRef",R[11],"This wrapper around `Rc<Btor>` exists simply so we can…",N,N],[3,"Memory",E,"A `Memory` which tracks which of its contents are public…",N,N],[3,"Backend",E,"A `Backend` which performs dynamic taint tracking and…",N,N],[4,"BV",E,"A wrapper around `boolector::BV` which can represent…",N,N],[13,"Public",E,E,6,N],[13,"Secret",E,"`Secret` values are opaque because we don't care about…",6,N],[12,"btor",R[12],E,6,N],[12,"width",E,E,6,N],[12,"symbol",E,E,6,N],[13,"PartiallySecret",R[11],"`PartiallySecret` values have some secret and some…",6,N],[12,"secret_mask",R[12],"A vector the length of the `PartiallySecret` value's…",6,N],[12,"data",E,"A `BV`, which must have bitwidth exactly equal to the…",6,N],[12,"symbol",E,E,6,N],[11,"is_secret",R[11],E,6,[[["self"]],["bool"]]],[11,"as_public",E,"Gets the value out of a `BV::Public`, panicking if it is…",6,[[["self"]],["bv"]]],[6,"StructDescriptions",R[0],"A map from struct name to an `AbstractData` description of…",N,N],[11,"first_ct_violation",E,"Return the `violation_message` for the first…",1,[[["self"]],[["str"],[R[26],["str"]]]]],[11,"first_error_or_violation",E,"Return the first `NotConstantTime` or `OtherError` result…",1,[[["self"]],[[R[13]],[R[26],[R[13]]]]]],[11,"path_statistics",E,E,1,[[["self"]],["pathstatistics"]]],[11,"into",E,E,0,[[],[U]]],[11,"from",E,E,0,[[[T]],[T]]],[11,R[18],E,E,0,[[["self"]],[T]]],[11,R[19],E,E,0,[[["self"],[T]]]],[11,R[15],E,E,0,[[[U]],[R[14]]]],[11,R[16],E,E,0,[[],[R[14]]]],[11,R[22],E,E,0,[[["self"]],[T]]],[11,R[17],E,E,0,[[["self"]],[T]]],[11,R[20],E,E,0,[[["self"]],[R[23]]]],[11,"into",E,E,7,[[],[U]]],[11,"from",E,E,7,[[[T]],[T]]],[11,R[15],E,E,7,[[[U]],[R[14]]]],[11,R[16],E,E,7,[[],[R[14]]]],[11,R[22],E,E,7,[[["self"]],[T]]],[11,R[17],E,E,7,[[["self"]],[T]]],[11,R[20],E,E,7,[[["self"]],[R[23]]]],[11,"into",E,E,5,[[],[U]]],[11,"from",E,E,5,[[[T]],[T]]],[11,R[18],E,E,5,[[["self"]],[T]]],[11,R[19],E,E,5,[[["self"],[T]]]],[11,R[21],E,E,5,[[["self"]],[R[27]]]],[11,R[15],E,E,5,[[[U]],[R[14]]]],[11,R[16],E,E,5,[[],[R[14]]]],[11,R[22],E,E,5,[[["self"]],[T]]],[11,R[17],E,E,5,[[["self"]],[T]]],[11,R[20],E,E,5,[[["self"]],[R[23]]]],[11,"into",E,E,1,[[],[U]]],[11,"from",E,E,1,[[[T]],[T]]],[11,R[21],E,E,1,[[["self"]],[R[27]]]],[11,R[15],E,E,1,[[[U]],[R[14]]]],[11,R[16],E,E,1,[[],[R[14]]]],[11,R[22],E,E,1,[[["self"]],[T]]],[11,R[17],E,E,1,[[["self"]],[T]]],[11,R[20],E,E,1,[[["self"]],[R[23]]]],[11,"into",E,E,2,[[],[U]]],[11,"from",E,E,2,[[[T]],[T]]],[11,R[15],E,E,2,[[[U]],[R[14]]]],[11,R[16],E,E,2,[[],[R[14]]]],[11,R[22],E,E,2,[[["self"]],[T]]],[11,R[17],E,E,2,[[["self"]],[T]]],[11,R[20],E,E,2,[[["self"]],[R[23]]]],[11,"into",E,E,3,[[],[U]]],[11,"from",E,E,3,[[[T]],[T]]],[11,R[18],E,E,3,[[["self"]],[T]]],[11,R[19],E,E,3,[[["self"],[T]]]],[11,R[15],E,E,3,[[[U]],[R[14]]]],[11,R[16],E,E,3,[[],[R[14]]]],[11,R[22],E,E,3,[[["self"]],[T]]],[11,R[17],E,E,3,[[["self"]],[T]]],[11,R[20],E,E,3,[[["self"]],[R[23]]]],[11,"into",E,E,4,[[],[U]]],[11,"from",E,E,4,[[[T]],[T]]],[11,R[15],E,E,4,[[[U]],[R[14]]]],[11,R[16],E,E,4,[[],[R[14]]]],[11,R[22],E,E,4,[[["self"]],[T]]],[11,R[17],E,E,4,[[["self"]],[T]]],[11,R[20],E,E,4,[[["self"]],[R[23]]]],[11,"into",R[11],E,8,[[],[U]]],[11,"from",E,E,8,[[[T]],[T]]],[11,R[18],E,E,8,[[["self"]],[T]]],[11,R[19],E,E,8,[[["self"],[T]]]],[11,R[15],E,E,8,[[[U]],[R[14]]]],[11,R[16],E,E,8,[[],[R[14]]]],[11,R[22],E,E,8,[[["self"]],[T]]],[11,R[17],E,E,8,[[["self"]],[T]]],[11,R[20],E,E,8,[[["self"]],[R[23]]]],[11,"into",E,E,9,[[],[U]]],[11,"from",E,E,9,[[[T]],[T]]],[11,R[18],E,E,9,[[["self"]],[T]]],[11,R[19],E,E,9,[[["self"],[T]]]],[11,R[15],E,E,9,[[[U]],[R[14]]]],[11,R[16],E,E,9,[[],[R[14]]]],[11,R[22],E,E,9,[[["self"]],[T]]],[11,R[17],E,E,9,[[["self"]],[T]]],[11,R[20],E,E,9,[[["self"]],[R[23]]]],[11,"into",E,E,10,[[],[U]]],[11,"from",E,E,10,[[[T]],[T]]],[11,R[18],E,E,10,[[["self"]],[T]]],[11,R[19],E,E,10,[[["self"],[T]]]],[11,R[15],E,E,10,[[[U]],[R[14]]]],[11,R[16],E,E,10,[[],[R[14]]]],[11,R[22],E,E,10,[[["self"]],[T]]],[11,R[17],E,E,10,[[["self"]],[T]]],[11,R[20],E,E,10,[[["self"]],[R[23]]]],[11,"into",E,E,6,[[],[U]]],[11,"from",E,E,6,[[[T]],[T]]],[11,R[18],E,E,6,[[["self"]],[T]]],[11,R[19],E,E,6,[[["self"],[T]]]],[11,R[15],E,E,6,[[[U]],[R[14]]]],[11,R[16],E,E,6,[[],[R[14]]]],[11,R[22],E,E,6,[[["self"]],[T]]],[11,R[17],E,E,6,[[["self"]],[T]]],[11,R[20],E,E,6,[[["self"]],[R[23]]]],[11,"clone",R[0],E,0,[[["self"]],[R[5]]]],[11,"default",E,"Default values for all configuration parameters.",0,[[],[R[5]]]],[11,"as_ref",R[11],E,8,[[["self"]],["btor"]]],[11,"from",E,E,8,[[["btor"],["rc",["btor"]]],[R[24]]]],[11,"clone",R[0],E,5,[[["self"]],[R[8]]]],[11,"clone",E,E,3,[[["self"]],[R[7]]]],[11,"clone",R[11],E,8,[[["self"]],[R[24]]]],[11,"clone",E,E,6,[[["self"]],["bv"]]],[11,"clone",E,E,9,[[["self"]],["memory"]]],[11,"clone",E,E,10,[[["self"]],[R[3]]]],[11,"eq",R[0],E,5,[[["self"],[R[8]]],["bool"]]],[11,"ne",E,E,5,[[["self"],[R[8]]],["bool"]]],[11,"eq",E,E,3,[[[R[7]],["self"]],["bool"]]],[11,"ne",E,E,3,[[[R[7]],["self"]],["bool"]]],[11,"eq",R[11],E,8,[[["self"],[R[24]]],["bool"]]],[11,"ne",E,E,8,[[["self"],[R[24]]],["bool"]]],[11,"eq",E,E,6,[[["bv"],["self"]],["bool"]]],[11,"ne",E,E,6,[[["bv"],["self"]],["bool"]]],[11,"eq",E,E,9,[[["self"],["memory"]],["bool"]]],[11,"ne",E,E,9,[[["self"],["memory"]],["bool"]]],[11,"fmt",R[0],E,5,[[["self"],[R[25]]],[R[14]]]],[11,"fmt",E,E,1,[[["self"],[R[25]]],[R[14]]]],[11,"fmt",E,E,5,[[["self"],[R[25]]],[R[14]]]],[11,"fmt",E,E,3,[[["self"],[R[25]]],[R[14]]]],[11,"fmt",R[11],E,8,[[["self"],[R[25]]],[R[14]]]],[11,"fmt",E,E,6,[[["self"],[R[25]]],[R[14]]]],[11,"fmt",E,E,9,[[["self"],[R[25]]],[R[14]]]],[11,"fmt",E,E,10,[[["self"],[R[25]]],[R[14]]]],[11,"deref",E,E,8,[[["self"]],["btor"]]],[11,"new",E,E,8,[[],["self"]]],[11,"duplicate",E,E,8,[[["self"]],["self"]]],[11,"match_bv",E,E,8,[[["bv"],["self"]],[["bv"],[R[26],["bv"]]]]],[11,"match_array",E,E,8,[[["self"],["array"]],[[R[26],["array"]],["array",["rc"]]]]],[11,"new",E,E,6,[[["str"],[R[24]],["u32"],[R[26],["str"]]],["self"]]],[11,"from_bool",E,E,6,[[[R[24]],["bool"]],["self"]]],[11,"from_i32",E,E,6,[[[R[24]],["u32"],["i32"]],["self"]]],[11,"from_u32",E,E,6,[[[R[24]],["u32"]],["self"]]],[11,"from_i64",E,E,6,[[["i64"],[R[24]],["u32"]],["self"]]],[11,"from_u64",E,E,6,[[[R[24]],["u64"],["u32"]],["self"]]],[11,"zero",E,E,6,[[[R[24]],["u32"]],["self"]]],[11,"one",E,E,6,[[[R[24]],["u32"]],["self"]]],[11,"ones",E,E,6,[[[R[24]],["u32"]],["self"]]],[11,"from_binary_str",E,E,6,[[["str"],[R[24]]],["self"]]],[11,"from_dec_str",E,E,6,[[["str"],[R[24]],["u32"]],["self"]]],[11,"from_hex_str",E,E,6,[[["str"],[R[24]],["u32"]],["self"]]],[11,"as_binary_str",E,E,6,[[["self"]],[[R[26],[R[27]]],[R[27]]]]],[11,"as_u64",E,E,6,[[["self"]],[[R[26],["u64"]],["u64"]]]],[11,"as_bool",E,E,6,[[["self"]],[[R[26],["bool"]],["bool"]]]],[11,"get_a_solution",E,E,6,[[["self"]],[[R[28]],[R[14],[R[28]]]]]],[11,R[29],E,E,6,[[["self"]]]],[11,"get_id",E,E,6,[[["self"]],["i32"]]],[11,"get_width",E,E,6,[[["self"]],["u32"]]],[11,"get_symbol",E,E,6,[[["self"]],[["str"],[R[26],["str"]]]]],[11,"set_symbol",E,E,6,[[["str"],["self"],[R[26],["str"]]]]],[11,"is_const",E,E,6,[[["self"]],["bool"]]],[11,"has_same_width",E,E,6,[[["self"]],["bool"]]],[11,"assert",E,E,6,[[["self"]],[R[14]]]],[11,"is_failed_assumption",E,E,6,[[["self"]],["bool"]]],[11,"_eq",E,E,6,[[["self"]],["self"]]],[11,"_ne",E,E,6,[[["self"]],["self"]]],[11,"add",E,E,6,[[["self"]],["self"]]],[11,"sub",E,E,6,[[["self"]],["self"]]],[11,"mul",E,E,6,[[["self"]],["self"]]],[11,"udiv",E,E,6,[[["self"]],["self"]]],[11,"sdiv",E,E,6,[[["self"]],["self"]]],[11,"urem",E,E,6,[[["self"]],["self"]]],[11,"srem",E,E,6,[[["self"]],["self"]]],[11,"smod",E,E,6,[[["self"]],["self"]]],[11,"inc",E,E,6,[[["self"]],["self"]]],[11,"dec",E,E,6,[[["self"]],["self"]]],[11,"neg",E,E,6,[[["self"]],["self"]]],[11,"uaddo",E,E,6,[[["self"]],["self"]]],[11,"saddo",E,E,6,[[["self"]],["self"]]],[11,"usubo",E,E,6,[[["self"]],["self"]]],[11,"ssubo",E,E,6,[[["self"]],["self"]]],[11,"umulo",E,E,6,[[["self"]],["self"]]],[11,"smulo",E,E,6,[[["self"]],["self"]]],[11,"sdivo",E,E,6,[[["self"]],["self"]]],[11,"not",E,E,6,[[["self"]],["self"]]],[11,"and",E,E,6,[[["self"]],["self"]]],[11,"or",E,E,6,[[["self"]],["self"]]],[11,"xor",E,E,6,[[["self"]],["self"]]],[11,"nand",E,E,6,[[["self"]],["self"]]],[11,"nor",E,E,6,[[["self"]],["self"]]],[11,"xnor",E,E,6,[[["self"]],["self"]]],[11,"sll",E,E,6,[[["self"]],["self"]]],[11,"srl",E,E,6,[[["self"]],["self"]]],[11,"sra",E,E,6,[[["self"]],["self"]]],[11,"rol",E,E,6,[[["self"]],["self"]]],[11,"ror",E,E,6,[[["self"]],["self"]]],[11,"redand",E,E,6,[[["self"]],["self"]]],[11,"redor",E,E,6,[[["self"]],["self"]]],[11,"redxor",E,E,6,[[["self"]],["self"]]],[11,"ugt",E,E,6,[[["self"]],["self"]]],[11,"ugte",E,E,6,[[["self"]],["self"]]],[11,"sgt",E,E,6,[[["self"]],["self"]]],[11,"sgte",E,E,6,[[["self"]],["self"]]],[11,"ult",E,E,6,[[["self"]],["self"]]],[11,"ulte",E,E,6,[[["self"]],["self"]]],[11,"slt",E,E,6,[[["self"]],["self"]]],[11,"slte",E,E,6,[[["self"]],["self"]]],[11,"uadds",E,E,6,[[["self"]],["self"]]],[11,"sadds",E,E,6,[[["self"]],["self"]]],[11,"usubs",E,E,6,[[["self"]],["self"]]],[11,"ssubs",E,E,6,[[["self"]],["self"]]],[11,"zext",E,E,6,[[["u32"],["self"]],["self"]]],[11,"sext",E,E,6,[[["u32"],["self"]],["self"]]],[11,"slice",E,E,6,[[["u32"],["self"]],["self"]]],[11,"concat",E,E,6,[[["self"]],["self"]]],[11,"repeat",E,E,6,[[["u32"],["self"]],["self"]]],[11,"iff",E,E,6,[[["self"]],["self"]]],[11,"implies",E,E,6,[[["self"]],["self"]]],[11,"cond_bv",E,E,6,[[["self"]],["self"]]],[11,"new_uninitialized",E,E,9,[[["str"],[R[24]],[R[26],["str"]],["bool"]],["self"]]],[11,"new_zero_initialized",E,E,9,[[["str"],[R[24]],[R[26],["str"]],["bool"]],["self"]]],[11,"read",E,E,9,[[["u32"],["self"]],[R[14]]]],[11,"write",E,E,9,[[["self"]],[R[14]]]],[11,R[29],E,E,9,[[["self"]],[R[24]]]],[11,"change_solver",E,E,9,[[["self"],[R[24]]]]],[11,"new",R[0],"Creates a new `Config` with the given `loop_bound`,…",0,[[[R[26],[R[30]]],[R[30]],["usize"],["concretize"],["bool"]],[R[5]]]],[11,"from_bc_path",E,"Construct a new `Project` from a path to an LLVM bitcode…",7,[[],[[R[4]],[R[27]],[R[14],[R[4],R[27]]]]]],[11,"from_bc_paths",E,"Construct a new `Project` from multiple LLVM bitcode files",7,[[],[[R[4]],[R[27]],[R[14],[R[4],R[27]]]]]],[11,"from_bc_dir",E,R[31],7,[[["str"]],[["error"],[R[14],[R[4],"error"]],[R[4]]]]],[11,"from_bc_dir_with_blacklist",E,R[31],7,[[["str"]],[["error"],[R[14],[R[4],"error"]],[R[4]]]]],[11,"add_bc_path",E,"Add the code in the given LLVM bitcode file to the `Project`",7,[[["self"]],[[R[14],[R[27]]],[R[27]]]]],[11,"add_bc_dir",E,"Add the code in the given directory to the `Project`. See…",7,[[["str"],["self"]],[["error"],[R[14],["error"]]]]],[11,"add_bc_dir_with_blacklist",E,"Add the code in the given directory, except for…",7,[[["str"],["self"]],[["error"],[R[14],["error"]]]]],[11,"all_functions",E,"Iterate over all `Function`s in the `Project`. Gives pairs…",7,[[["self"]]]],[11,"all_global_vars",E,"Iterate over all `GlobalVariable`s in the `Project`. Gives…",7,[[["self"]]]],[11,"all_global_aliases",E,"Iterate over all `GlobalAlias`es in the `Project`. Gives…",7,[[["self"]]]],[11,"all_named_struct_types",E,"Iterate over all named struct types in the `Project`.…",7,[[["self"]]]],[11,"active_module_names",E,"Get the names of the LLVM modules which have been parsed…",7,[[["self"]]]],[11,"get_func_by_name",E,"Search the project for a function with the given name. If…",7,[[["str"],["self"]],[R[26]]]],[11,"get_named_struct_type_by_name",E,"Search the project for a named struct type with the given…",7,[[["str"],["self"]],[R[26]]]],[11,"get_inner_struct_type_from_named",E,"Given a `NamedStructType`, get the `StructType`…",7,[[["type"],["self"]],[[R[26],["arc"]],["arc",["rwlock"]]]]]],"p":[[3,"Config"],[3,R[32]],[3,R[33]],[4,R[34]],[4,R[35]],[3,R[36]],[4,"BV"],[3,"Project"],[3,"BtorRef"],[3,"Memory"],[3,"Backend"]]};
initSearch(searchIndex);addSearchOptions(searchIndex);