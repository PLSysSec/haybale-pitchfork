initSidebarItems({"fn":[["allocate_args","Allocate the function parameters given in `params` with their corresponding `AbstractData` descriptions."]],"struct":[["Context","This `Context` serves two purposes: first, simply collecting some objects together so we can pass them around as a unit; but second, allowing some state to persist across invocations of `allocate_arg` (particularly, tracking `AbstractValue::Named` values, thus allowing names used for one arg to reference values defined for another)"],["InitializationContext","As opposed to the `Context`, which contains global-ish state preserved across all allocations (even of different function args), this `InitializationContext` contains more immediate information about where we are and what we're doing."]]});