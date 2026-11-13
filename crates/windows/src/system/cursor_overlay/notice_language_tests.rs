use super::text;

#[test]
fn chooses_ui_language_and_falls_back_to_english() {
    assert!(text(0x0804).contains("控制此电脑"));
    assert!(text(0x0404).contains("控制此電腦"));
    assert!(text(0x0411).contains("操作中"));
    assert!(text(0x040c).contains("contrôle"));
    assert_eq!(text(0x0000), "acdesktop is controlling this computer");
}
