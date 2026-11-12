//! 侧边栏组件 —— 图标 + 文字导航项 + 底部用户信息。

use gpui::{div, prelude::*, px, rgb, Context, FontWeight, IntoElement, Window};

use crate::data::NavItem;
use crate::theme::{
    BRAND, CONTENT_BORDER, SIDEBAR_BG, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY, TRANSPARENT,
};

/// 侧边栏状态
pub struct Sidebar {
    active: NavItem,
    items: Vec<NavItem>,
}

impl Sidebar {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            active: NavItem::Projects,
            items: vec![
                NavItem::Projects,
                NavItem::Editors,
                NavItem::Templates,
                NavItem::AssetStore,
                NavItem::Community,
                NavItem::Settings,
            ],
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(px(200.0))
            .h_full()
            .bg(SIDEBAR_BG)
            .border_r_1()
            .border_color(CONTENT_BORDER)
            .child(self.render_brand())
            .child(self.render_section_label("主导航"))
            .children(self.items.iter().map(|item| self.render_nav_item(*item)))
            .child(self.render_spacer())
            .child(self.render_section_label("帮助"))
            .child(self.render_spacer())
            .child(self.render_user_info())
    }
}

impl Sidebar {
    fn render_brand(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .px_6()
            .py_5()
            .child(
                div()
                    .size_9()
                    .bg(BRAND)
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("M"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::BOLD)
                            .text_color(TEXT_PRIMARY)
                            .child("MyGo"),
                    )
                    .child(div().text_xs().text_color(TEXT_MUTED).child("版本 1.0")),
            )
    }

    fn render_section_label(&self, title: &'static str) -> impl IntoElement {
        div()
            .px_6()
            .pt_4()
            .pb_2()
            .text_xs()
            .text_color(TEXT_MUTED)
            .child(title.to_string())
    }

    fn render_spacer(&self) -> impl IntoElement {
        div().flex_1()
    }

    fn render_nav_item(&self, item: NavItem) -> impl IntoElement {
        let is_active = item == self.active;
        let icon_char = self.nav_icon(item);

        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .mx_3()
            .px_3()
            .py_2p5()
            .rounded(px(6.0))
            .bg(if is_active { BRAND } else { TRANSPARENT })
            .hover(move |s| if !is_active { s.bg(rgb(0x2d2e32)) } else { s })
            .child(
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .text_sm()
                    .text_color(if is_active {
                        rgb(0xffffff)
                    } else {
                        TEXT_SECONDARY
                    })
                    .child(icon_char),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if is_active {
                        rgb(0xffffff)
                    } else {
                        TEXT_PRIMARY
                    })
                    .child(item.label()),
            )
    }

    fn render_user_info(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .mx_3()
            .px_3()
            .py_3()
            .bg(rgb(0x2a2b2f))
            .rounded(px(6.0))
            .border_t_1()
            .border_color(rgb(0x35363a))
            .child(
                div()
                    .size_8()
                    .bg(rgb(0x6366f1))
                    .rounded(px(6.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0xffffff))
                    .child("G"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(TEXT_PRIMARY)
                            .child("个人版"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(TEXT_MUTED)
                            .child("gaojian@mygo.dev"),
                    ),
            )
    }

    fn nav_icon(&self, item: NavItem) -> &'static str {
        match item {
            NavItem::Projects => "▦",
            NavItem::Editors => "⬇",
            NavItem::Templates => "▤",
            NavItem::AssetStore => "◎",
            NavItem::Community => "◉",
            NavItem::Settings => "⚙",
        }
    }
}
