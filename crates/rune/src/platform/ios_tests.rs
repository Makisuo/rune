#[test]
fn insets_are_the_gaps_between_the_window_and_its_safe_area() {
    // An iPhone 17 Pro in portrait at 3x: 402 x 874 points, 62 above and 34 below.
    assert_eq!(super::insets([0, 0], [1206, 2622], [0, 186], [1206, 2334], 3.0), [62.0, 0.0, 34.0, 0.0]);
    // In landscape the sensor housing and home indicator move to the sides and bottom.
    assert_eq!(super::insets([0, 0], [2622, 1206], [186, 0], [2250, 1143], 3.0), [0.0, 62.0, 21.0, 62.0]);
}
