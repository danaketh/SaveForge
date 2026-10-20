use crate::common::World;
use saveforge_app::documents::{DetectOutcome, Failure, detect_file, open_document};
use saveforge_app::editor::{Editor, EditorEffect};
use saveforge_app::render::UiMessage;
use saveforge_host::Event;
use saveforge_plugin_api::ui::Widget;

fn open(world: &World, content: &str) -> Editor {
    let path = world.save_file("a.fixture", content);
    let DetectOutcome::Open(plugin) = detect_file(&world.services.engine, &world.registry(), &path)
        .unwrap()
        .outcome
    else {
        panic!("fixture did not match");
    };
    Editor::new(
        open_document(
            &world.services.engine,
            &plugin,
            &path,
            &world.services.tokens,
        )
        .unwrap(),
    )
}

fn shown_text(editor: &Editor) -> &str {
    match &editor.tree.nodes[1].widget {
        Widget::Text(span) => &span.text,
        other => panic!("unexpected {other:?}"),
    }
}

fn type_text(value: &str) -> UiMessage {
    UiMessage::Plugin(Event::TextChanged(("text".to_owned(), value.to_owned())))
}

#[test]
fn plugin_event_updates_tree_and_dirty() {
    let world = World::new();
    let mut editor = open(&world, "hello");
    assert_eq!(editor.file_name(), "a.fixture");
    assert!(!editor.dirty);
    assert_eq!(editor.handle(type_text("bye")), EditorEffect::None);
    assert_eq!(shown_text(&editor), "bye");
    assert!(editor.dirty);
    assert!(editor.has_unsaved_changes());
}

#[test]
fn number_edit_for_unknown_field_is_ignored() {
    let world = World::new();
    let mut editor = open(&world, "hello");
    let effect = editor.handle(UiMessage::NumberEdited {
        id: "nope".to_owned(),
        text: "5".to_owned(),
    });
    assert_eq!(effect, EditorEffect::None);
    assert!(!editor.dirty);
}

#[test]
fn list_choice_is_host_side_only() {
    let world = World::new();
    let mut editor = open(&world, "hello");
    editor.handle(UiMessage::ListChoice {
        id: "inv".to_owned(),
        value: "iron".to_owned(),
    });
    // The fixture has no list, but the choice is recorded until the next tree.
    assert_eq!(editor.ui.list_choice("inv"), Some("iron"));
    assert!(!editor.dirty);
}

#[test]
fn plugin_crash_marks_editor_and_reports_fault() {
    let world = World::new();
    let mut editor = open(&world, "hello");
    editor.handle(type_text("changed"));
    let effect = editor.handle(UiMessage::Plugin(Event::Pressed("spin".to_owned())));
    assert_eq!(effect, EditorEffect::PluginFault);
    assert!(editor.crashed.is_some());
    // A crashed document has nothing left to save.
    assert!(!editor.has_unsaved_changes());
    // Later input is ignored.
    assert_eq!(editor.handle(type_text("x")), EditorEffect::None);
    assert_eq!(shown_text(&editor), "changed");
}

#[test]
fn input_is_ignored_while_saving() {
    let world = World::new();
    let mut editor = open(&world, "hello");
    editor.saving = true;
    editor.handle(type_text("bye"));
    assert_eq!(shown_text(&editor), "hello");
    editor.saved();
    assert!(!editor.saving);
    assert!(!editor.dirty);
}

#[test]
fn non_fault_crash_is_not_counted() {
    let world = World::new();
    let mut editor = open(&world, "hello");
    assert_eq!(editor.crash(&Failure::new("disk gone")), EditorEffect::None);
    assert_eq!(editor.crashed.as_deref(), Some("disk gone"));
}
