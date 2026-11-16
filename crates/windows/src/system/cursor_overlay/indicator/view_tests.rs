use super::*;

#[test]
fn glow_is_subtle_and_fades_toward_the_screen_interior() {
    const EDGE: i32 = 24;
    for edge in 0..4 {
        let surface = edge_surface(EDGE, EDGE, edge, EDGE).unwrap();
        let (outer, inner) = match edge {
            0 => ((4, 0), (4, EDGE - 1)),
            1 => ((4, EDGE - 1), (4, 0)),
            2 => ((0, 4), (EDGE - 1, 4)),
            _ => ((EDGE - 1, 4), (0, 4)),
        };
        assert!(surface.alpha_at(outer.0, outer.1) <= 42);
        assert!(surface.alpha_at(outer.0, outer.1) > surface.alpha_at(inner.0, inner.1));
    }
}
