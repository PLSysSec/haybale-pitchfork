initSidebarItems({"enum":[["AbstractValue",""],["CompleteAbstractData","An abstract description of a value: if it is public or not, if it is a pointer or not, does it point to data that is public/secret, maybe it's a struct with some public and some secret fields, etc."],["ConstantTimeResultForPath",""]],"fn":[["check_for_ct_violation","Checks whether a function is \"constant-time\" in the secrets identified by the `args` data structure. That is, does the function ever make branching decisions, or perform address calculations, based on secrets."],["check_for_ct_violation_in_inputs","Checks whether a function is \"constant-time\" in its inputs. That is, does the function ever make branching decisions, or perform address calculations, based on its inputs."]],"mod":[["allocation",""],["hook_helpers","This module contains helper functions that may be useful in writing function hooks."],["secret","The `BV`, `Memory`, and `Backend` in this module are intended to be used qualified whenever there is a chance of confusing them with `haybale::backend::{BV, Memory, Backend}`, `haybale::memory::Memory`, or `boolector::BV`."]],"struct":[["AbstractData","An abstract description of a value: if it is public or not, if it is a pointer or not, does it point to data that is public/secret, maybe it's a struct with some public and some secret fields, etc."],["Config","Various settings which affect how the symbolic execution is performed."],["ConstantTimeResultForFunction",""],["PathStatistics",""],["Project","A `Project` is a collection of LLVM code to be explored, consisting of one or more LLVM modules"]],"type":[["StructDescriptions","A map from struct name to an `AbstractData` description of the struct"]]});