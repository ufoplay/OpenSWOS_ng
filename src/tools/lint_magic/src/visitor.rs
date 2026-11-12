//! The rules, on one parsed Rust file.

use std::fmt;

use syn::visit::{self, Visit};
use syn::{
    Attribute, Ident, ImplItemConst, ItemConst, ItemFn, ItemImpl, ItemMod, ItemStatic, LitFloat,
    LitInt, TraitItemConst, Variant,
};

/// Integer values allowed outside constants: they carry no domain meaning.
const ALLOWED_INTEGERS: &[u128] = &[0, 1];

/// Float values allowed outside constants.
const ALLOWED_FLOATS: &[&str] = &["0.0", "1.0"];

/// The name of an anonymous constant (`const _: () = ...;`), used for compile-time
/// assertions: it needs no documentation.
const ANONYMOUS_CONST: &str = "_";

/// What a finding is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    /// A numeric literal outside a `const`/`static` item.
    MagicNumber(String),
    /// A `const`/`static` item without a doc comment.
    UndocumentedConst(String),
}

/// One rule violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// 1-based line.
    pub line: usize,
    /// 1-based column.
    pub column: usize,
    /// What is wrong.
    pub kind: FindingKind,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { line, column, kind } = self;
        match kind {
            FindingKind::MagicNumber(literal) => write!(
                f,
                "{line}:{column}: magic number `{literal}`: name it with a documented const"
            ),
            FindingKind::UndocumentedConst(name) => {
                write!(f, "{line}:{column}: `{name}` has no doc comment")
            }
        }
    }
}

/// Every finding in `source`, a whole Rust file, in source order.
///
/// # Errors
///
/// Fails when `source` does not parse.
pub fn lint_source(source: &str) -> Result<Vec<Finding>, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut visitor = Visitor::default();
    visitor.visit_file(&file);
    Ok(visitor.findings)
}

#[derive(Default)]
struct Visitor {
    /// How many `const`/`static` items enclose the current node.
    const_depth: usize,
    findings: Vec<Finding>,
}

impl Visitor {
    fn push(&mut self, span: proc_macro2::Span, kind: FindingKind) {
        let start = span.start();
        self.findings.push(Finding {
            line: start.line,
            column: start.column + 1,
            kind,
        });
    }

    fn require_docs(&mut self, attrs: &[Attribute], ident: &Ident) {
        if ident != ANONYMOUS_CONST && !attrs.iter().any(|attr| attr.path().is_ident("doc")) {
            self.push(
                ident.span(),
                FindingKind::UndocumentedConst(ident.to_string()),
            );
        }
    }

    fn inside_const(&mut self, visit: impl FnOnce(&mut Self)) {
        self.const_depth += 1;
        visit(self);
        self.const_depth -= 1;
    }
}

/// Whether the attributes mark test-only code: `#[test]` (or `#[tokio::test]` & co.),
/// or `#[cfg(test)]`.
fn is_test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        let is_test_attr = path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "test");
        let is_cfg_test = path.is_ident("cfg")
            && attr
                .parse_args::<Ident>()
                .is_ok_and(|ident| ident == "test");
        is_test_attr || is_cfg_test
    })
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        if !is_test_only(&item.attrs) {
            visit::visit_item_mod(self, item);
        }
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if !is_test_only(&item.attrs) {
            visit::visit_item_fn(self, item);
        }
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if !is_test_only(&item.attrs) {
            visit::visit_item_impl(self, item);
        }
    }

    fn visit_item_const(&mut self, item: &'ast ItemConst) {
        if is_test_only(&item.attrs) {
            return;
        }
        self.require_docs(&item.attrs, &item.ident);
        self.inside_const(|visitor| visit::visit_item_const(visitor, item));
    }

    fn visit_item_static(&mut self, item: &'ast ItemStatic) {
        if is_test_only(&item.attrs) {
            return;
        }
        self.require_docs(&item.attrs, &item.ident);
        self.inside_const(|visitor| visit::visit_item_static(visitor, item));
    }

    fn visit_impl_item_const(&mut self, item: &'ast ImplItemConst) {
        self.require_docs(&item.attrs, &item.ident);
        self.inside_const(|visitor| visit::visit_impl_item_const(visitor, item));
    }

    fn visit_trait_item_const(&mut self, item: &'ast TraitItemConst) {
        self.require_docs(&item.attrs, &item.ident);
        self.inside_const(|visitor| visit::visit_trait_item_const(visitor, item));
    }

    /// An explicit discriminant is the variant's value, and the variant names it: it is
    /// not a magic number.
    fn visit_variant(&mut self, variant: &'ast Variant) {
        self.visit_fields(&variant.fields);
    }

    /// Attribute arguments (`#[repr(align(4))]`, `#[arg(default_value_t = 3)]`...) are
    /// configuration, not code.
    fn visit_attribute(&mut self, _attr: &'ast Attribute) {}

    fn visit_lit_int(&mut self, lit: &'ast LitInt) {
        let allowed = lit
            .base10_parse::<u128>()
            .is_ok_and(|value| ALLOWED_INTEGERS.contains(&value));
        if self.const_depth == 0 && !allowed {
            self.push(lit.span(), FindingKind::MagicNumber(lit.to_string()));
        }
    }

    fn visit_lit_float(&mut self, lit: &'ast LitFloat) {
        let allowed = ALLOWED_FLOATS.contains(&lit.base10_digits());
        if self.const_depth == 0 && !allowed {
            self.push(lit.span(), FindingKind::MagicNumber(lit.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<FindingKind> {
        lint_source(source)
            .expect("valid Rust")
            .into_iter()
            .map(|finding| finding.kind)
            .collect()
    }

    fn magic(literal: &str) -> FindingKind {
        FindingKind::MagicNumber(literal.to_owned())
    }

    #[test]
    fn literals_in_code_are_magic() {
        assert_eq!(kinds("fn f(x: u8) -> u8 { x >> 5 }"), vec![magic("5")]);
        assert_eq!(kinds("fn f() -> f32 { 2.5 }"), vec![magic("2.5")]);
    }

    #[test]
    fn zero_and_one_are_allowed() {
        assert!(kinds("fn f(x: u8) -> u8 { x.wrapping_add(1) * 0 + 1u8 }").is_empty());
        assert!(kinds("fn f() -> f32 { 1.0 - 0.0 }").is_empty());
    }

    #[test]
    fn literals_in_documented_constants_are_fine() {
        let source = "/// Doc.\nconst A: u8 = 42;\n/// Doc.\nstatic B: [u8; 3] = [7, 8, 9];";
        assert!(kinds(source).is_empty());
    }

    #[test]
    fn associated_and_trait_constants_count_as_constants() {
        let source = "struct S;\nimpl S {\n    /// Doc.\n    const A: u8 = 42;\n}\n\
                      trait T {\n    /// Doc.\n    const B: u8 = 7;\n}";
        assert!(kinds(source).is_empty());
    }

    #[test]
    fn undocumented_constants_are_reported() {
        assert_eq!(
            kinds("const A: u8 = 42;"),
            vec![FindingKind::UndocumentedConst("A".to_owned())]
        );
        assert_eq!(
            kinds("static B: u8 = 0;"),
            vec![FindingKind::UndocumentedConst("B".to_owned())]
        );
        let source = "struct S;\nimpl S { const C: u8 = 2; }";
        assert_eq!(
            kinds(source),
            vec![FindingKind::UndocumentedConst("C".to_owned())]
        );
    }

    #[test]
    fn anonymous_constants_need_no_docs() {
        assert!(kinds("const _: () = assert!(true);").is_empty());
    }

    #[test]
    fn array_lengths_in_types_are_magic() {
        assert_eq!(kinds("struct S { name: [u8; 17] }"), vec![magic("17")]);
    }

    #[test]
    fn enum_discriminants_are_named_values() {
        assert!(kinds("#[repr(u8)]\nenum Position { Goalkeeper = 0, Striker = 7 }").is_empty());
    }

    #[test]
    fn attribute_arguments_are_ignored() {
        assert!(kinds("#[repr(align(4))]\nstruct S;").is_empty());
    }

    #[test]
    fn test_code_is_ignored() {
        let source = "#[cfg(test)]\nmod tests {\n    fn helper() -> u8 { 42 }\n}\n\
                      #[test]\nfn check() { let _ = 99; }\n\
                      #[cfg(test)]\nconst FIXTURE: u8 = 3;\n\
                      #[tokio::test]\nasync fn check_async() { let _ = 98; }";
        assert!(kinds(source).is_empty());
    }

    #[test]
    fn other_cfgs_are_not_test_code() {
        assert_eq!(
            kinds("#[cfg(unix)]\nfn f() -> u8 { 42 }"),
            vec![magic("42")]
        );
    }

    #[test]
    fn macro_bodies_are_not_parsed() {
        assert!(kinds("fn f() { println!(\"{}\", 42); }").is_empty());
    }

    #[test]
    fn findings_carry_their_position() {
        let findings = lint_source("fn f() -> u8 {\n    40 + 2\n}").expect("valid Rust");
        let positions: Vec<(usize, usize)> = findings.iter().map(|f| (f.line, f.column)).collect();
        assert_eq!(positions, vec![(2, 5), (2, 10)]);
        assert_eq!(
            findings.first().map(ToString::to_string).as_deref(),
            Some("2:5: magic number `40`: name it with a documented const")
        );
    }

    #[test]
    fn invalid_rust_is_an_error() {
        assert!(lint_source("fn (").is_err());
    }
}
