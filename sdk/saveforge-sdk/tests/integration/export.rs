//! Native-target checks: the export macro expands, the adapter forwards to the
//! plugin, and logging works off-wasm. The full wasm round trip is covered by
//! the host tests against the fixture plugin.

use saveforge_sdk::bindings::exports::saveforge::plugin::editor::Guest;
use saveforge_sdk::ui::{Event, UiTree};
use saveforge_sdk::{Adapter, AuxFile, Document, Level, Plugin, log};

struct Echo;

struct EchoDoc;

impl Document for EchoDoc {
    fn view(&self) -> UiTree {
        UiTree { nodes: Vec::new() }
    }
    fn update(&mut self, _event: Event) {}
    fn is_dirty(&self) -> bool {
        false
    }
    fn serialize(&self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
    fn mark_saved(&mut self) {}
}

impl Plugin for Echo {
    type Doc = EchoDoc;

    fn detect(file_name: &str, header: &[u8]) -> Option<u32> {
        let is_echo = std::path::Path::new(file_name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("echo"));
        (is_echo && !header.is_empty()).then_some(3)
    }

    fn open(_file_name: &str, _bytes: Vec<u8>, _aux: Vec<AuxFile>) -> Result<EchoDoc, String> {
        Ok(EchoDoc)
    }
}

saveforge_sdk::export_plugin!(Echo);

#[test]
fn adapter_forwards_detect() {
    assert_eq!(
        <Adapter<Echo> as Guest>::detect("a.echo".to_owned(), vec![1]),
        Some(3)
    );
    assert_eq!(
        <Adapter<Echo> as Guest>::detect("a.echo".to_owned(), vec![]),
        None
    );
    assert_eq!(
        <Adapter<Echo> as Guest>::detect("a.txt".to_owned(), vec![1]),
        None
    );
}

#[test]
fn logging_works_off_wasm() {
    log(Level::Info, "plain message");
    saveforge_sdk::info!("formatted {}", 1);
    saveforge_sdk::warn!("w");
    saveforge_sdk::error!("e");
    saveforge_sdk::debug!("d");
}
