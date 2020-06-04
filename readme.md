[![Build](https://github.com/SrTobi/forward_goto/workflows/Rust/badge.svg)](https://github.com/SrTobi/forward_goto/actions)
[![Creates.io](https://img.shields.io/crates/v/forward_goto?style)](https://crates.io/crates/forward_goto)
[![Docs](https://docs.rs/forward_goto/badge.svg)](https://docs.rs/forward_goto/)

# forward_goto

This crate provides a sound and safe way to jump from a goto forward
in the control-flow to a label.

```rust
use forward_goto::rewrite_forward_goto;

#[rewrite_forward_goto]
fn decode_msg(luck: impl Fn() -> bool, is_alan_turing: bool) {
    if is_alan_turing {
        forward_goto!('turing_complete);
    }

    println!("You'll need a lot of luck for this");
    
    if luck() {
        println!("Seems you were lucky");

        forward_label!('turing_complete);

        println!("Message decoded!");
    } else {
        println!("No luck today...");
    }
}
```

# Should you use it?

Probably not!