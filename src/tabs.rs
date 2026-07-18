use crate::ApiClient;
use crate::helpers::build_method_tag;
use gpui::*;
use gpui_component::input::InputEvent;
use gpui_component::select::{SelectEvent, SelectState};
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::{ActiveTheme as _, button::*, *};

pub fn new_tab(window: &mut Window, cx: &mut Context<ApiClient>) -> Entity<crate::Node> {
    let name_entity =
        cx.new(|cx| gpui_component::input::InputState::new(window, cx).default_value("Untitled"));
    let url =
        cx.new(|cx| gpui_component::input::InputState::new(window, cx).placeholder("Enter URL..."));
    let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .into_iter()
        .map(String::from)
        .collect();
    let method = cx.new(|cx| {
        SelectState::new(
            methods,
            Some(IndexPath {
                section: 0,
                row: 0,
                column: 0,
            }),
            window,
            cx,
        )
    });
    let response_panel = cx.new(|cx| {
        gpui_component::input::InputState::new(window, cx)
            .code_editor("json")
            .line_number(true)
            .default_value("")
    });

    cx.new(|cx| crate::Node {
        name: name_entity,
        path: String::new(),
        is_file: true,
        children: vec![],
        node_type: crate::NodeType::File {
            method,
            url,
            pending: false,
            dirty: false,
            selected_editor_config: 0,
            response_panel: Some(response_panel),
            show_response_panel: false,
            query_params: vec![],
        },
    })
}

pub fn render_editor_config(api: &mut ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
    let selected = api
        .active_tab_index
        .and_then(|i| api.tabs.get(i))
        .map(|tab| tab.read(cx).selected_editor_config())
        .unwrap_or(0);

    div()
        .w_full()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            div().px(px(24.)).child(
                TabBar::new("request-tabs")
                    .w_full()
                    .with_variant(tab::TabVariant::Underline)
                    .selected_index(selected)
                    .child(Tab::new().label("Params"))
                    .child(Tab::new().label("Authorization"))
                    .child(Tab::new().label("Headers"))
                    .child(Tab::new().label("Body"))
                    .child(Tab::new().label("Settings"))
                    .on_click(cx.listener(
                        move |this: &mut ApiClient, idx: &usize, _window, cx| {
                            if let Some(tab) = this.active_tab_index.and_then(|i| this.tabs.get(i))
                            {
                                tab.update(cx, |node, _cx| {
                                    node.set_selected_editor_config(*idx);
                                });
                            }
                            cx.notify();
                        },
                    )),
            ),
        )
}

pub fn render_new_tab_button(_api: &ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
    h_flex()
        .h_full()
        .items_center()
        .justify_center()
        .px_2()
        .child(
            Button::new("add-tab")
                .ghost()
                .xsmall()
                .icon(IconName::Plus)
                .tooltip("Add Tab")
                .on_click(cx.listener(|this: &mut ApiClient, _event, window, cx| {
                    let node = new_tab(window, cx);
                    // let node = setup_tab(node, window, cx);
                    this.tabs.push(node);
                    this.active_tab_index = Some(this.tabs.len() - 1);
                    cx.notify();
                })),
        )
}

pub fn render_tab(_api: &ApiClient, cx: &mut Context<ApiClient>, tab: Entity<crate::Node>) -> Tab {
    let tab_to_close = tab.clone();
    let tab_state = tab.read(cx);
    let name = tab_state.name.read(cx).value().to_string();
    let method_str = tab_state.method_value(cx);
    let close_id = tab_to_close.entity_id();

    Tab::default()
        .px_1()
        .prefix(div().mr_1().child(build_method_tag(&method_str)))
        .label(name)
        .suffix(
            Button::new(("close-tab", close_id))
                .ghost()
                .xsmall()
                .icon(IconName::Close)
                .on_click(
                    cx.listener(move |this: &mut ApiClient, _: &ClickEvent, _window, cx| {
                        let close_id = tab_to_close.entity_id();
                        this.tabs.retain(|t| t.entity_id() != close_id);
                        this.active_tab_index = if this.tabs.is_empty() {
                            None
                        } else {
                            Some(this.tabs.len() - 1)
                        };
                        cx.notify();
                    }),
                ),
        )
}

pub fn render_tab_bar(api: &ApiClient, cx: &mut Context<ApiClient>) -> impl IntoElement {
    let selected = api.active_tab_index.unwrap_or(0);

    let sidebar_collapsed = api.sidebar_collapsed;

    TabBar::new("tabs")
        .min_h(px(32.))
        .prefix(
            h_flex().px(px(8.)).items_center().child(
                SidebarToggleButton::new()
                    .collapsed(sidebar_collapsed)
                    .on_click(cx.listener(move |this: &mut ApiClient, _, _window, cx| {
                        this.sidebar_collapsed = !this.sidebar_collapsed;
                        cx.notify();
                    })),
            ),
        )
        .selected_index(selected)
        .on_click(
            cx.listener(move |this: &mut ApiClient, idx: &usize, _window, cx| {
                if *idx < this.tabs.len() {
                    this.active_tab_index = Some(*idx);
                    cx.notify();
                }
            }),
        )
        .track_scroll(&api.scroll_handle)
        .suffix(render_new_tab_button(api, cx))
        .children(api.tabs.iter().map(|tab| render_tab(api, cx, tab.clone())))
}
