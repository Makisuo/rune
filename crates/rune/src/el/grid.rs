//! Grid tracks and placement: CSS `grid-template-columns`, `grid-template-rows` and
//! `grid-column` spans, for layouts `equal_columns` can't express (a fixed sidebar beside a
//! flexible column, a label column sized to its widest label).

use super::*;

/// One grid track's size, as CSS writes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridTrack {
    /// A fixed size in points.
    Px(f32),
    /// A share of the space the fixed and content-sized tracks leave (`1fr`).
    Fr(f32),
    /// Sized to its content, stretching into free space (`auto`).
    Auto,
    /// As narrow as its content can be (`min-content`).
    MinContent,
    /// As wide as its content wants (`max-content`).
    MaxContent,
}

impl GridTrack {
    fn sizing(self) -> TrackSizingFunction {
        match self {
            GridTrack::Px(v) => length(v),
            GridTrack::Fr(f) => fr(f),
            GridTrack::Auto => auto(),
            GridTrack::MinContent => min_content(),
            GridTrack::MaxContent => max_content(),
        }
    }
}

impl El {
    /// Lay children out on a grid with these columns, filling the cells row by row. Rows are
    /// added as needed, each as tall as its tallest cell; `gaps` spaces rows and columns.
    pub fn grid_columns(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Self {
        self.style.set_grid_template_columns(tracks.into_iter().map(GridTrack::sizing).collect());
        self
    }
    /// The grid's row tracks; rows past them size to their content.
    pub fn grid_rows(mut self, tracks: impl IntoIterator<Item = GridTrack>) -> Self {
        self.style.set_grid_template_rows(tracks.into_iter().map(GridTrack::sizing).collect());
        self
    }
    /// Take `n` columns of a `grid_columns` parent, from the next free cell (`grid-column: span n`).
    pub fn grid_column_span(mut self, n: u16) -> Self {
        self.style.set_grid_column(GridPlacement::Auto, span(n.max(1)));
        self
    }
    /// Take every column of a `grid_columns` parent, on a row of its own (`grid-column: 1 / -1`).
    pub fn grid_column_full(mut self) -> Self {
        self.style.set_grid_column(line(1), line(-1));
        self
    }
    /// Start at row line `n` of a `grid_rows` parent, 1 the first (`grid-row-start: n`).
    pub fn grid_row_start(mut self, n: i16) -> Self {
        self.style.set_grid_row(line(n), GridPlacement::Auto);
        self
    }
    /// Centre each item across its cell, at its own width (`justify-items: center`).
    pub fn grid_justify_center(mut self) -> Self {
        self.style.set_justify_items(AlignItems::CENTER);
        self
    }
}
