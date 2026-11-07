//! `El::grid_columns`, `grid_rows` and column spans: CSS grid tracks on the layout engine.
use super::*;
use crate::el::GridTrack;
use crate::{Cx, El, Env, col, el};

#[derive(Clone, Copy)]
enum Case {
    /// A fixed sidebar beside a flexible column, two rows.
    SidebarAndFr,
    /// A label column as wide as its widest label, then the rest.
    AutoThenFr,
    /// A heading across both columns, then two cells.
    FullSpan,
    /// Fixed row heights.
    Rows,
}

fn view(case: &Case, _: &mut Cx<Case>) -> El {
    let cell = |key: &str| el().key(key.to_string()).h(20.0);
    let grid = el().key("grid").w(300.0).gaps(4.0, 10.0);
    // The root takes the window's size; the grid sits in it at its own width.
    col().child(match case {
        Case::SidebarAndFr => grid
            .grid_columns([GridTrack::Px(80.0), GridTrack::Fr(1.0)])
            .children(["a", "b", "c", "d"].map(cell)),
        Case::AutoThenFr => grid
            .grid_columns([GridTrack::Auto, GridTrack::Fr(1.0)])
            .child(el().key("label").w(60.0).h(20.0))
            .child(cell("value"))
            .child(el().key("short").w(30.0).h(20.0))
            .child(cell("value2")),
        Case::FullSpan => grid
            .grid_columns([GridTrack::Fr(1.0), GridTrack::Fr(1.0)])
            .child(cell("heading").grid_column_full())
            .children(["a", "b"].map(cell)),
        Case::Rows => grid
            .grid_columns([GridTrack::Fr(1.0)])
            .grid_rows([GridTrack::Px(30.0), GridTrack::Px(50.0)])
            .children([el().key("a"), el().key("b")]),
    })
}

fn rects(case: Case, keys: &[&str]) -> Vec<[f32; 4]> {
    let mut rt = Runtime::new(case, view, (400.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    keys.iter().map(|k| rt.presented(k).unwrap_or_else(|| panic!("{k} not drawn")).0).collect()
}

#[test]
fn a_fixed_track_beside_a_fraction_takes_its_size_and_leaves_the_rest() {
    let r = rects(Case::SidebarAndFr, &["a", "b", "c", "d"]);
    // 300 wide, a 10-point column gap: 80 and 210.
    assert_eq!(r[0], [0.0, 0.0, 80.0, 20.0]);
    assert_eq!(r[1], [90.0, 0.0, 210.0, 20.0]);
    // The third child starts the second row, 4 points below.
    assert_eq!(r[2], [0.0, 24.0, 80.0, 20.0]);
    assert_eq!(r[3], [90.0, 24.0, 210.0, 20.0]);
}

#[test]
fn an_auto_track_is_as_wide_as_its_widest_cell() {
    let r = rects(Case::AutoThenFr, &["label", "value", "short", "value2"]);
    assert_eq!(r[0][2], 60.0);
    assert_eq!(r[1][0], 70.0, "the value column starts after the widest label and the gap");
    assert_eq!(r[3][0], 70.0, "every row shares the label column's width");
}

#[test]
fn a_full_span_takes_a_row_of_its_own() {
    let r = rects(Case::FullSpan, &["heading", "a", "b"]);
    assert_eq!(r[0], [0.0, 0.0, 300.0, 20.0]);
    assert_eq!(r[1], [0.0, 24.0, 145.0, 20.0]);
    assert_eq!(r[2], [155.0, 24.0, 145.0, 20.0]);
}

#[test]
fn row_tracks_size_the_rows() {
    let r = rects(Case::Rows, &["a", "b"]);
    assert_eq!((r[0][1], r[0][3]), (0.0, 30.0));
    assert_eq!((r[1][1], r[1][3]), (34.0, 50.0));
}
