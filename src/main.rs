use gpui::*;
use gpui_component::input::{Input, InputState};
use gpui_component::select::{Select, SelectState};
use gpui_component::sidebar::{Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::tag::Tag;
use gpui_component::{button::*, *};
use std::sync::atomic::{AtomicUsize, Ordering};

fn next_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

struct Node {
    path: String,
    name: String,
    method: String,
    children: Vec<Node>,
    is_file: bool,
}

struct TabState {
    id: usize,
    name: String,
    path: String,
    method: Entity<SelectState<Vec<String>>>,
    url: Entity<InputState>,
    pending: bool,
}

struct ApiClient {
    nodes: Vec<Node>,
    tabs: Vec<TabState>,
    active_tab: Option<usize>,
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
        };

        let tab = this.new_tab(window, cx, "get_req");
        this.active_tab = Some(tab.id);
        this.tabs.push(tab);

        this
    }

    fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>, name: &str) -> TabState {
        let id = next_id();
        let url = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));
        let method = cx.new(|cx| {
            SelectState::new(
                vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                Some(IndexPath::default()),
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
            pending: false,
        }
    }

    fn build_method_tag(method: &str) -> impl IntoElement {
        match method {
            "GET" => Tag::color(ColorName::Green).outline().child("GET"),

            "POST" => Tag::color(ColorName::Blue).outline().child("POST"),

            "PUT" => Tag::color(ColorName::Yellow).outline().child("PUT"),

            "PATCH" => Tag::color(ColorName::Orange).outline().child("PATCH"),

            "DELETE" => Tag::color(ColorName::Red).outline().child("DELETE"),

            "HEAD" => Tag::color(ColorName::Purple).outline().child("HEAD"),

            "OPTIONS" => Tag::color(ColorName::Gray).outline().child("OPTIONS"),

            _ => Tag::color(ColorName::Neutral).outline().child("Nan"),
        }
    }

    fn render_node(node: &Node) -> SidebarMenuItem {
        let is_file = node.is_file;
        let method = node.method.clone();

        let item = SidebarMenuItem::new(node.name.clone()).suffix(move |_, _| {
            if is_file {
                div().child(Self::build_method_tag(method.clone().as_str()))
            } else {
                div()
            }
        });

        if node.children.is_empty() {
            item
        } else {
            item.children(node.children.iter().map(Self::render_node))
        }
    }

    fn render_sidebar(&self, _cx: &Context<Self>) -> impl IntoElement {
        Sidebar::new("api-sidebar")
            .header(
                SidebarHeader::new()
                    .child(h_flex().gap_2().child(IconName::Folder).child("Workspace")),
            )
            .child(
                SidebarGroup::new("Explorer")
                    .child(SidebarMenu::new().children(self.nodes.iter().map(Self::render_node))),
            )
    }

    fn render_new_tab_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("add-tab")
            .label("Add Tab")
            .secondary()
            .on_click(cx.listener(|this: &mut ApiClient, _event, window, cx| {
                let tab = this.new_tab(window, cx, "new");
                this.active_tab = Some(tab.id);
                this.tabs.push(tab);
                cx.notify();
            }))
    }

    fn editor_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            .children(self.tabs.iter().map(|tab| {
                let tab_id = tab.id;
                Tab::default()
                    .suffix(
                        Button::new("back")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(
                                move |this: &mut ApiClient, _: &ClickEvent, _window, cx| {
                                    this.tabs.retain(|t| t.id != tab_id);
                                    this.active_tab = this.tabs.last().map(|t| t.id);
                                    cx.notify()
                                },
                            )),
                    )
                    .label(tab.name.clone())
                    .large()
                    .outline()
            }))
    }

    fn render_editor(&self, _cx: &Context<Self>) -> impl IntoElement {
        let Some(tab) = self
            .active_tab
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
        else {
            return div().child("No tab open");
        };

        h_flex()
            .gap_2()
            .child(Select::new(&tab.method))
            .child(Input::new(&tab.url))
            .child(Button::new("send").primary().label("Send"))
            .into()
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .child(self.render_sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .v_flex()
                    .child(self.editor_header(cx))
                    .child(self.render_editor(cx))
                    .child(self.render_new_tab_button(cx)),
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
