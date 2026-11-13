pub(crate) fn text(language: u16) -> &'static str {
    match language & 0x03ff {
        0x04 if matches!(language, 0x0404 | 0x0c04 | 0x1404) => "acdesktop 正在控制此電腦",
        0x04 => "acdesktop 正在控制此电脑",
        0x07 => "acdesktop steuert diesen Computer",
        0x0a => "acdesktop está controlando este equipo",
        0x0c => "acdesktop contrôle cet ordinateur",
        0x10 => "acdesktop sta controllando questo computer",
        0x11 => "acdesktop がこのコンピューターを操作中です",
        0x12 => "acdesktop이 이 컴퓨터를 제어하고 있습니다",
        0x16 => "acdesktop está controlando este computador",
        _ => "acdesktop is controlling this computer",
    }
}

#[cfg(test)]
#[path = "notice_language_tests.rs"]
mod tests;
