//! MyGo GPUI —— 参考 Unity Hub / 团结引擎 Hub 风格的 Go 项目管理桌面应用。
#![allow(dead_code, unused_imports, unused_variables)]

mod data;
mod sidebar;
mod theme;

use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, Entity, FontWeight,
    IntoElement, SharedString, Window, WindowBounds, WindowOptions,
};

use crate::data::{sample_projects, NavItem, Project};
use crate::sidebar::Sidebar;
use crate::theme::{
    BRAND, BTN_SECONDARY, CONTENT_BG, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, WINDOW_BG,
};

/// 根视图 —— 包含侧边栏 + 主内容区
pub struct MyGoApp {
    current_nav: NavItem,
    projects: Vec<Project>,
    search_query: SharedString,
    sidebar: Entity<Sidebar>,
}

impl MyGoApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sidebar = cx.new(|cx| Sidebar::new(cx));
        Self {
            current_nav: NavItem::Projects,
            projects: sample_projects(),
            search_query: SharedString::from(""),
            sidebar,
        }
    }

    fn filtered_projects(&self) -> Vec<&Project> {
        let q = self.search_query.to_string().to_lowercase();
        self.projects
            .iter()
            .filter(|p| q.is_empty() || p.name.to_string().to_lowercase().contains(&q))
            .collect()
    }
}

impl Render for MyGoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .w_full()
            .h_full()
            .bg(WINDOW_BG)
            .child(self.sidebar.clone())
            .child(self.render_main_content(cx))
    }
}

impl MyGoApp {
    fn render_main_content(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let filtered = self.filtered_projects();
        let project_count = filtered.len();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .bg(CONTENT_BG)
            .child(self.render_header(project_count))
            .child(self.render_project_list(filtered))
    }

    fn render_header(&self, project_count: usize) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px_8()
            .py_5()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .text_color(TEXT_PRIMARY)
                            .child(self.current_nav.label()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(TEXT_SECONDARY)
                            .child(format!("共 {} 个项目", project_count)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(BTN_SECONDARY)
                            .text_color(TEXT_PRIMARY)
                            .hover(|s| s.bg(rgb(0x45464b)))
                            .child("搜索项目"),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(BTN_SECONDARY)
                            .text_color(TEXT_PRIMARY)
                            .hover(|s| s.bg(rgb(0x45464b)))
                            .child("刷新"),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(BTN_SECONDARY)
                            .text_color(TEXT_PRIMARY)
                            .hover(|s| s.bg(rgb(0x45464b)))
                            .child("添加项目..."),
                    )
                    .child(
                        div()
                            .px_5()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(BRAND)
                            .text_color(rgb(0xffffff))
                            .hover(|s| s.bg(rgb(0x66a0ff)))
                            .child("新建项目"),
                    ),
            )
    }

    fn render_project_list(&self, projects: Vec<&Project>) -> impl IntoElement {
        div().flex().flex_col().flex_1().px_8().pb_8().children(
            projects
                .into_iter()
                .map(|p| self.render_project_card(p.clone())),
        )
    }

    fn render_project_card(&self, p: Project) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_5()
            .px_5()
            .py_4()
            .w_full()
            .bg(rgb(0x2a2b2f))
            .rounded(px(8.0))
            .border_1()
            .border_color(rgb(0x35363a))
            .hover(|s| s.bg(rgb(0x323338)).border_color(rgb(0x45464b)))
            .child(self.render_project_icon())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(TEXT_PRIMARY)
                                    .child(p.name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(BRAND)
                                    .text_color(rgb(0xffffff))
                                    .child(p.engine_version.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(rgb(0x3a3b40))
                                    .text_color(TEXT_SECONDARY)
                                    .child(p.template.clone()),
                            ),
                    )
                    .child(div().text_sm().text_color(TEXT_MUTED).child(p.path.clone())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(TEXT_SECONDARY)
                            .child(self.render_git_label(p.git_branch.clone())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(TEXT_MUTED)
                            .child(format!("打开于 {}", p.last_opened)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(BRAND)
                            .text_color(rgb(0xffffff))
                            .hover(|s| s.bg(rgb(0x66a0ff)))
                            .child("打开"),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .rounded(px(6.0))
                            .bg(rgb(0x3a3b40))
                            .text_color(TEXT_SECONDARY)
                            .hover(|s| s.bg(rgb(0x45464b)).text_color(TEXT_PRIMARY))
                            .child("目录"),
                    ),
            )
    }

    fn render_project_icon(&self) -> impl IntoElement {
        div()
            .size_12()
            .rounded(px(8.0))
            .bg(BRAND)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xffffff))
                    .child("M"),
            )
    }

    fn render_git_label(&self, branch: Option<SharedString>) -> impl IntoElement {
        match branch {
            Some(b) => div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .text_color(rgb(0x4ade80))
                .child("GIT · ")
                .child(b),
            None => div().text_color(TEXT_MUTED).child("—"),
        }
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(780.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| MyGoApp::new(cx)),
        )
        .unwrap();
    });
}
