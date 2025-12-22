# bn-bdash-extras

[![Documentation](https://img.shields.io/badge/docs-passing-green?style=flat)](https://bn-bdash-extras.bdash.net.nz/)

An assortment of helpers that I've found to be useful when writing Binary Ninja plug-ins in Rust.

## Supported Binary Ninja versions

The development branch is compatible with Binary Ninja's 5.3-dev releases.
Support for stable Binary Ninja versions is available on other branches. 

## Activity configuration

Upstreamed! See the [`activity`](https://dev-rust.binary.ninja/binaryninja/workflow/activity/index.html) module.

## LLIL instruction matching

Types and macros to simplify matching over
[`LowLevelILInstruction`](https://dev-rust.binary.ninja/binaryninja/low_level_il/instruction/struct.LowLevelILInstruction.html)
and [`LowLevelILExpression`](https://dev-rust.binary.ninja/binaryninja/low_level_il/expression/struct.LowLevelILExpression.html)
can be found in the [`llil`](https://bn-bdash-extras.bdash.net.nz/bn_bdash_extras/llil/index.html) module.

```rust
match_instr!{
    instr,
    // Basic patterns
    CallSsa(ConstPtr(address), _) => println!("Direct call to {:#x}", address),
     
    // Variable bindings and guards
    instr @ SetRegSsa(dest, add @ Add(RegSsa(src), Const(value))) if value > 10 => {
        println!(
            "Increment of {src:?} by {value} > 10 at {:#x} (dest={dest:?}, add={add:?})",
            instr.address(),
        );
    },
     
    // OR patterns
    CallSsa(_, _) | TailCallSsa(_, _) => println!("Function call"),
     
    _ => {}
};
```

