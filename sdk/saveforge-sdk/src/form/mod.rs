//! Schema-driven documents (spec §4.6): describe the editor declaratively and
//! let [`FormDocument`] render it and apply edits to a JSON value.

mod document;
pub mod path;
mod schema;
mod visibility;

pub use document::{ADD_SUFFIX, Codec, FormDocument, JsonCodec, TABS_ID};
pub use schema::{
    FieldDef, FieldKind, FormPage, FormSchema, FormSection, checkbox, choice, list, number, object,
    text,
};
pub use visibility::{VisibilityOp, VisibilityRule};
