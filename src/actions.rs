use crate::{ApiClient, Node, NodeType};
use gpui::{Action, AppContext, Context, Window};
use gpui_component::input::InputState;
use gpui_component::select::SelectState;
use gpui_component::IndexPath;

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CreateFile {
    pub parent: String,
}

impl ApiClient {
    pub fn handle_create_file(
        &mut self,
        action: &CreateFile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_path = self.selected_workspace.clone();

        match crate::fs::create_file("new", &action.parent) {
            Ok(path) => {
                let name_entity =
                    cx.new(|cx| InputState::new(window, cx).default_value("new.json"));
                let url = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));
                let methods: Vec<String> =
                    vec!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
                        .into_iter()
                        .map(String::from)
                        .collect();

                let new_node = cx.new(|cx| Node {
                    name: name_entity,
                    path,
                    is_file: true,
                    children: vec![],
                    node_type: NodeType::File {
                        method: cx.new(|cx| {
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
                        }),
                        url,
                        pending: false,
                        dirty: false,
                        selected_editor_config: 0,
                        response_panel: None,
                        show_response_panel: false,
                        query_params: vec![],
                    },
                });

                if let Some(folder) = self.find_node(&action.parent, cx) {
                    folder.update(cx, |node, _cx| {
                        node.children.push(new_node);
                    });
                }

                cx.notify();
            }
            Err(err) => eprintln!("Failed to create file: {err}"),
        }
    }
}
