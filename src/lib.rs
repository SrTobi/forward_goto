#![allow(unused)]
#![feature(cmp_min_max_by)]

extern crate proc_macro;
extern crate proc_macro2;

mod collector;

use std::cmp::min;
use collector::Collector;
use quote::{quote, quote_spanned};
use quote::ToTokens;
use syn::*;


/*


    {
        println!("before");
        forward_goto!('test);

        println!("inbetween");

        forward_label('test);
        println!("end");
    }


    {
        println!("before");
        'test: loop {
            break 'test;

            println!("inbetween");
            break 'test;
        }

        println!("end");
    }



    {
        println!("before");
        if test1 {
            forward_goto!('test);
        }

        println!("inbetween");

        if test2 {
            if test {
                forward_label('test);
                println!("after");
            } else {
                println!("alternative");
            }
            println!("together!");
        }

        println!("end");
    }

    {
        println!("before");
        loop 'outer: {
            loop 'inner: {
                loop 'test: {
                    if test1 {
                        break 'test;
                    }

                    println!("inbetween");

                    if test2 {
                        if test {
                            break 'test;
                        } else {
                            println!("alternative");
                        }
                        break 'inner;
                    }
                    break 'outer;
                } // end 'test

                println!("after");
                break 'inner;

            } // end 'inner

            println!("together!");
            break 'outer;

        } // end 'outer

        println!("end");
    }

*/


#[proc_macro_attribute]
pub fn rewrite_forward_goto(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(item as ItemFn);

    let mut collector = Collector::new();
    traverse_boxed_block(&mut input.block, &mut collector);

    let result = proc_macro::TokenStream::from(quote!(#input));
    println!("done");
    println!("{}", &result);
    result
}

fn traverse_boxed_block(boxed: &mut Box<syn::Block>, collector: &mut Collector) {
    traverse_stmts(&mut boxed.stmts, collector)
}

fn traverse_stmts(stmts: &mut Vec<Stmt>, collector: &mut Collector) {
    let mut i = 0;
    while i < stmts.len() {
        println!("start stmt");
        {
            let stmt = stmts.get_mut(i).unwrap();
            let mut collector = collector.enter_statement(i);
            traverse_stmt(stmt, &mut collector);
        }

        if let Some((start_index, end_label, continuations)) = collector.retrieve_continuations() {
            println!("build goto {}", i);
            let rest = stmts.split_off(i + 1);

            i = start_index;
            let mut inner = stmts.split_off(start_index);
            inner.push(new_break_stmt(end_label.clone()));

            for (start_index, incomings, continuation, outgoing) in continuations {
                inner = {
                    let mut inside_stmts = if i == start_index {
                        inner
                    } else {
                        debug_assert!(start_index < i);
                        i = start_index;
                        let mut inside_stmts = stmts.split_off(start_index);
                        inside_stmts.extend(inner.into_iter());
                        inside_stmts
                    };

                    if let Some(last) = incomings.last().cloned() {
                        for (idx, incoming) in incomings.into_iter().enumerate() {
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
            println!("finished build goto {} in {}", i, stmts.len());
            continue;
        }


        if collector.should_push_continuation() {
            let continuation = stmts.split_off(i + 1);
            println!("push continuation {}", continuation.len());
            let target = collector.push_continuation(continuation);
            stmts.push(expr_to_stmt(new_break_expr(target)));
            println!("pushed continuation");
            return;
        }
        println!("end stmt");

        i += 1;
    }
}

fn traverse_stmt(stmt: &mut Stmt, collector: &mut Collector) {
    match stmt {
        Stmt::Item(_) => (),
        Stmt::Local(local) => {
            match local.init {
                Some((_, ref mut expr_box)) => 
                    traverse_boxed_expr(expr_box, collector),
                None => (),
            }
        },
        Stmt::Expr(expr) => traverse_expr(expr, collector, true),
        Stmt::Semi(expr, _) => traverse_expr(expr, collector, true),
    }
}

fn traverse_boxed_expr(expr: &mut Box<Expr>, collector: &mut Collector) {
    traverse_expr(expr, collector, false);
}

fn traverse_expr(expr: &mut Expr, collector: &mut Collector, is_statement: bool) {
    let replacement_expr = match expr {
        Expr::Macro(mac) => {
            let mac = &mac.mac;
            let path = &mac.path;
            let forward_macro = path.is_ident("forward_goto") || path.is_ident("forward_label");
            if forward_macro {
                let tokens = &mac.tokens;
                let lifetime: Lifetime = parse2(tokens.clone()).unwrap();

                println!("found macro");
                if path.is_ident("forward_goto") {
                    collector.add_goto(lifetime.clone());
                } else {
                    collector.add_label(lifetime.clone());
                }

                Some(new_break_expr(lifetime))
            } else {
                None
            }
        },
        Expr::If(ExprIf { cond, then_branch, else_branch, .. }) => {
            traverse_boxed_expr(cond, &mut collector.cut());
            traverse_stmts(&mut then_branch.stmts, collector);
            if let Some((_, expr)) = else_branch {
                println!("traverse else");
                traverse_boxed_expr(expr, &mut collector.enter());
            }
            None
        },
        Expr::Match(ExprMatch { expr, arms, .. }) => {
            traverse_boxed_expr(expr, &mut collector.cut());
            for arm in arms.iter_mut() {
                traverse_boxed_expr(&mut arm.body, &mut collector.enter());
            }
            None
        },
        Expr::Block(ExprBlock { block, ..}) => {
            traverse_stmts(&mut block.stmts, collector);
            None
        },
        _ => None,
    };

    if let Some(replacement) = replacement_expr {
        *expr = replacement;
    }
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

fn new_loop_block(lifetime: Lifetime, body: Vec<Stmt>) -> Stmt {
    let result = parse_quote!{
        #lifetime: loop {
            #(#body)*
        };
    };
    result
}