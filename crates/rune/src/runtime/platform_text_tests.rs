//! `Runtime::platform_text`: a platform text view's edits (iOS's text proxy) reaching the field.
use super::text_change;
use crate::{Cx, El, Env, Key, Runtime, TextEdit, TextStyle, col, text_input};

struct Form {
    field: TextEdit,
    submits: usize,
}

fn view(s: &Form, _: &mut Cx<Form>) -> El {
    col().w_full().h_full().child(
        text_input(&s.field, TextStyle::new(14.0, 400.0).lh(20.0))
            .key("field")
            .size(280.0, 24.0)
            .on_edit(|s: &mut Form| &mut s.field)
            .on_submit(|s: &mut Form, _| s.submits += 1),
    )
}

fn runtime(text: &str) -> Runtime<Form> {
    let mut field = TextEdit::new(text);
    field.focused = true;
    field.set_caret(text.len());
    let mut rt = Runtime::new(Form { field, submits: 0 }, view, (320.0, 120.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

#[test]
fn the_difference_is_the_smallest_replacement_on_character_boundaries() {
    assert_eq!(text_change("same", "same"), None);
    assert_eq!(text_change("teh cat", "the cat"), Some((1, 3, 3)));
    assert_eq!(text_change("hi", "hi there"), Some((2, 2, 8)));
    assert_eq!(text_change("hi there", "hi"), Some((2, 8, 2)));
    // é and è share their first byte: the change starts at the character, not inside it.
    assert_eq!(text_change("café", "cafè"), Some((3, 5, 5)));
    // A repeated letter: the end is matched first from the back, never past the start.
    assert_eq!(text_change("aa", "aaa"), Some((2, 2, 3)));
}

#[test]
fn an_autocorrection_replaces_the_word_and_takes_the_text_views_caret() {
    let mut rt = runtime("teh");
    rt.platform_text("the ".into(), 4, 4);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.field.text, "the ");
    assert_eq!((rt.state.field.anchor, rt.state.field.focus), (4, 4));
    assert_eq!(rt.focused_text(), Some(("the ".into(), 4, 4)));
}

#[test]
fn a_selection_alone_moves_the_caret_and_leaves_the_text() {
    let mut rt = runtime("hello world");
    rt.platform_text("hello world".into(), 6, 11);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.field.text, "hello world");
    assert_eq!((rt.state.field.anchor, rt.state.field.focus), (6, 11));
}

#[test]
fn return_from_the_text_view_is_enter_and_submits() {
    let mut rt = runtime("ship it");
    rt.key(crate::KeyEvent { key: Key::Enter, mods: crate::Mods::default() });
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.submits, 1);
}
