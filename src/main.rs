use gpui::*;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::table::{
    Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow,
};
use gpui_component::tag::Tag;
use gpui_component::theme::{Theme, ThemeRegistry};
use gpui_component::{ActiveTheme as _, button::*, *};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn next_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone)]
struct Node {
    path: String,
    name: String,
    method: String,
    children: Vec<Node>,
    is_file: bool,
}

struct QueryParamState {
    key: Entity<InputState>,
    value: Entity<InputState>,
    active: bool,
}

struct TabState {
    id: usize,
    name: String,
    path: String,
    method: Entity<SelectState<Vec<String>>>,
    url: Entity<InputState>,
    query_params: Vec<Entity<QueryParamState>>,
    pending: bool,
}

struct ApiClient {
    nodes: Vec<Node>,
    tabs: Vec<TabState>,
    active_tab: Option<usize>,
    scroll_handle: ScrollHandle,
}

impl ApiClient {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let nodes = vec![Node {
            path: "/api".into(),
            name: "API Client".into(),
            method: String::new(),
            is_file: false,
            children: vec![
                Node {
                    path: "/api/get_users".into(),
                    name: "Get Users".into(),
                    method: "GET".into(),
                    is_file: true,
                    children: vec![],
                },
                Node {
                    path: "/api/user".into(),
                    name: "Create User".into(),
                    method: "POST".into(),
                    is_file: false,
                    children: vec![Node {
                        path: "/api/user/create".into(),
                        name: "Create User".into(),
                        method: "GET".into(),
                        is_file: true,
                        children: vec![],
                    }],
                },
                Node {
                    path: "/api/update_user".into(),
                    name: "Update User".into(),
                    method: "PUT".into(),
                    is_file: true,
                    children: vec![],
                },
            ],
        }];

        let mut this = Self {
            nodes,
            tabs: Vec::new(),
            active_tab: None,
            scroll_handle: ScrollHandle::new(),
        };

        let tab = this.new_tab(window, cx, "get_req", "GET".to_string());
        this.active_tab = Some(tab.id);
        this.tabs.push(tab);

        this
    }

    fn new_tab(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        name: &str,
        method: String,
    ) -> TabState {
        let id = next_id();
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));
        let methods: Vec<String> = vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
            .into_iter()
            .map(String::from)
            .collect();
        let selected_method = methods.iter().position(|m| *m == method).unwrap_or(0);
        let method = cx.new(|cx| {
            SelectState::new(
                methods,
                Some(IndexPath {
                    section: 0,
                    row: selected_method,
                    column: 0,
                }),
                window,
                cx,
            )
        });

        TabState {
            id,
            name: name.into(),
            path: String::new(),
            method,
            url,
            query_params: vec![],
            pending: false,
        }
    }

    fn build_method_tag(method: &str) -> impl IntoElement {
        match method {
            "GET" => Tag::color(ColorName::Green).outline().child("GET").xsmall(),

            "POST" => Tag::color(ColorName::Blue).outline().child("POST").xsmall(),

            "PUT" => Tag::color(ColorName::Yellow)
                .outline()
                .child("PUT")
                .xsmall(),

            "PATCH" => Tag::color(ColorName::Orange)
                .outline()
                .child("PATCH")
                .xsmall(),

            "DELETE" => Tag::color(ColorName::Red)
                .outline()
                .child("DELETE")
                .xsmall(),

            "HEAD" => Tag::color(ColorName::Purple)
                .outline()
                .child("HEAD")
                .xsmall(),

            "OPTIONS" => Tag::color(ColorName::Gray)
                .outline()
                .child("OPTIONS")
                .xsmall(),

            _ => Tag::color(ColorName::Neutral)
                .outline()
                .child("Nan")
                .xsmall(),
        }
    }

    fn render_node(&self, node: &Node, cx: &mut Context<Self>) -> SidebarMenuItem {
        let is_file = node.is_file;
        let name = node.name.clone();
        let method = node.method.clone();

        let method_for_suffix = method.clone();
        let method_for_click = method.clone();
        let name_for_click = name.clone();

        let mut item = SidebarMenuItem::new(name.clone()).suffix(move |_, _| {
            if is_file {
                div().child(Self::build_method_tag(method_for_suffix.as_str()))
            } else {
                div()
            }
        });

        if is_file {
            item = item.on_click(
                cx.listener(move |this: &mut ApiClient, _event, window, cx| {
                    let tab = this.new_tab(window, cx, &name_for_click, method_for_click.clone());
                    this.active_tab = Some(tab.id);
                    this.tabs.push(tab);
                    cx.notify();
                }),
            );
        }

        if node.children.is_empty() {
            item
        } else {
            let mut children = Vec::new();

            for child in &node.children {
                children.push(self.render_node(child, cx));
            }

            item.children(children)
        }
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Sidebar::new("api-sidebar")
            .header(
                SidebarHeader::new().child(
                    h_flex()
                        .gap(rems(0.75))
                        .child(IconName::Palette)
                        .child(div().flex_1().child("workspace")),
                ),
            )
            .child(
                SidebarGroup::new("Explorer").child(
                    SidebarMenu::new().children(
                        self.nodes
                            .iter()
                            .map(|child| Self::render_node(&self, child, cx)),
                    ),
                ),
            )
    }

    fn render_new_tab_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .border_l_1()
            .border_color(cx.theme().border)
            .h_full()
            .px_2()
            .items_center()
            .justify_center()
            .child(
                Button::new("add-tab")
                    .ghost()
                    .small()
                    .icon(IconName::Plus)
                    .on_click(cx.listener(|this: &mut ApiClient, _event, window, cx| {
                        let tab = this.new_tab(window, cx, "new", "GET".to_string());
                        this.active_tab = Some(tab.id);
                        this.tabs.push(tab);
                        cx.notify();
                    })),
            )
    }

    fn render_tab(&self, cx: &mut Context<Self>, tab: &TabState) -> Tab {
        let id = tab.id;
        let name = tab.name.clone();

        let method = tab
            .method
            .read(cx)
            .selected_value()
            .map(String::as_str)
            .unwrap_or("");

        Tab::default()
            .px_1()
            .with_variant(tab::TabVariant::Tab)
            .prefix(div().mr_1().child(Self::build_method_tag(method)))
            .label(name)
            .suffix(
                h_flex().child(
                    Button::new(("close-tab", id))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Close)
                        .on_click(cx.listener(
                            move |this: &mut ApiClient, _: &ClickEvent, _window, cx| {
                                this.tabs.retain(|t| t.id != id);
                                this.active_tab = this.tabs.last().map(|t| t.id);
                                cx.notify();
                            },
                        )),
                ),
            )
    }
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self
            .active_tab
            .and_then(|id| self.tabs.iter().position(|t| t.id == id))
            .unwrap_or(0);

        TabBar::new("tabs")
            .selected_index(selected)
            .on_click(
                cx.listener(move |this: &mut ApiClient, idx: &usize, _window, cx| {
                    if let Some(tab) = this.tabs.get(*idx) {
                        this.active_tab = Some(tab.id);
                        cx.notify();
                    }
                }),
            )
            .track_scroll(&self.scroll_handle)
            .suffix(self.render_new_tab_button(cx))
            .children(self.tabs.iter().map(|tab| Self::render_tab(&self, cx, tab)))
    }

    fn render_editor(&self, _cx: &Context<Self>) -> impl IntoElement {
        let Some(tab) = self
            .active_tab
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
        else {
            return div().child("No tab open");
        };

        h_flex()
            .w_full()
            .gap(rems(0.5))
            .child(div().w(px(110.)).child(Select::new(&tab.method)))
            .child(div().flex_1().child(Input::new(&tab.url)))
            .child(Button::new("send").primary().label("Send"))
    }

    fn new_query_param(&mut self, window: &mut Window, cx: &mut Context<Self>, tab_id: usize) {
        let qp = cx.new(|cx| QueryParamState {
            key: cx.new(|cx| InputState::new(window, cx)),
            value: cx.new(|cx| InputState::new(window, cx)),
            active: true,
        });
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.query_params.push(qp);
        }
    }

    fn render_query_params_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(tab) = self
            .active_tab
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
        else {
            return div();
        };
        let tab_id = tab.id;

        v_flex()
            .gap(rems(1.))
            .child(
                h_flex()
                    .items_center()
                    .gap(rems(0.5))
                    .child(div().flex_1().child(TableCaption::new().child("Query Parameters")))
                    .child(
                        Button::new("add-qp")
                            .label("Add Query")
                            .ghost()
                            .small()
                            .on_click(
                                cx.listener(move |this: &mut ApiClient, _event, window, cx| {
                                    this.new_query_param(window, cx, tab_id);
                                    cx.notify();
                                }),
                            ),
                    ),
            )
            .child(
                Table::new()
                    .child(
                        TableHeader::new().child(
                            TableRow::new()
                                .child(TableHead::new().child(""))
                                .child(TableHead::new().child("Key"))
                                .child(TableHead::new().child("Value")),
                        ),
                    )
                    .child(
                        TableBody::new().children(tab.query_params.iter().enumerate().map(
                            |(i, entity)| {
                                let entity = entity.clone();
                                let (key, value, active) = {
                                    let state = entity.read(cx);
                                    (state.key.clone(), state.value.clone(), state.active)
                                };

                                TableRow::new()
                                    .child(TableCell::new().child(
                                        Checkbox::new(format!("qp-{i}")).checked(active).on_click({
                                            let entity = entity.clone();
                                            cx.listener(move |_this: &mut ApiClient, checked: &bool, _window, cx| {
                                                entity.update(cx, |qp, _cx| qp.active = *checked);
                                                cx.notify();
                                            })
                                        }),
                                    ))
                                    .child(TableCell::new().child(Input::new(&key)))
                                    .child(TableCell::new().child(Input::new(&value)))
                                    .child(
                                        TableCell::new().child(
                                            Button::new("del")
                                                .ghost()
                                                .xsmall()
                                                .icon(IconName::Delete)
                                                .on_click({
                                                    let entity = entity.clone();
                                                    cx.listener(move |this: &mut ApiClient, _: &ClickEvent, _window, cx| {
                                                        if let Some(target_tab) =
                                                            this.tabs.iter_mut().find(|t| t.id == tab_id)
                                                        {
                                                            target_tab
                                                                .query_params
                                                                .retain(|q| q.entity_id() != entity.entity_id());
                                                        }
                                                        cx.notify();
                                                    })
                                                }),
                                        ),
                                    )
                            },
                        )),
                    ),
            )
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .child(div().w(px(256.)).h_full().child(self.render_sidebar(cx)))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .v_flex()
                    .gap(rems(1.))
                    .overflow_y_scrollbar()
                    .child(
                        div()
                            .flex_none()
                            .overflow_x_hidden()
                            .child(self.render_tab_bar(cx)),
                    )
                    .child(div().px(px(24.)).child(self.render_editor(cx)))
                    .child(
                        div()
                            .px(px(24.))
                            .pb(px(24.))
                            .child(self.render_query_params_section(cx)),
                    ),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|view_cx| ApiClient::new(window, view_cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
