use super::FrameWriter;
use crate::system::capture::persistent::FramePixels;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn saved_pngs_have_matching_timestamps_in_the_manifest() {
    crate::tree::fixture::bootstrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("acdesktop-frame-writer-{nonce}"));
    std::fs::create_dir(&dir).expect("scratch directory");
    let frame = FramePixels {
        width: 2,
        height: 2,
        pixels: vec![255; 16],
    };
    let mut writer = FrameWriter::open(&dir).expect("writer");
    writer.submit(&frame, 125).expect("first frame");
    writer.submit(&frame, 270).expect("second frame");
    writer.finish().expect("write complete");
    let rows: Vec<serde_json::Value> = std::fs::read_to_string(dir.join("frames.jsonl"))
        .expect("manifest")
        .lines()
        .map(|line| serde_json::from_str(line).expect("manifest row"))
        .collect();
    assert_eq!(rows.len(), 2);
    for (row, (file, at_ms)) in rows
        .iter()
        .zip([("frame-0000.png", 125), ("frame-0001.png", 270)])
    {
        assert_eq!(row["file"], file);
        assert_eq!(row["at_ms"], at_ms);
        let bytes = std::fs::read(dir.join(file)).expect("saved frame");
        assert_eq!(acdesktop_core::parse_png_dimensions(&bytes), Some((2, 2)));
    }
    std::fs::remove_dir_all(dir).expect("remove scratch directory");
}
