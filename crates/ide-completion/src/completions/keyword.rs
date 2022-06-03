//! Completes keywords, except:
//! - `self`, `super` and `crate`, as these are considered part of path completions.
//! - `await`, as this is a postfix completion we handle this in the postfix completions.

use syntax::ast::Item;

use crate::{
    context::NameRefContext, CompletionContext, CompletionItem, CompletionItemKind, Completions,
};

pub(crate) fn complete_expr_keyword(acc: &mut Completions, ctx: &CompletionContext) {
    let item = match ctx.nameref_ctx() {
        Some(NameRefContext { keyword: Some(item), record_expr: None, .. })
            if !ctx.is_non_trivial_path() =>
        {
            item
        }
        _ => return,
    };

    let mut add_keyword = |kw, snippet| add_keyword(acc, ctx, kw, snippet);

    match item {
        Item::Impl(it) => {
            if it.for_token().is_none() && it.trait_().is_none() && it.self_ty().is_some() {
                add_keyword("for", "for");
            }
            add_keyword("where", "where");
        }
        Item::Enum(_)
        | Item::Fn(_)
        | Item::Struct(_)
        | Item::Trait(_)
        | Item::TypeAlias(_)
        | Item::Union(_) => {
            add_keyword("where", "where");
        }
        _ => (),
    }
}

pub(super) fn add_keyword(acc: &mut Completions, ctx: &CompletionContext, kw: &str, snippet: &str) {
    let mut item = CompletionItem::new(CompletionItemKind::Keyword, ctx.source_range(), kw);

    match ctx.config.snippet_cap {
        Some(cap) => {
            if snippet.ends_with('}') && ctx.incomplete_let {
                // complete block expression snippets with a trailing semicolon, if inside an incomplete let
                cov_mark::hit!(let_semi);
                item.insert_snippet(cap, format!("{};", snippet));
            } else {
                item.insert_snippet(cap, snippet);
            }
        }
        None => {
            item.insert_text(if snippet.contains('$') { kw } else { snippet });
        }
    };
    item.add_to(acc);
}

#[cfg(test)]
mod tests {
    use expect_test::{expect, Expect};

    use crate::tests::{check_edit, completion_list};

    fn check(ra_fixture: &str, expect: Expect) {
        let actual = completion_list(ra_fixture);
        expect.assert_eq(&actual)
    }

    #[test]
    fn test_else_edit_after_if() {
        check_edit(
            "else",
            r#"fn quux() { if true { () } $0 }"#,
            r#"fn quux() { if true { () } else {
    $0
} }"#,
        );
    }

    #[test]
    fn test_keywords_after_unsafe_in_block_expr() {
        check(
            r"fn my_fn() { unsafe $0 }",
            expect![[r#"
                kw fn
                kw impl
                kw trait
                sn pd
                sn ppd
            "#]],
        );
    }

    #[test]
    fn test_completion_await_impls_future() {
        check(
            r#"
//- minicore: future
use core::future::*;
struct A {}
impl Future for A {}
fn foo(a: A) { a.$0 }
"#,
            expect![[r#"
                kw await expr.await
                sn box   Box::new(expr)
                sn call  function(expr)
                sn dbg   dbg!(expr)
                sn dbgr  dbg!(&expr)
                sn let   let
                sn letm  let mut
                sn match match expr {}
                sn ref   &expr
                sn refm  &mut expr
            "#]],
        );

        check(
            r#"
//- minicore: future
use std::future::*;
fn foo() {
    let a = async {};
    a.$0
}
"#,
            expect![[r#"
                kw await expr.await
                sn box   Box::new(expr)
                sn call  function(expr)
                sn dbg   dbg!(expr)
                sn dbgr  dbg!(&expr)
                sn let   let
                sn letm  let mut
                sn match match expr {}
                sn ref   &expr
                sn refm  &mut expr
            "#]],
        )
    }

    #[test]
    fn let_semi() {
        cov_mark::check!(let_semi);
        check_edit(
            "match",
            r#"
fn main() { let x = $0 }
"#,
            r#"
fn main() { let x = match $1 {
    $0
}; }
"#,
        );

        check_edit(
            "if",
            r#"
fn main() {
    let x = $0
    let y = 92;
}
"#,
            r#"
fn main() {
    let x = if $1 {
    $0
};
    let y = 92;
}
"#,
        );

        check_edit(
            "loop",
            r#"
fn main() {
    let x = $0
    bar();
}
"#,
            r#"
fn main() {
    let x = loop {
    $0
};
    bar();
}
"#,
        );
    }
}
