//! Bridges the SDK traits to the generated wit-bindgen `Guest` traits.

use std::cell::RefCell;
use std::fmt;
use std::marker::PhantomData;

use crate::bindings::exports::saveforge::plugin::editor::{self as wit, AuxFile};
use crate::ui::{Event, UiTree};
use crate::{Document, Plugin};

#[doc(hidden)]
pub struct Adapter<P>(PhantomData<P>);

#[doc(hidden)]
pub struct DocAdapter(RefCell<Box<dyn Document>>);

impl<P> fmt::Debug for Adapter<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Adapter")
    }
}

impl fmt::Debug for DocAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DocAdapter")
    }
}

impl<P: Plugin> wit::Guest for Adapter<P> {
    type Document = DocAdapter;

    fn detect(file_name: String, header: Vec<u8>) -> Option<u32> {
        P::detect(&file_name, &header)
    }

    fn open(file_name: String, bytes: Vec<u8>, aux: Vec<AuxFile>) -> Result<wit::Document, String> {
        let doc = P::open(&file_name, bytes, aux)?;
        Ok(wit::Document::new(DocAdapter(RefCell::new(Box::new(doc)))))
    }
}

impl wit::GuestDocument for DocAdapter {
    fn view(&self) -> UiTree {
        self.0.borrow().view()
    }

    fn update(&self, ev: Event) {
        self.0.borrow_mut().update(ev);
    }

    fn is_dirty(&self) -> bool {
        self.0.borrow().is_dirty()
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        self.0.borrow().serialize()
    }

    fn mark_saved(&self) {
        self.0.borrow_mut().mark_saved();
    }
}
