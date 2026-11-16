use super::*;

#[test]
fn hosted_helper_must_be_a_regular_acdesktop_executable() {
    let path = std::env::temp_dir().join(format!("acdesktop-helper-test-{}", std::process::id()));
    std::fs::create_dir(&path).unwrap();
    let invalid = path.join("other.exe");
    std::fs::write(&invalid, b"fixture").unwrap();
    assert!(HelperImage::open(&invalid).is_err());
    let valid = path.join("acdesktop.exe");
    std::fs::write(&valid, b"fixture").unwrap();
    let image = HelperImage::open(&valid).unwrap();
    assert_eq!(image.path, valid);
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(&valid)
            .is_err()
    );
    assert!(ensure_distinct(&image._file, &image._file).is_err());
    let host = File::open(std::env::current_exe().unwrap()).unwrap();
    assert!(ensure_distinct(&image._file, &host).is_ok());
    drop(image);
    std::fs::remove_file(invalid).unwrap();
    std::fs::remove_file(valid).unwrap();
    std::fs::remove_dir(path).unwrap();
}
