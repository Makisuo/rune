- **Grid tracks: `El::grid_columns`, `El::grid_rows`, `GridTrack`, and column spans.** A grid's
  columns and rows can now be any mix of fixed points, fractions of the free space, and
  content-sized tracks (`GridTrack::Px`, `Fr`, `Auto`, `MinContent`, `MaxContent`), as CSS
  `grid-template-columns` and `grid-template-rows` write them; children fill the cells row by row
  and `gaps` spaces them. `El::grid_column_span(n)` and `El::grid_column_full()` place a child
  across columns (`grid-column: span n` and `1 / -1`). Before, only `equal_columns` reached the
  grid layout Taffy already ran.
