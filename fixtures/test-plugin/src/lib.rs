//! Fixture plugin for `saveforge-host` tests. Every fault the host must survive
//! can be provoked through its inputs (see each match arm).

use std::fmt::Write as _;

use saveforge_sdk::ui::{Event, Node, UiTree, Widget, text, text_input};
use saveforge_sdk::{AuxFile, Document, Plugin, col, export_plugin, info};

#[derive(Debug)]
pub struct Fixture;

#[derive(Debug)]
pub struct FixtureDoc {
    text: String,
    dirty: bool,
}

impl Plugin for Fixture {
    type Doc = FixtureDoc;

    fn detect(file_name: &str, header: &[u8]) -> Option<u32> {
        if file_name == "spin-detect.fixture" {
            spin();
        }
        // The host must truncate headers to 512 bytes; refusing longer ones
        // lets a test prove that it did.
        if header.len() > 512 || header.starts_with(b"no-match") {
            return None;
        }
        Some(5)
    }

    fn open(file_name: &str, bytes: Vec<u8>, aux: Vec<AuxFile>) -> Result<FixtureDoc, String> {
        info!("opening {file_name}");
        match bytes.as_slice() {
            b"panic" => panic!("fixture panic in open"),
            b"hog" => hog(),
            b"reject" => return Err("rejected by fixture".to_owned()),
            _ => {}
        }
        let mut text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        for file in aux {
            write!(
                text,
                " +{}={}",
                file.id,
                String::from_utf8_lossy(&file.bytes)
            )
            .expect("writing to a String cannot fail");
        }
        Ok(FixtureDoc { text, dirty: false })
    }
}

impl Document for FixtureDoc {
    fn view(&self) -> UiTree {
        match self.text.as_str() {
            "spin-view" => spin(),
            "bad-tree" => UiTree {
                nodes: vec![Node {
                    widget: Widget::Column,
                    children: vec![7],
                }],
            },
            _ => col![
                text(self.text.clone()),
                text_input("text", "Text", self.text.clone())
            ]
            .into_tree(),
        }
    }

    fn update(&mut self, event: Event) {
        match event {
            Event::TextChanged((id, value)) if id == "text" => {
                self.text = value;
                self.dirty = true;
            }
            Event::Pressed(id) if id == "spin" => spin(),
            Event::Pressed(id) if id == "panic" => panic!("fixture panic in update"),
            _ => {}
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn serialize(&self) -> Result<Vec<u8>, String> {
        if self.text == "unserializable" {
            Err("cannot serialize".to_owned())
        } else {
            Ok(self.text.clone().into_bytes())
        }
    }

    fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

fn spin() -> ! {
    loop {
        std::hint::spin_loop();
    }
}

fn hog() -> ! {
    let mut blocks: Vec<Vec<u8>> = Vec::new();
    loop {
        blocks.push(vec![1u8; 16 * 1024 * 1024]);
        std::hint::black_box(&blocks);
    }
}

export_plugin!(Fixture);
