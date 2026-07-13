use crate::helpers::find_node_mut;
use crate::{ApiClient, Node};
use gpui::{Action, Context, Window};

#[derive(Clone, PartialEq, Action)]
#[action(namespace = fs, no_json)]
pub struct CreateFile {
    pub parent: String,
}

impl ApiClient {
    pub fn handle_create_file(
        &mut self,
        action: &CreateFile,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_path = self.selected_workspace.path.clone();

        match crate::fs::create_file("new", &action.parent) {
            Ok(path) => {
                let new_node = Node {
                    name: "new.json".to_string(),
                    path,
                    method: "GET".to_string(),
                    children: vec![],
                    is_file: true,
                };

                pub fn find_node_mut<'a>(
                    nodes: &'a mut [Node],
                    path: &str,
                ) -> Option<&'a mut Node> {
                    for node in nodes.iter_mut() {
                        if node.path == path {
                            return Some(node);
                        }
                        if let Some(found) = find_node_mut(&mut node.children, path) {
                            return Some(found);
                        }
                    }
                    None
                }

                if let Some(ws) = self
                    .workspaces
                    .iter_mut()
                    .find(|w| w.path == workspace_path)
                {
                    if let Some(folder) = find_node_mut(&mut ws.nodes, &action.parent) {
                        folder.children.push(new_node);
                    }
                }

                cx.notify();
            }
            Err(err) => eprintln!("Failed to create file: {err}"),
        }
    }
}
