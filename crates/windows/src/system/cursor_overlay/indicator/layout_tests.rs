use super::*;

#[test]
fn negative_origin_and_scaled_displays_keep_edges_and_status_on_their_monitor() {
    let monitor = OverlayMonitor {
        bounds: Rect {
            x: -2560.0,
            y: -200.0,
            width: 2560.0,
            height: 1440.0,
        },
        work_area: Rect {
            x: -2560.0,
            y: -200.0,
            width: 2560.0,
            height: 1400.0,
        },
        scale: 2.0,
        is_primary: false,
    };
    let layout = display_layout(&monitor).unwrap();
    assert_eq!(layout.edge, 48);
    assert_eq!(layout.edges[0], (-2560, -200, 2560, 48, 0));
    assert_eq!(layout.edges[3], (-48, -152, 48, 1344, 3));
    assert_eq!(layout.status, (-1920, -152, 1280, 180));
}

#[test]
fn tiny_or_overflowing_displays_are_rejected() {
    let monitor = OverlayMonitor {
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 20.0,
        },
        work_area: Rect {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 20.0,
        },
        scale: 1.0,
        is_primary: true,
    };
    assert!(display_layout(&monitor).is_err());
}
