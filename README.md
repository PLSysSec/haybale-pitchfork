# `pitchfork`: Verifying constant-time code with symbolic execution

`pitchfork` is a tool for verifying that constant-time code is, indeed,
constant-time.
It can analyze code written in C/C++, Rust, or any other language which can
compile to LLVM bitcode (e.g., Swift, Go, and others).
Given a function to analyze, `pitchfork` will either report that the function
is constant-time, or give a detailed explanation of why it is not, including
the line number of the constant-time violation in the original source, full
sequence of branch decisions leading to the violation, and values of all
variables at the point of the violation.

`pitchfork` is built on the [`haybale`] symbolic execution engine, which is
also written in Rust.

## What is constant-time, and what are constant-time violations?

Constant-time programming is the de-facto technique for hardening
cryptographic implementations against timing side-channel attacks; it has
been adopted by almost all major cryptographic libraries. Constant-time code
obeys two fundamental principles:

1. Secret values must not influence control flow (e.g., branch conditions or jump targets), and
2. Secret values must not influence the addresses of memory accesses (e.g., array indexes).

These principles ensure that a program's timing characteristics and memory
access patterns are completely independent of the secret values which it
operates on.
Thus, even powerful timing side-channel attacks - and in particular, _cache
attacks_, which glean information from timing attacks on the processor
cache - are unable to recover any information about these secret values.

## Getting started

### 1. Install

`pitchfork` is on [crates.io](https://crates.io/crates/haybale-pitchfork), under
the name `haybale-pitchfork`. You can add it as a dependency in your
`Cargo.toml`, selecting the feature corresponding to the LLVM version you want:

```toml
[dependencies]
haybale-pitchfork = { version = "0.3.0", features = ["llvm-10"] }
```

Currently, the supported LLVM versions are `llvm-9` and `llvm-10`.

If you want to use the name `pitchfork` instead of `haybale_pitchfork` in
your code, you can use Cargo's [dependency renaming] feature:

```toml
[dependencies]
pitchfork = { package = "haybale-pitchfork", version = "0.3.0", features = ["llvm-10"] }
```

Because it is built on [`haybale`], `pitchfork` also depends (indirectly) on
the LLVM and Boolector libraries, which must both be available on your
system.
See the [`llvm-sys`] or [`boolector-sys`] READMEs for more details and
instructions.

### 2. Acquire bitcode to analyze

Since `pitchfork` operates on LLVM bitcode, you'll need some bitcode to get
started.
If the program or function you want to analyze is written in C, you can
generate LLVM bitcode (`*.bc` files) with `clang`'s `-c` and `-emit-llvm`
flags:

```bash
clang -c -emit-llvm source.c -o source.bc
```

For debugging purposes, you may also want LLVM text-format (`*.ll`) files,
which you can generate with `clang`'s `-S` and `-emit-llvm` flags:

```bash
clang -S -emit-llvm source.c -o source.ll
```

If the program or function you want to analyze is written in Rust, you can
likewise use `rustc`'s `--emit=llvm-bc` and `--emit=llvm-ir` flags.

Note that in order for `pitchfork` to print source-location information
(e.g., source filename and line number) for constant-time violations and
other errors, the LLVM bitcode will need to include debuginfo.
You can ensure debuginfo is included by passing the `-g` flag to `clang`,
`clang++`, or `rustc` when generating bitcode.

### 3. Create a Project

A `Project` contains all of the code currently being analyzed, which may be
one or more LLVM modules.
To get started, simply create a `Project` from a single bitcode file:

```rust
let project = Project::from_bc_path(&Path::new("/path/to/file.bc"))?;
```

For more ways to create `Project`s, including analyzing entire libraries, see
the [`Project` documentation].

### 4. Check a function for constant-time violations

Let's suppose we want to check for constant-time violations in the following
C function:

```c
int foo(int x) {
    if (x > 10) {
        return x % 200 * 3;
    } else {
        return x + 10;
    }
}
```

We can use [`check_for_ct_violation_in_inputs()`] to analyze this function,
considering all of its inputs (in this case just `x`) to be secret:

```rust
let result = check_for_ct_violation_in_inputs("foo", &project, Config::default(), &PitchforkConfig::default());
```

and then pretty-print the result of the analysis:

```rust
println!("{}", result);
```

Since `x` influences a branch condition, `pitchfork` reports a constant-time
violation.

## User Guide: Analyzing Functions

In the "Getting Started" example above, we used
[`check_for_ct_violation_in_inputs()`], which considers all inputs to the
function to be secret.
However, sometimes some inputs are public and others secret.
Or sometimes we have arguments which are pointers to secret data, or the
secret data is hidden deep inside a struct.

The more general function [`check_for_ct_violation()`] takes two additional
arguments which aid in annotating data as public or secret: `args` and `sd`.
In this section we'll walk through analyzing some more examples of functions,
showing more ways to annotate public and secret data.

### Specifying some function arguments as public

Consider this C function:

```c
int ct(int x, int y, int option) {
    volatile int z[3] = { 0, 2, 300 };
    z[2] = y;
    if (option > 3) {
        return z[1];
    } else {
        return z[2];
    }
}
```

If we analyze this function using [`check_for_ct_violation_in_inputs()`],
we'll see a constant-time violation, since the `option` argument is used in a
branch condition:

```rust
println!("{}", check_for_ct_violation_in_inputs("ct", &project, Config::default(), &PitchforkConfig::default()));
```

However, let's suppose the `option` argument to this function actually
just denotes some configuration option and shouldn't be considered secret.
We can communicate this to `pitchfork` by using the `args` argument to
the more general function [`check_for_ct_violation()`].

Each element of the `args` iterator describes the corresponding argument to
`ct()`.
The elements of `args` are of type [`AbstractData`]; there's a lot of
different options for how we could describe the argument, but let's focus on
the two most basic ones:

- `AbstractData::default()` is the most important option. It uses the
information in the `StructDescriptions` and the type information in the
LLVM bitcode to automatically generate an appropriate description for the
argument. We'll discuss `StructDescriptions` in more detail later; for now,
all that's important is that unless the `StructDescriptions` say differently,
anything marked `default()` will be considered _public_.
- `AbstractData::secret()` is the easiest way to mark an argument secret.
It uses the type information in the LLVM bitcode to automatically generate
an appropriate description for the argument based on its size, this time
marking it secret. Note that this may not work how you want if the argument
is a pointer; see the section on pointer arguments below.

For the function `ct()` above, we can analyze it using the following argument
descriptions:

```rust
println!("{}", check_for_ct_violation(
    "ct",
    &project,
    Some(vec![AbstractData::secret(), AbstractData::secret(), AbstractData::default()]),
    &StructDescriptions::new(),
    Config::default(),
    &PitchforkConfig::default(),
));
```

This time, `pitchfork` should report that the function is constant-time.

### Specifying particular values for some function arguments

Let's consider the following slight variant of the `ct()` function above:

```c
int ct(int x, int y, int option) {
    volatile int z[3] = { 0, 2, 300 };
    z[2] = y;
    if (option > 3) {
        return z[1];
    } else {
        return z[x % 3];
    }
}
```

If we analyze this function with the annotations given above, `pitchfork` will
(correctly) report a constant-time violation, because the `z[x % 3]` uses an
array index which depends on secret data.

But, let's suppose we're only interested in analyzing the function in the case
where the value of `option` is `5`.
We can express that using `AbstractData` like this:

```rust
println!("{}", check_for_ct_violation(
    "ct",
    &project,
    Some(vec![
        AbstractData::secret(),
        AbstractData::secret(),
        AbstractData::pub_i32(AbstractValue::ExactValue(5)),
    ]),
    &StructDescriptions::new(),
    Config::default(),
    &PitchforkConfig::default(),
));
```

Here we used `AbstractData::pub_i32()` to specify a public 32-bit integer,
and `AbstractValue::ExactValue(5)` to give that integer the exact value `5`.
This means that the analysis will only consider the case when the value of
that argument is `5`, and as a result, will report that there are no
constant-time violations in the function.

`ExactValue` is also useful for when a function argument specifies something
like an input array length, and you only want to consider a particular array
length in the analysis.
See the docs on [`AbstractValue`] for more ways to specify argument values.

As a side note, notice that you can't specify the value of something marked
secret.
This is because the value of anything marked secret doesn't matter for the
symbolic analysis.
In fact, if the value of something marked secret would matter for the
analysis (e.g., for determining which paths were valid, or which address in
memory was loaded), that would already be a constant-time violation, as it
would mean a secret value had influenced a branch condition or memory
address.

### Pointers to arrays of secret data

Let's suppose we have a C function which takes a pointer to an array of 32
secret integers, and adds `1` to each element of the array:

```c
uint32_t secret_array(uint32_t* arr) {
    for (int i = 0; i < 32; i++) {
        arr[i] += 1;
    }
    return arr[0];
}
```

Using `AbstractData::secret()` for `arr` might not work how you expect:

```rust
println!("{}", check_for_ct_violation(
    "secret_array",
    &project,
    Some(vec![AbstractData::secret()]),
    &StructDescriptions::new(),
    Config::default(),
    &PitchforkConfig::default(),
));
```

This function looks fine, but `pitchfork` will report a constant-time violation!
This is because it applies `secret` to the argument itself, which in this
case is a pointer.
Since the pointer value `arr` is secret, the access `arr[i]` results in an
address which depends on the secret value of the pointer `arr`.

Instead, we can tell `pitchfork` that `arr` itself (the pointer) is public, but
points to an array of secret data:
```rust
AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::secret(), 32))
```

We'll also need to raise the "loop bound", which defaults to 10 as of this
writing. (For more details, see the [docs on
`Config.loop_bound`](https://PLSysSec.github.io/haybale-pitchfork/haybale_pitchfork/struct.Config.html#structfield.loop_bound).)
```rust
let mut config = Config::default();
config.loop_bound = 100;
// then pass this `config` to `check_for_ct_violation()`
```

Now, `pitchfork` should verify that the function is constant-time.

### More constraints regarding arrays

The following C function takes pointers to two arrays: `public_arr` which
contains public values and has length `public_arr_len`, and `secret_arr`
which contains secret values and has length 32.
Let's suppose that `public_arr_len` and `i` are also public - the only secret
values are the values in `secret_arr`.

```c
uint32_t secret_array_var_length(uint32_t* public_arr, uint32_t public_arr_len, uint32_t* secret_arr, uint32_t i) {
    uint32_t x = public_arr[i];
    for (int j = 0; j < 32; j++) {
        secret_arr[j] += x;
    }
    if (x > 10) {
        return public_arr[0] + secret_arr[0];
    } else {
        return public_arr[1] + secret_arr[1];
    }
}
```

If we check this function for constant-time violations as-is, with these
annotations...

```rust
println!("{}", check_for_ct_violation(
    "secret_array_var_length",
    &project,
    Some(vec![
        AbstractData::default(),
        AbstractData::default(),
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::secret(), 32)),
        AbstractData::default(),
    ]),
    &StructDescriptions::new(),
    config,  // with loop_bound set to 100 as above
    &PitchforkConfig::default(),
));
```

...we'll find that `pitchfork` reports a constant-time violation.
Specifically, if `i` is too large, `public_arr[i]` could actually load secret
data from `secret_arr` (off the end of `public_arr`), which is then used in a
branch condition.

In this case, the function is actually safe as long as callers pass an `i`
which is in-bounds.
To analyze only the case where `i` is in-bounds, we can specify a maximum
length for `public_arr` (in this example, let's say 72 elements), and
constrain both `public_arr_len` and `i` to be within this maximum length:

```rust
println!("{}", check_for_ct_violation(
    "secret_array_var_length",
    &project,
    Some(vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::default(), 72)),
        AbstractData::pub_i32(AbstractValue::Range(0, 72)),
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::secret(), 32)),
        AbstractData::pub_i32(AbstractValue::Range(0, 71)),
    ]),
    &StructDescriptions::new(),
    config,  // with loop_bound set to 100 as above
    &PitchforkConfig::default(),
));
```

With these annotations, `pitchfork` will verify that the function is
constant-time, given these assumptions.

Notice two more things about this function: first, we can choose a large maximum
length for `public_arr` without knowing what its actual length will be.
Assuming a large size for arrays doesn't really harm anything: `pitchfork`
will "allocate" extra memory for the array, but that memory will go unused
and (probably) not affect the analysis.
Second, the above analysis will still consider inputs where `i > public_arr_len`;
it just happens that the function above is still constant-time in these cases,
since we made the assumption that `i` is within the actual allocated size of
`public_arr` (which is 72, despite `public_arr_len` being potentially smaller).
If we wanted to consider only those cases where additionally `i < public_arr_len`,
we could use this for `public_arr_len`:
```rust
AbstractData::pub_i32(AbstractValue::named("public_arr_len", AbstractValue::Range(0, 72)))
```
and this for `i`:
```rust
AbstractData::pub_i32(AbstractValue::UnsignedLessThan("public_arr_len".into()))
```

You might wonder what `pitchfork` assumed about `public_arr` in the first
place when we used `AbstractData::default()`. The answer is that it assumed
an array of length `AbstractData::DEFAULT_ARRAY_LENGTH` (currently 1024 as of
this writing).
For more details on the precise behavior of `default()`, see the docs on
[`AbstractData::default()`].

### Structs containing secret data

Let's consider this C function:

```c
uint32_t uses_a_struct(Context* ctx, uint32_t* public_input, uint32_t* public_output) {
    for (int i = 0; i < 32; i++) {
        public_output[i] = public_input[i] ^ ctx->secret_key;
    }
    if (ctx->public_option) {
        return public_output[0];
    } else {
        return public_output[1];
    }
}
```
where the `Context` struct is defined as
```c
typedef struct {
    uint32_t public_option;
    uint32_t another_thing;
    uint32_t secret_key;
} Context;
```

We want to specify that the `ctx` argument points to a struct with some
secret data (`secret_key`) and some public data (`public_option`).
We can do this using `AbstractData::_struct`:
```rust
println!("{}", check_for_ct_violation(
    "uses_a_struct",
    &project,
    Some(vec![
        AbstractData::pub_pointer_to(AbstractData::_struct("Context", vec![
            AbstractData::default(),  // public_option
            AbstractData::default(),  // another_thing
            AbstractData::secret(),   // secret_key
        ])),
        AbstractData::default(),  // public_input
        AbstractData::default(),  // public_output
    ]),
    &StructDescriptions::new(),
    config,  // with loop_bound set to 100 as above
    &PitchforkConfig::default(),
));
```

With this description, `pitchfork` should correctly report that the function
is constant-time.

Another way to do the same thing---which is more useful if there are multiple
`Context` structs floating around, or if there are other structs which contain
pointers to `Context` structs, etc---is via the `StructDescriptions`.
We can give `pitchfork` a description of the `Context` struct:
```rust
let sd: StructDescriptions = vec![
    ("struct.Context".into(), AbstractData::_struct("Context", vec![
        AbstractData::default(),  // public_option
        AbstractData::default(),  // another_thing
        AbstractData::secret(),   // secret_key
    ])),
].into_iter().collect();
```
Then, when we use `AbstractData::default()`, it will automatically take into
account the description of the `Context` struct which we give in the
`StructDescriptions` argument:
```rust
println!("{}", check_for_ct_violation(
    "uses_a_struct",
    &project,
    Some(vec![
        AbstractData::default(),  // ctx
        AbstractData::default(),  // public_input
        AbstractData::default(),  // public_output
    ]),
    &sd,
    config,  // with loop_bound set to 100 as above
    &PitchforkConfig::default(),
));
```

Furthermore, any other `Context` structs it encounters as part of the same or
other arguments will also use this description.

[`haybale`]: https://github.com/PLSysSec/haybale
[dependency renaming]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml
[`llvm-sys`]: https://crates.io/crates/llvm-sys
[`boolector-sys`]: https://crates.io/crates/boolector-sys
[`Project` documentation]: https://docs.rs/haybale/0.6.1/haybale/project/struct.Project.html
[`Project`]: https://docs.rs/haybale/0.6.1/haybale/project/struct.Project.html
[`Config`]: https://docs.rs/haybale/0.6.1/haybale/config/struct.Config.html
[`check_for_ct_violation_in_inputs()`]: https://docs.rs/haybale-pitchfork/0.3.0/haybale_pitchfork/fn.check_for_ct_violation_in_inputs.html
[`check_for_ct_violation()`]: https://docs.rs/haybale-pitchfork/0.3.0/haybale_pitchfork/fn.check_for_ct_violation.html
[`AbstractData`]: https://docs.rs/haybale-pitchfork/0.3.0/haybale_pitchfork/struct.AbstractData.html
[`AbstractValue`]: https://docs.rs/haybale-pitchfork/0.3.0/haybale_pitchfork/enum.AbstractValue.html
[`AbstractData::default()`]: https://docs.rs/haybale-pitchfork/0.3.0/haybale_pitchfork/struct.AbstractData.html#method.default
