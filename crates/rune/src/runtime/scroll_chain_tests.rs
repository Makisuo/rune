//! A wheel over a viewport that cannot move that way scrolls the one around it.
use super::*;
use crate::{col, el, ScrollSpec};

fn view(_: &(), _: &mut Cx<()>) -> El {
    // An outer viewport 100 tall over 300 of content; inside it, one that fits its content.
    let inner = col().key("inner").w_full().h(80.0).scroll(ScrollSpec { stick_bottom: false })
        .child(el().key("small").w_full().h(40.0));
    col().key("outer").size(100.0, 100.0).scroll(ScrollSpec { stick_bottom: false })
        .child(col().w_full().child(inner).child(el().key("below").w_full().h(220.0)))
}

#[test]
fn a_wheel_chains_past_a_viewport_with_nothing_to_scroll() {
    let mut rt = Runtime::new((), view, (100.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let (_, before) = rt.find("small").unwrap();
    rt.wheel(50.0, 20.0, -50.0);
    rt.frame(0.0);
    let (_, after) = rt.find("small").unwrap();
    assert!(after < before - 40.0, "the outer viewport scrolled: {before} -> {after}");
}
