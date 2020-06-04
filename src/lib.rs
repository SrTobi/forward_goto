//! This crate provides a sound and safe way to jump from a goto forward
//! in the control-flow to a label.
//!
//! Rust has no goto mechanism with which it is possible to arbitrarily
//! jump through the control-flow.
//! 
//! This crates provides a procedural macro that allows to write gotos
//! that can jump forward in the control-flow to a specified label.
//! 
//! The jump is safe in terms of the borrow-checker, but several
//! restrictions apply. See [`rewrite_forward_goto`] for more information.
//! 
//! ```
//! use forward_goto::rewrite_forward_goto;
//! 
//! #[rewrite_forward_goto]
//! fn decode_msg(luck: impl Fn() -> bool, is_alan_turing: bool) {
//!     if is_alan_turing {
//!         forward_goto!('turing_complete);
//!     }
//! 
//!     println!("You'll need a lot of luck for this");
//!     
//!     if luck() {
//!         println!("Seems you were lucky");
//! 
//!         forward_label!('turing_complete);
//!
//!         println!("Message decoded!");
//!     } else {
//!         println!("No luck today...");
//!     }
//! }
//! ```

extern crate proc_macro;
extern crate proc_macro2;

mod result;
mod collector;

use collector::Collector;
use quote::{quote, quote_spanned};
use syn::*;
use result::{Result};


/// This macro will rewrite the annotated function so that the control-flow
/// will go from a goto `forward_goto!('label)` directly to a corresponding label
/// `forward_label!('label)`.
///
/// This is achieved by wrapping necessary statements into `loops` and jump to
/// their ends via `break`. Because of this implementation it is only possible to jump
/// forward in the control-flow and not backwards. This, however, implies that
/// all normal rust features — especially the borrow checker — work as expected.
/// 
/// For gotos and labels apply multiple restrictions:
/// 1. Every goto has at most one corresponding label, but multiple gotos can go to
///    the same label.
/// 2. Only forward jumps are allowed, meaning that the goto must come before the label.
///    in the code. Backward jumps are not allowed. 'Side jumps' 
///    (i.e. from a then-branch into an else-branch) are possible,
///    as long as the goto is physically before the label.
/// 3. Any statement after a label in the control-flow may not be the result statement
///    of a block until all current labels are rewired to their corresponding gotos.
/// 
/// ```ignore
/// #[rewrite_forward_goto]
/// fn test() -> i32 {
///     forward_goto!('into_block);
///     
///     let result = {
///         forward_label!('into_block);
///         "the result" // <- error
///     }
/// 
///     42 // would be allowed, because it's outside of 'into_block influence
/// }
/// ```
///
/// Because of they way the rewriting is done, it is only possible to use
/// definitions that are reachable on all code paths.
/// 
/// ```
/// # use forward_goto::rewrite_forward_goto;
/// #[rewrite_forward_goto]
/// fn test(b: bool) {
///     fn f1() {}
/// 
///     if b {
///         forward_goto!('jump);
///     }
/// 
///     fn f2() {}
///     
///     {
///         f2(); // f2 can be used here
///         forward_label!('jump);
///         f1();
///         // f2 cannot be used here
///     };
/// 
///     f1();
/// }
/// ``` 
/// 
#[proc_macro_attribute]
pub fn rewrite_forward_goto(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);

    let mut collector = Collector::new();

    let result = traverse_boxed_block(&mut input.block, &mut collector);
        
    let output = match result.and(collector.check()) {
        Ok(()) => {
            proc_macro::TokenStream::from(quote!(
                #[allow(unreachable_code)]
                #input
            ))
        },
        Err((span, msg)) => {
            let error = quote_spanned!(span=>
                compile_error!(#msg)
            );

            input.block = parse_quote!(
                {
                    #error
                }
            );

            proc_macro::TokenStream::from(quote!(#input))
        },
    };

    //eprintln!("done");
    //eprintln!("{}", &output);
    output
}

fn traverse_boxed_block(boxed: &mut Box<Block>, collector: &mut Collector) -> Result<()> {
    traverse_stmts(&mut boxed.stmts, collector)
}

fn traverse_block(block: &mut Block, collector: &mut Collector) -> Result<()> {
    traverse_stmts(&mut block.stmts, collector)
}

fn traverse_stmts(stmts: &mut Vec<Stmt>, collector: &mut Collector) -> Result<()> {
    let mut i = 0;
    while i < stmts.len() {
        //eprintln!("start stmt");
        {
            let stmt = stmts.get_mut(i).unwrap();
            let mut collector = collector.enter_statement(i);
            traverse_stmt(stmt, &mut collector)?;
        }

        if let Some((start_index, end_label, continuations)) = collector.retrieve_continuations() {
            //eprintln!("build goto {}", i);
            let rest = stmts.split_off(i + 1);

            i = start_index;
            let mut inner = stmts.split_off(start_index);
            inner.push(new_break_stmt(end_label.clone()));

            for (incomings, continuation, outgoing) in continuations {
                inner = {
                    let mut inside_stmts = inner;

                    if let Some(last) = incomings.last().cloned() {
                        for incoming in incomings.into_iter() {
                            inside_stmts.push(new_break_stmt(last.clone()));
                            inside_stmts = vec![new_loop_block(incoming, inside_stmts)];
                        }
                    }

                    inside_stmts.extend(continuation);
                    inside_stmts.push(new_break_stmt(outgoing));
                    inside_stmts
                }
            }
            stmts.push(new_loop_block(end_label, inner));
            stmts.extend(rest);
            //eprintln!("finished build goto {} in {}", i, stmts.len());
            continue;
        }


        if collector.should_push_continuation() {
            let mut continuation = stmts.split_off(i + 1);
            //eprintln!("push continuation {}", continuation.len());
            if let Some(stmt@Stmt::Expr(_)) = continuation.last_mut() {
                //eprintln!("err");
                collector.add_error(stmt, "Result statement is in label continuation and cannot result in a value. Consider adding ';'");
            }
            let target = collector.push_continuation(continuation);
            stmts.push(expr_to_stmt(new_break_expr(target)));
            //eprintln!("pushed continuation");
            return Ok(());
        }
        //eprintln!("end stmt");

        i += 1;
    }

    Ok(())
}

fn traverse_stmt(stmt: &mut Stmt, collector: &mut Collector) -> Result<()> {
    match stmt {
        Stmt::Item(_) => Ok(()),
        Stmt::Local(local) => {
            match local.init {
                Some((_, ref mut expr_box)) => 
                    traverse_boxed_expr(expr_box, collector),
                None => Ok(()),
            }
        },
        Stmt::Expr(expr) => traverse_expr(expr, collector, true),
        Stmt::Semi(expr, _) => traverse_expr(expr, collector, true),
    }
}

fn traverse_boxed_expr(expr: &mut Box<Expr>, collector: &mut Collector) -> Result<()> {
    traverse_expr(expr, collector, false)
}

fn traverse_expr(expr: &mut Expr, collector: &mut Collector, _is_statement: bool) -> Result<()> {
    let replacement_expr = match expr {
        Expr::Macro(mac) => {
            let mac = &mac.mac;
            let path = &mac.path;
            let forward_macro = path.is_ident("forward_goto") || path.is_ident("forward_label");
            if forward_macro {
                let tokens = &mac.tokens;
                let lifetime: Lifetime = parse2(tokens.clone()).unwrap();

                //eprintln!("found macro");
                if path.is_ident("forward_goto") {
                    collector.add_goto(lifetime.clone());
                } else {
                    collector.add_label(lifetime.clone())?;
                }

                Some(new_break_expr(lifetime))
            } else {
                None
            }
        },
        Expr::If(ExprIf { cond, then_branch, else_branch, .. }) => {
            traverse_boxed_expr(cond, &mut collector.cut())?;
            traverse_block(then_branch, &mut collector.enter())?;
            if let Some((_, expr)) = else_branch {
                //eprintln!("traverse else");
                traverse_boxed_expr(expr, &mut collector.enter())?;
            }
            None
        },
        Expr::Match(ExprMatch { expr, arms, .. }) => {
            traverse_boxed_expr(expr, &mut collector.cut())?;
            for arm in arms.iter_mut() {
                traverse_boxed_expr(&mut arm.body, &mut collector.enter())?;
            }
            None
        },
        Expr::Block(ExprBlock { block, ..}) => {
            traverse_block(block, &mut collector.enter())?;
            None
        },
        Expr::Let(ExprLet { expr, .. }) => {
            traverse_boxed_expr(expr, &mut collector.cut())?;
            None
        },
        Expr::Loop(ExprLoop { body, .. }) => {
            traverse_block(body, &mut collector.cut())?;
            None
        },
        _ => None,
    };

    if let Some(replacement) = replacement_expr {
        *expr = replacement;
    }

    Ok(())
}

fn new_break_stmt(lifetime: Lifetime) -> Stmt {
    expr_to_stmt(new_break_expr(lifetime))
}

fn expr_to_stmt(expr: Expr) -> Stmt {
    Stmt::Semi(expr, Token![;](proc_macro2::Span::call_site()))
}

fn new_break_expr(lifetime: Lifetime) -> Expr {
    Expr::Break(ExprBreak {
        attrs: Vec::new(),
        break_token: Token![break](proc_macro2::Span::call_site()),
        label: Some(lifetime),
        expr: None,
    })
}

fn new_loop_block(label: Lifetime, body: Vec<Stmt>) -> Stmt {
    expr_to_stmt(Expr::Loop(ExprLoop {
        attrs: Vec::new(),
        label: Some(Label {
            name: label,
            colon_token: Token![:](proc_macro2::Span::call_site()),
        }),
        loop_token: Token![loop](proc_macro2::Span::call_site()),
        body: Block {
            brace_token: token::Brace { span: proc_macro2::Span::call_site() },
            stmts: body,
        },
    }))
}
