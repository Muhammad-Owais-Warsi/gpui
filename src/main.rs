mod actions;
mod fs;
mod helpers;
mod http;
// mod http;
mod query_params;
mod tabs;
use crate::helpers::{build_method_tag, next_id, read_dir_to_nodes};
use crate::http::send_get;
use crate::query_params::query_params_from_json;
use crate::tabs::{render_editor_config, render_tab_bar};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::Theme;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::resizable::{resizable_panel, v_resizable};
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectEvent, SelectState};
use gpui_component::sidebar::{
    Sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem,
};
use gpui_component::{button::*, *};
use std::path::PathBuf;

#[derive(Clone)]
struct Workspace {
    name: String,
    path: String,
    nodes: Vec<Entity<Node>>,
}

#[derive(Clone)]
pub struct NodeData {
    path: String,
    name: String,
    method: String,
    children: Vec<NodeData>,
    is_file: bool,
}

#[derive(Clone)]
enum NodeType {
    File {
        method: Entity<SelectState<Vec<String>>>,
        url: Entity<InputState>,
        pending: bool,
        dirty: bool,
        selected_editor_config: usize,
        response_panel: Option<Entity<InputState>>,
        show_response_panel: bool,
        query_params: Vec<Entity<query_params::QueryParams>>,
    },

    Folder,
}

#[derive(Clone)]
pub struct Node {
    pub path: String,
    pub is_file: bool,
    pub children: Vec<Entity<Node>>,
    name: Entity<InputState>,
    node_type: NodeType,
}

impl Node {
    pub fn file_method(&self) -> Option<&Entity<SelectState<Vec<String>>>> {
        match &self.node_type {
            NodeType::File { method, .. } => Some(method),
            _ => None,
        }
    }

    pub fn file_url(&self) -> Option<&Entity<InputState>> {
        match &self.node_type {
            NodeType::File { url, .. } => Some(url),
            _ => None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        matches!(&self.node_type, NodeType::File { dirty: true, .. })
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        if let NodeType::File { dirty: d, .. } = &mut self.node_type {
            *d = dirty;
        }
    }

    pub fn show_response_panel(&self) -> bool {
        matches!(
            &self.node_type,
            NodeType::File {
                show_response_panel: true,
                ..
            }
        )
    }

    pub fn set_show_response_panel(&mut self, show: bool) {
        if let NodeType::File {
            show_response_panel,
            ..
        } = &mut self.node_type
        {
            *show_response_panel = show;
        }
    }

    pub fn response_panel(&self) -> Option<&Entity<InputState>> {
        match &self.node_type {
            NodeType::File { response_panel, .. } => response_panel.as_ref(),
            _ => None,
        }
    }

    pub fn selected_editor_config(&self) -> usize {
        match &self.node_type {
            NodeType::File {
                selected_editor_config,
                ..
            } => *selected_editor_config,
            _ => 0,
        }
    }

    pub fn set_selected_editor_config(&mut self, config: usize) {
        if let NodeType::File {
            selected_editor_config,
            ..
        } = &mut self.node_type
        {
            *selected_editor_config = config;
        }
    }

    pub fn query_params(&self) -> &[Entity<query_params::QueryParams>] {
        match &self.node_type {
            NodeType::File { query_params, .. } => query_params,
            _ => &[],
        }
    }

    pub fn query_params_mut(&mut self) -> &mut Vec<Entity<query_params::QueryParams>> {
        match &mut self.node_type {
            NodeType::File { query_params, .. } => query_params,
            _ => panic!("query_params called on non-file node"),
        }
    }

    pub fn method_value(&self, cx: &App) -> String {
        self.file_method()
            .and_then(|m| m.read(cx).selected_value().map(String::from))
            .unwrap_or_default()
    }
}

pub(crate) struct ApiClient {
    pub(crate) workspaces: Vec<Workspace>,
    pub(crate) selected_workspace: String,
    pub(crate) tabs: Vec<Entity<Node>>,
    pub(crate) active_tab_index: Option<usize>,
    pub(crate) scroll_handle: ScrollHandle,
    pub(crate) theme: Entity<SelectState<Vec<SharedString>>>,
    pub(crate) sidebar_collapsed: bool,
}

impl ApiClient {
    fn find_node(&self, path: &str, cx: &App) -> Option<Entity<Node>> {
        for ws in &self.workspaces {
            if let Some(found) = Self::find_in_nodes(&ws.nodes, path, cx) {
                return Some(found);
            }
        }
        None
    }

    fn find_in_nodes(nodes: &[Entity<Node>], path: &str, cx: &App) -> Option<Entity<Node>> {
        for node in nodes {
            if node.read(cx).path == path {
                return Some(node.clone());
            }
            let children = node.read(cx).children.clone();
            if let Some(found) = Self::find_in_nodes(&children, path, cx) {
                return Some(found);
            }
        }
        None
    }

    pub fn update_node_method_in_nodes(
        nodes: &[Entity<Node>],
        path: &str,
        _method: &str,
        cx: &mut App,
    ) -> bool {
        for node in nodes {
            if node.read(cx).path == path {
                return true;
            }
            let children = node.read(cx).children.clone();
            if Self::update_node_method_in_nodes(&children, path, _method, cx) {
                return true;
            }
        }
        false
    }

    fn new(window: &mut Window, cx: &mut Context<Self>, default_theme: SharedString) -> Self {
        let themes: Vec<SharedString> =
            ThemeRegistry::global(cx).themes().keys().cloned().collect();

        let default_theme_idx = themes.iter().position(|t| *t == default_theme).unwrap_or(0);

        let theme = cx.new(|cx| {
            SelectState::new(
                themes,
                Some(IndexPath {
                    section: 0,
                    row: default_theme_idx,
                    column: 0,
                }),
                window,
                cx,
            )
        });

        cx.subscribe_in(&theme, window, |_, _, event, _window, cx| {
            if let SelectEvent::Confirm(Some(name)) = event {
                let registry = ThemeRegistry::global(cx);
                if let Some(theme_config) = registry.themes().get(name).cloned() {
                    let mode = theme_config.mode;
                    let theme = Theme::global_mut(cx);
                    if mode.is_dark() {
                        theme.dark_theme = theme_config;
                    } else {
                        theme.light_theme = theme_config;
                    }

                    Theme::change(mode, None, cx);
                    cx.refresh_windows();
                }
            }
        })
        .detach();

        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let workspace_path = home.join("projects").join("react-app");
        let raw_nodes = read_dir_to_nodes(&workspace_path);
        let workspace = Workspace {
            name: "react-app".into(),
            path: workspace_path.to_string_lossy().to_string(),
            nodes: Self::build_nodes(window, cx, raw_nodes),
        };

        // let selected_workspace = workspace.clone();

        Self {
            workspaces: vec![workspace],
            selected_workspace: workspace_path.to_string_lossy().to_string(), // for now hardcoded
            tabs: Vec::new(),
            active_tab_index: None,
            scroll_handle: ScrollHandle::new(),
            theme,
            sidebar_collapsed: false,
        }
    }

    fn build_nodes(
        window: &mut Window,
        cx: &mut Context<Self>,
        nodes: Vec<NodeData>,
    ) -> Vec<Entity<Node>> {
        nodes
            .into_iter()
            .map(|n| {
                let children = Self::build_nodes(window, cx, n.children);
                let name_entity = cx.new(|cx| InputState::new(window, cx).default_value(&n.name));

                let node_type = if n.is_file {
                    let methods: Vec<String> =
                        vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                            .into_iter()
                            .map(String::from)
                            .collect();
                    let selected_method = methods.iter().position(|m| *m == n.method).unwrap_or(0);

                    NodeType::File {
                        method: cx.new(|cx| {
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
                        }),
                        url: cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL...")),
                        pending: false,
                        dirty: false,
                        selected_editor_config: 0,
                        response_panel: Some(cx.new(|cx| {
                            InputState::new(window, cx)
                                .code_editor("json")
                                .line_number(true)
                                .default_value("")
                        })),

                        show_response_panel: false,
                        query_params: vec![],
                    }
                } else {
                    NodeType::Folder
                };

                let node = cx.new(|_cx| Node {
                    name: name_entity,
                    path: n.path,
                    is_file: n.is_file,
                    children,
                    node_type,
                });

                if n.is_file.clone() {
                    let node_clone = node.clone();
                    cx.subscribe_in(
                        &node_clone.read(cx).file_url().cloned().expect("file node"),
                        window,
                        move |_this: &mut ApiClient, _, event, _window, cx| {
                            if let InputEvent::Change = event {
                                let node_clone = node_clone.clone();
                                cx.defer(move |cx| {
                                    node_clone.update(cx, |node, _cx| {
                                        node.set_dirty(true);
                                    });
                                });
                            }
                        },
                    )
                    .detach();

                    let node_clone = node.clone();
                    cx.subscribe_in(
                        &node_clone
                            .read(cx)
                            .file_method()
                            .cloned()
                            .expect("file node"),
                        window,
                        move |_this: &mut ApiClient, _, event, _window, cx| {
                            if let SelectEvent::Confirm(Some(new_method)) = event {
                                // let new_method = new_method.clone();
                                let node_clone = node_clone.clone();
                                cx.defer(move |cx| {
                                    node_clone.update(cx, |node, cx| {
                                        node.set_dirty(true);
                                        cx.notify();
                                    });
                                });
                            }
                        },
                    )
                    .detach();
                }

                node
            })
            .collect()
    }

    fn render_node(
        &self,
        node: &Entity<Node>,
        cx: &mut Context<Self>,
        active_path: Option<&String>,
    ) -> SidebarMenuItem {
        let node_state = node.read(cx);
        let is_file = node_state.is_file;
        let name = node_state.name.read(cx).value().to_string();
        let path = node_state.path.clone();
        let method_str = node_state.method_value(cx);

        let method_for_suffix = method_str.clone();
        let name_for_click = name.clone();
        let path_for_click = path.clone();
        let node_for_tab = node.clone();

        let mut item = SidebarMenuItem::new(name.clone())
            .suffix(move |_, _| {
                if is_file {
                    div().child(build_method_tag(&method_for_suffix))
                } else {
                    div()
                }
            })
            .active(active_path == Some(&path));

        if !is_file {
            item = item.context_menu(move |menu, _window, _cx| {
                menu.menu(
                    "Create File",
                    Box::new(actions::CreateFile {
                        parent: path.clone(),
                    }),
                )
            });
        }

        if is_file {
            item = item.on_click(
                cx.listener(move |this: &mut ApiClient, _event, window, cx| {
                    if let Some(idx) = this
                        .tabs
                        .iter()
                        .position(|t| t.read(cx).path == path_for_click)
                    {
                        this.active_tab_index = Some(idx);
                        cx.notify();
                        return;
                    }

                    let tab = node_for_tab.clone();

                    // this.tabs.push(node_for_tab.clone());

                    if let Ok(content) = std::fs::read_to_string(&path_for_click) {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(url) = value.get("url").and_then(|v| v.as_str()) {
                                let url = url.to_string();
                                let url_entity = tab.read(cx).file_url().unwrap().clone();
                                url_entity.update(cx, |input, cx| input.set_value(url, window, cx));
                            }

                            let query_params =
                                query_params_from_json(window, cx, tab.clone(), &value);

                            tab.update(cx, |node, _cx| {
                                *node.query_params_mut() = query_params;
                            });
                        }
                    }

                    this.tabs.push(tab);
                    this.active_tab_index = Some(this.tabs.len() - 1);
                    cx.notify();
                }),
            );
        }

        let children_entities = node.read(cx).children.clone();
        if children_entities.is_empty() {
            item
        } else {
            let mut children = Vec::new();
            for child in &children_entities {
                children.push(self.render_node(child, cx, active_path));
            }
            item.children(children)
        }
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_path = self
            .active_tab_index
            .and_then(|i| self.tabs.get(i))
            .map(|t| t.read(cx).path.clone());

        Sidebar::new("api-sidebar")
            .collapsible(SidebarCollapsible::Icon)
            .collapsed(self.sidebar_collapsed)
            .children(
                self.workspaces
                    .iter()
                    .find(|ws| ws.path == self.selected_workspace)
                    .into_iter()
                    .map(|ws| {
                        SidebarGroup::new(&ws.name).child(SidebarMenu::new().children(
                            ws.nodes.iter().map(|child| {
                                Self::render_node(self, child, cx, active_path.as_ref())
                            }),
                        ))
                    }),
            )
    }

    fn render_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_active_tab = !self.tabs.is_empty();

        div()
            .flex_none()
            .h(px(50.0))
            .w_full()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .flex()
            .items_center()
            .px(px(16.0))
            .child(if is_active_tab {
                h_flex().gap(rems(0.5)).child(
                    Button::new("toggle-response")
                        .ghost()
                        .small()
                        .icon(IconName::PanelBottom)
                        .tooltip("Response")
                        .on_click(cx.listener(|this: &mut ApiClient, _, _window, cx| {
                            if let Some(tab) = this.active_tab_index.and_then(|i| this.tabs.get(i))
                            {
                                tab.update(cx, |node, cx| {
                                    let show = !node.show_response_panel();
                                    node.set_show_response_panel(show);
                                    cx.notify();
                                })
                            }
                            cx.notify();
                        })),
                )
            } else {
                div()
            })
            .child(div().flex_1())
            .child(
                div()
                    .w(px(140.0))
                    .child(Select::new(&self.theme).appearance(false)),
            )
    }

    fn render_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(tab) = self.active_tab_index.and_then(|i| self.tabs.get(i)) else {
            return div().child("No tab open");
        };

        let tab_state = tab.read(cx);

        let method = tab_state.file_method().cloned();
        let url = tab_state.file_url().cloned();
        let is_dirty = tab_state.is_dirty();
        let response_panel = tab_state.response_panel().cloned();

        drop(tab_state); // <-- important

        let method = method.expect("tab should be a file node");
        let url = url.expect("tab should be a file node");

        h_flex()
            .w_full()
            .gap(rems(0.5))
            .child(div().w(px(110.)).child(Select::new(&method)))
            .child(div().flex_1().child(Input::new(&url)))
            .child(
                Button::new("save")
                    .secondary()
                    .label("Save")
                    .when(is_dirty, |this| {
                        this.child(div().size_2().rounded_full().bg(cx.theme().primary))
                    }),
            )
            .child(Button::new("send").primary().label("Send").on_click({
                let url = url.clone();

                cx.listener(move |this: &mut ApiClient, _, _window, cx| {
                    let url_str = url.read(cx).value().to_string();

                    if let Some(tab) = this.active_tab_index.and_then(|i| this.tabs.get(i)) {
                        tab.update(cx, |node, cx| {
                            node.set_show_response_panel(true);
                            cx.notify();
                        });
                    }

                    cx.notify();

                    let response_panel = response_panel.clone();
                    let url = url_str;

                    cx.spawn(async move |this, cx| {
                        let result = http::send_get(&url).await;

                        let _ = this.update_in(cx, |_this, window, cx| {
                            if let Some(rp) = response_panel {
                                rp.update(cx, |state, cx| match result {
                                    Ok(response) => {
                                        let formatted =
                                            serde_json::from_str::<serde_json::Value>(&response)
                                                .ok()
                                                .and_then(|v| serde_json::to_string_pretty(&v).ok())
                                                .unwrap_or(response);

                                        state.set_value(formatted, window, cx);
                                    }
                                    Err(err) => {
                                        state.set_value(format!("Error: {err}"), window, cx);
                                    }
                                });
                            }
                        });
                    })
                    .detach();
                })
            }))
    }
}

impl Render for ApiClient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_tab = self.active_tab_index.is_some();

        let show_response = self
            .active_tab_index
            .and_then(|i| self.tabs.get(i))
            .map(|t| t.read(cx).show_response_panel())
            .unwrap_or(false);

        let main_content = if has_tab {
            let editor_content = div()
                .size_full()
                .min_h(px(0.))
                .v_flex()
                .gap(px(16.))
                .child(
                    div()
                        .flex_none()
                        .v_flex()
                        .px(px(24.))
                        .pt(rems(1.0))
                        .child(self.render_editor(cx)),
                )
                .child(render_editor_config(self, cx))
                .child(
                    div().flex_1().overflow_y_scrollbar().px(px(24.)).child(
                        match self
                            .active_tab_index
                            .and_then(|i| self.tabs.get(i))
                            .map(|tab| tab.read(cx).selected_editor_config())
                            .unwrap_or(0)
                        {
                            0 => query_params::render_query_params_section(self, cx)
                                .into_any_element(),
                            _ => div().into_any_element(),
                        },
                    ),
                );
            if show_response {
                let response_content = div()
                    .w_full()
                    .h_full()
                    .min_h(px(0.)) // <- add this
                    .v_flex()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .child(
                        h_flex()
                            .w_full()
                            .flex_none()
                            .px(px(24.))
                            .py_2()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .child(div().text_sm().font_semibold().child("Response"))
                            .child(
                                Button::new("close-response")
                                    .ghost()
                                    .tooltip("Close Response")
                                    .small()
                                    .icon(IconName::Close)
                                    .on_click(cx.listener(
                                        |this: &mut ApiClient, _, _window, cx| {
                                            if let Some(tab) =
                                                this.active_tab_index.and_then(|i| this.tabs.get(i))
                                            {
                                                tab.update(cx, |node, cx| {
                                                    node.set_show_response_panel(false);
                                                    cx.notify();
                                                })
                                            }
                                            cx.notify();
                                        },
                                    )),
                            ),
                    )
                    .child({
                        if let Some(response_panel_state) = self
                            .active_tab_index
                            .and_then(|i| self.tabs.get(i))
                            .map(|t| t.read(cx).response_panel().cloned())
                            .flatten()
                        {
                            Input::new(&response_panel_state)
                                .flex_1()
                                // .disabled(true) // think about this
                                .appearance(false)
                                .into_any_element()
                        } else {
                            div().child("issue").into_any_element()
                        }
                    });
                v_resizable("editor-response-split")
                    .child(
                        resizable_panel()
                            .size(px(500.))
                            .size_range(px(200.)..px(4000.))
                            .child(editor_content),
                    )
                    .child(
                        resizable_panel()
                            .size(px(280.))
                            .size_range(px(100.)..px(600.))
                            .child(response_content),
                    )
                    .into_any_element()
            } else {
                editor_content.into_any_element()
            }
        } else {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("No tab open")
                .into_any_element()
        };

        div()
            .size_full()
            .flex()
            .on_action(cx.listener(Self::handle_create_file))
            .child(self.render_sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .min_h(px(0.))
                    .v_flex()
                    .child(
                        div()
                            .flex_none()
                            .overflow_x_hidden()
                            .child(render_tab_bar(self, cx)),
                    )
                    .child(div().flex_1().min_h(px(0.)).child(main_content))
                    .child(self.render_footer(cx)),
            )
    }
}
fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        let theme_name = SharedString::from("Asciinema");
        let default_theme = theme_name.clone();
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme);
        }

        if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
            if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
                Theme::global_mut(cx).apply_config(&theme);
            }
        }) {
            eprintln!("Failed to watch themes directory: {}", err);
        }

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|view_cx| ApiClient::new(window, view_cx, default_theme));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
