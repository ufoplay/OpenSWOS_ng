//! `lint_magic`: finds magic numbers and undocumented constants (docs/PLAN.md §5.4).
//!
//! Every numeric literal other than `0`/`1` must live in a `const`/`static` item, and
//! every such item needs a doc comment saying what the value means, its unit and where
//! it comes from. Test code (`#[cfg(test)]`, `#[test]`, `tests/`), enum discriminants,
//! attribute arguments and macro bodies are not checked.

pub mod scan;
pub mod visitor;
