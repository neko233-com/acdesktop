//! MyGo 深色主题 —— 参考 Unity Hub / 团结引擎 Hub 的配色。
//! 整体风格：大方、优雅、得体。

use gpui::Rgba;

/// 辅助：将 0xRRGGBB 十六进制颜色转为 Rgba (alpha = 1.0)
const fn rgba(hex: u32) -> Rgba {
    let r = ((hex >> 16) & 0xff) as f32 / 255.0;
    let g = ((hex >> 8) & 0xff) as f32 / 255.0;
    let b = (hex & 0xff) as f32 / 255.0;
    Rgba { r, g, b, a: 1.0 }
}

/// 完全透明
const fn transparent() -> Rgba {
    Rgba { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }
}

/// 窗口背景 —— 深灰偏蓝黑
pub const WINDOW_BG: Rgba = rgba(0x1e1f22);

/// 侧边栏背景
pub const SIDEBAR_BG: Rgba = rgba(0x16171a);

/// 主内容区背景
pub const CONTENT_BG: Rgba = rgba(0x232428);

/// 卡片/列表项悬停背景
pub const HOVER_BG: Rgba = rgba(0x2d2e32);

/// 选中项背景
pub const ACTIVE_BG: Rgba = rgba(0x3a3b40);

/// 侧边栏分隔线
pub const SIDEBAR_BORDER: Rgba = rgba(0x2a2b2f);

/// 主分隔线
pub const CONTENT_BORDER: Rgba = rgba(0x35363a);

/// 主文字颜色
pub const TEXT_PRIMARY: Rgba = rgba(0xe4e4e7);

/// 次要文字颜色
pub const TEXT_SECONDARY: Rgba = rgba(0x9a9aa2);

/// 弱化文字（时间戳等）
pub const TEXT_MUTED: Rgba = rgba(0x6b6c72);

/// 品牌强调色 —— MyGo 蓝
pub const BRAND: Rgba = rgba(0x4a8dff);

/// 品牌强调色（悬停）
pub const BRAND_HOVER: Rgba = rgba(0x66a0ff);

/// 成功/绿色
pub const SUCCESS: Rgba = rgba(0x4ade80);

/// 警告/橙色
pub const WARNING: Rgba = rgba(0xf59e0b);

/// 错误/红色
pub const DANGER: Rgba = rgba(0xef4444);

/// 输入框背景
pub const INPUT_BG: Rgba = rgba(0x2a2b2f);

/// 按钮次要背景
pub const BTN_SECONDARY: Rgba = rgba(0x3a3b40);

/// 按钮次要背景悬停
pub const BTN_SECONDARY_HOVER: Rgba = rgba(0x45464b);

/// 边框/分割线弱色
pub const DIVIDER: Rgba = rgba(0x35363a);

/// 品牌深色（用于选中导航）
pub const BRAND_DARK: Rgba = rgba(0x3a6fd4);

/// 透明（同文件内引用）
pub const TRANSPARENT: Rgba = transparent();
