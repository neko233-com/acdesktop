//! 模拟数据 —— 项目列表、编辑器版本等。

use gpui::SharedString;

/// 一个项目记录
#[derive(Clone)]
pub struct Project {
    pub name: SharedString,
    pub engine_version: SharedString,
    pub template: SharedString,
    pub path: SharedString,
    pub last_opened: SharedString,
    pub git_branch: Option<SharedString>,
    pub starred: bool,
}

/// 一个可用的编辑器版本
#[derive(Clone)]
pub struct EditorVersion {
    pub version: SharedString,
    pub release_type: ReleaseType,
    pub release_notes: SharedString,
    pub recommended: bool,
}

/// 发布类型
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReleaseType {
    Stable,
    Beta,
    Alpha,
}

impl ReleaseType {
    pub fn label(self) -> &'static str {
        match self {
            ReleaseType::Stable => "正式发行",
            ReleaseType::Beta => "预览版",
            ReleaseType::Alpha => "存响",
        }
    }
}

/// 导航侧边栏条目
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Projects,
    Editors,
    Templates,
    AssetStore,
    Community,
    Settings,
}

impl NavItem {
    pub fn label(self) -> &'static str {
        match self {
            NavItem::Projects => "项目",
            NavItem::Editors => "安装",
            NavItem::Templates => "模板",
            NavItem::AssetStore => "资源商店",
            NavItem::Community => "社区",
            NavItem::Settings => "设置",
        }
    }
}

/// 示例项目数据
pub fn sample_projects() -> Vec<Project> {
    vec![
        Project {
            name: "tuantjie-project-sf".into(),
            engine_version: "1.11.0".into(),
            template: "URP".into(),
            path: "D:\\Code\\Projects\\tuantjie-project-sf".into(),
            last_opened: "13 分钟前".into(),
            git_branch: Some("main".into()),
            starred: false,
        },
        Project {
            name: "HotReload233.Validation".into(),
            engine_version: "1.9.2".into(),
            template: "URP".into(),
            path: "D:\\Code\\Projects\\HotReload233\\Validation-HotR".into(),
            last_opened: "6 天前".into(),
            git_branch: None,
            starred: false,
        },
        Project {
            name: "unity-xixuegui-net".into(),
            engine_version: "7000.0.0a16".into(),
            template: "URP".into(),
            path: "D:\\Code\\Poko\\Projects\\unity-xixuegui-net".into(),
            last_opened: "2026年1月6日".into(),
            git_branch: Some("main".into()),
            starred: true,
        },
        Project {
            name: "unity-project-mr".into(),
            engine_version: "2022.3.5f1".into(),
            template: "URP".into(),
            path: "D:\\Code\\Unity\\Projects\\unity-project-mr".into(),
            last_opened: "2026年2月10日".into(),
            git_branch: Some("main".into()),
            starred: false,
        },
        Project {
            name: "mygo-adventure".into(),
            engine_version: "1.10.3".into(),
            template: "HDRP".into(),
            path: "C:\\Users\\dev\\Documents\\MyGoProjects\\mygo-adventure".into(),
            last_opened: "昨天".into(),
            git_branch: Some("feature/world".into()),
            starred: true,
        },
    ]
}

/// 示例编辑器版本数据
pub fn sample_editor_versions() -> Vec<EditorVersion> {
    vec![
        EditorVersion {
            version: "1.11.0".into(),
            release_type: ReleaseType::Stable,
            release_notes: "当前最新稳定版".into(),
            recommended: true,
        },
        EditorVersion {
            version: "1.10.3".into(),
            release_type: ReleaseType::Stable,
            release_notes: "上一个稳定版".into(),
            recommended: false,
        },
        EditorVersion {
            version: "1.12.0b1".into(),
            release_type: ReleaseType::Beta,
            release_notes: "包含新的 SRP 改进".into(),
            recommended: false,
        },
        EditorVersion {
            version: "1.11.1a2".into(),
            release_type: ReleaseType::Alpha,
            release_notes: "实验性功能预览".into(),
            recommended: false,
        },
    ]
}
