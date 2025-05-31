# bn-bdash-extras

[![Documentation](https://img.shields.io/badge/docs-passing-green?style=flat)](https://bn-bdash-extras.bdash.net.nz/)

An assortment of helpers that I've found to be useful when writing Binary Ninja plug-ins in Rust.

## Activity configuration

Type-safe builders for defining the configuration for an [`Activity`](https://dev-rust.binary.ninja/binaryninja/workflow/struct.Activity.html)
can be found in the [`activity`](https://bn-bdash-extras.bdash.net.nz/bn_bdash_extras/activity/index.html) module.

```rust
let workflow = workflow.clone_to(&workflow.name());
let config = Config::action(
    "bdash.arm64e-pac",
    "Remove explicit arm64e PAC checks",
    "Remove explicit arm64e pointer authentication checks prior to tail calls",
)
.with_eligibility(
    Eligibility::auto()
        .with_predicate(ViewType::In(&["Mach-O", "DSCView", "KCView"]))
);
let activity = Activity::new_with_action(&config.to_string(), remove_arm64e_pac);
workflow.register_activity(&activity).unwrap();
```

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

