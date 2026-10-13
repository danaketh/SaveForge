//! Schema-driven documents (spec §4.6): describe the editor declaratively and
//! let `FormDocument` render it and apply edits to a JSON value.

pub mod path;
mod visibility;

pub use visibility::{VisibilityOp, VisibilityRule};
