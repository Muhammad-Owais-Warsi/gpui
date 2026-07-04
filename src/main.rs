use gpui::*;
// use gpui_component_assets::Assets;
use gpui_component::IconName;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::select::{Select, SelectDelegate, SelectState};
use gpui_component::sidebar::{
    Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
    SidebarToggleButton,
};
use gpui_component::spinner::Spinner;
use gpui_component::tag::Tag;
use gpui_component::{button::*, *};

struct Todo {
    id: String,
    title: String,
    completed: bool,
}

struct ButtonState {
    is_sending: bool,
}

struct MyView {
    input: Entity<InputState>,
    todos: Vec<Todo>,
    pending: ButtonState,
    methods: Entity<SelectState<Vec<String>>>,
}

impl MyView {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Enter URL..."));

        let select_state = cx.new(|cx| {
            SelectState::new(
                vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PATCH".to_string(),
                    "DELETE".to_string(),
                ],
                Some(IndexPath::default()), // Select first item
                window,
                cx,
            )
        });

        cx.subscribe_in(
            &input,
            window,
            |view, state, event, window, cx| match event {
                InputEvent::PressEnter { secondary, shift } => {
                    let text = state.read(cx).value();
                    let id = format!("id-{}", text);
                    let completed = false;
                    view.todos.push(Todo {
                        id,
                        title: text.to_string(),
                        completed,
                    });

                    state.update(cx, |input, _cx| {
                        input.set_value("", window, _cx);
                    });

                    cx.notify();
                }
                _ => {}
            },
        )
        .detach();
        Self {
            input,
            todos: Vec::new(),
            pending: ButtonState { is_sending: false },
            methods: select_state,
        }
    }
}

impl Render for MyView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button = Button::new("btn")
            .primary()
            .label(if self.pending.is_sending {
                "Sending..."
            } else {
                "Send"
            })
            .icon(IconName::Search)
            .loading(self.pending.is_sending);

        div()
            .child(
                Sidebar::new("sidebar")
                    .header(
                        SidebarHeader::new()
                            .child(h_flex().gap_2().child(IconName::Folder).child("Explorer")),
                    )
                    .child(
                        SidebarGroup::new("Folders").child(
                            SidebarMenu::new()
                                .child(
                                    SidebarMenuItem::new("src")
                                        .icon(IconName::FolderOpen)
                                        .active(true)
                                        .children([
                                            SidebarMenuItem::new("components")
                                                .icon(IconName::Folder)
                                                .children([SidebarMenuItem::new("button.rs")]),
                                            SidebarMenuItem::new("utils").icon(IconName::Folder),
                                            SidebarMenuItem::new("main.rs")
                                                .icon(IconName::File)
                                                .active(true),
                                        ]),
                                )
                                .child(SidebarMenuItem::new("tests").icon(IconName::Folder))
                                .child(SidebarMenuItem::new("Cargo.toml").icon(IconName::File)),
                        ),
                    ),
            )
            .size_full()
            .flex()
            .pt_10()
            .child(
                div()
                    .w_96()
                    .v_flex()
                    .gap_3()
                    .child(div().text_xl().font_semibold().child("Api Client"))
                    .child(
                        div()
                            .h_flex()
                            .child(Select::new(&self.methods))
                            .child(Input::new(&self.input))
                            .child(button),
                    ),
            )
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|view_cx| MyView::new(window, view_cx));
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}
