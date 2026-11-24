//! The shared GitHub workspace binding: the "Working in" panel in the left
//! rail and the repository change/create dialog. A workspace always shows the
//! panel: bound it names the repository, unbound it offers a compact create
//! form (inline input + Create button) and a "Connect existing" link that
//! opens the full picker dialog. The PWA in `fennec-coding-agent/web` mirrors
//! these words, ids and behavior in DOM.

use std::rc::Rc;

use gpui_kit::component::{
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    label::Label,
    notification::Notification,
    scroll::ScrollableElement,
    ActiveTheme, Sizable, Theme, WindowExt,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::icons::ui_icon;

/// One row of the repository list, decoupled from the backend client type.
#[derive(Clone)]
pub struct RepositoryEntry {
    pub full_name: String,
    pub private: bool,
}

/// The left-rail GitHub panel: workspace name, then either the connected
/// repository with branch · version · working tree, or a compact create form
/// for unbound workspaces. The unbound state shows an inline input + Create
/// button for making a new private repository, plus a "Connect existing" link
/// that opens the full picker dialog.
pub fn working_in_banner<F, G>(
    t: &Theme,
    workspace: &str,
    repository: Option<&str>,
    branch: &str,
    repo_input: &Entity<InputState>,
    on_change: F,
    on_create: G,
) -> Div
where
    F: Fn(&mut Window, &mut App) + Clone + 'static,
    G: Fn(String, &mut Window, &mut App) + Clone + 'static,
{
    div()
        .relative()
        .overflow_hidden()
        .flex()
        .flex_col()
        .p_3()
        .mb_1()
        .rounded_md()
        .border_1()
        .border_color(t.border)
        .bg(linear_gradient(
            90.0,
            linear_color_stop(t.accent.opacity(0.10), 0.0),
            linear_color_stop(t.accent.opacity(0.0), 0.78),
        ))
        .child(div().absolute().left_0().top_0().h_full().w(px(3.0)).bg(t.accent))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    Label::new("Working in")
                        .text_size(px(10.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.muted_foreground),
                )
                .child(
                    Button::new("change-github-repository")
                        .ghost()
                        .xsmall()
                        .w(px(24.0))
                        .h(px(24.0))
                        .rounded_md()
                        .justify_center()
                        .child(ui_icon("github", 14.0, t.muted_foreground))
                        .on_click({
                            let on_change = on_change.clone();
                            move |_, window, cx| on_change(window, cx)
                        }),
                ),
        )
        .child(
            Label::new(workspace.to_string())
                .text_size(px(15.0))
                .font_weight(FontWeight::SEMIBOLD)
                .mt_1(),
        )
        .child(match repository {
            Some(repository) => div()
                .flex()
                .flex_col()
                .child(
                    Label::new(repository.to_string())
                        .text_size(px(11.0))
                        .font_family("monospace")
                        .text_color(t.accent)
                        .mt_1(),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .mt_1()
                        .child(
                            Label::new(branch.to_string())
                                .font_family("monospace")
                                .text_size(px(11.0))
                                .text_color(t.muted_foreground),
                        )
                        .child(Label::new("·").text_size(px(11.0)).text_color(t.muted_foreground))
                        .child(
                            Label::new("1.0.0")
                                .font_family("monospace")
                                .text_size(px(10.0))
                                .text_color(t.muted_foreground),
                        )
                        .child(
                            Label::new("working tree")
                                .text_size(px(11.0))
                                .text_color(t.muted_foreground),
                        ),
                ),
            None => {
                let repo_input = repo_input.clone();
                let on_create = on_create.clone();
                let on_change_clone = on_change.clone();
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .mt_2()
                    .child(
                        Label::new("Create a private repository")
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.muted_foreground),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Input::new(&repo_input)
                                    .h(px(28.0))
                                    .flex_1()
                                    .text_size(px(11.0)),
                            )
                            .child(
                                Button::new("create-github-repository-inline")
                                    .primary()
                                    .small()
                                    .label("Create")
                                    .on_click({
                                        let repo_input = repo_input.clone();
                                        let on_create = on_create.clone();
                                        move |_, window, cx| {
                                            let name = repo_input.read(cx).value().trim().to_string();
                                            if name.is_empty() {
                                                window.push_notification(
                                                    Notification::error("Enter a repository name."),
                                                    cx,
                                                );
                                                return;
                                            }
                                            on_create(name, window, cx);
                                        }
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .child(
                                Button::new("connect-github-repository")
                                    .link()
                                    .xsmall()
                                    .label("Connect existing")
                                    .on_click(move |_, window, cx| on_change_clone(window, cx)),
                            ),
                    )
            }
        })
}

/// The repository change/create dialog. `on_select` receives the picked
/// `owner/repo`; the app records the link and closes. `on_create` receives a
/// validated name for a new private repository. The dialog always closes on
/// create — the app reports progress, since creation runs against the backend.
pub fn open_repository_dialog<S, C>(
    window: &mut Window,
    cx: &mut App,
    repositories: Vec<RepositoryEntry>,
    current: Option<String>,
    on_select: S,
    on_create: C,
) where
    S: Fn(String, &mut App) + 'static,
    C: Fn(String, &mut App) + 'static,
{
    let name_input = cx.new(|cx| InputState::new(window, cx));
    // The builder re-runs when the dialog layer rebuilds, so the callbacks
    // live behind an Rc rather than in one build.
    let on_select = Rc::new(on_select);
    let on_create = Rc::new(on_create);
    window.open_dialog(cx, move |dialog, _, cx| {
        let name_input = name_input.clone();
        let on_select = on_select.clone();
        let on_create = on_create.clone();
        let current = current.clone();
        let repositories = repositories.clone();
        dialog
            .w(px(520.0))
            .bg(cx.theme().colors.background)
            .title("GitHub repository")
            .content({
                let name_input = name_input.clone();
                let on_select = on_select.clone();
                let on_create = on_create.clone();
                let current = current.clone();
                let repositories = repositories.clone();
                move |content, _, cx| {
                    let t = cx.theme().clone();
                    let mut list = div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .max_h(px(260.0))
                        .overflow_y_scrollbar();
                    for repository in repositories.iter().cloned() {
                        let selected = current.as_deref() == Some(repository.full_name.as_str());
                        let repo_name = repository.full_name.clone();
                        let on_select = on_select.clone();
                        list = list.child(
                            Button::new(format!("github-repo-{}", repository.full_name))
                                .ghost()
                                .w_full()
                                .justify_start()
                                .px_3()
                                .py_2()
                                .when(selected, |button| {
                                    button.bg(t.accent.opacity(0.14)).border_color(t.accent)
                                })
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .w_full()
                                        .child(ui_icon(
                                            "github",
                                            16.0,
                                            if selected { t.accent } else { t.muted_foreground },
                                        ))
                                        .child(
                                            Label::new(repository.full_name)
                                                .text_size(px(12.0))
                                                .flex_1(),
                                        )
                                        .child(
                                            Label::new(if repository.private { "Private" } else { "Public" })
                                                .text_size(px(10.0))
                                                .text_color(t.muted_foreground),
                                        ),
                                )
                                .on_click(move |_, window, cx| {
                                    on_select(repo_name.clone(), cx);
                                    window.close_dialog(cx);
                                }),
                        );
                    }
                    content
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(
                            Label::new("Change the repository connected to this workspace.")
                                .text_size(px(12.0))
                                .text_color(t.muted_foreground),
                        )
                        .child(if repositories.is_empty() {
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(t.muted)
                                .border_1()
                                .border_color(t.border)
                                .overflow_y_scrollbar()
                                .child(
                                    Label::new(
                                        "No GitHub repositories are available. Sign in with gh, or create one below.",
                                    )
                                    .text_size(px(12.0))
                                    .text_color(t.muted_foreground),
                                )
                        } else {
                            list
                        })
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .border_t_1()
                                .border_color(t.border)
                                .pt_3()
                                .child(
                                    Label::new("Create a private repository")
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_size(px(12.0)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(Input::new(&name_input).h(px(32.0)).flex_1())
                                        .child(
                                            Button::new("github-create-repository")
                                                .label("Create")
                                                .primary()
                                                .on_click({
                                                    let name_input = name_input.clone();
                                                    let on_create = on_create.clone();
                                                    move |_, window, cx| {
                                                        let name = name_input
                                                            .read(cx)
                                                            .value()
                                                            .trim()
                                                            .to_string();
                                                        if name.is_empty() {
                                                            window.push_notification(
                                                                Notification::error(
                                                                    "Enter a repository name.",
                                                                ),
                                                                cx,
                                                            );
                                                            return;
                                                        }
                                                        on_create(name, cx);
                                                        window.close_dialog(cx);
                                                    }
                                                }),
                                        ),
                                ),
                        )
                }
            })
            .footer(
                div()
                    .flex()
                    .justify_end()
                    .p_3()
                    .child(Button::new("github-repository-cancel").label("Cancel").ghost()
                        .on_click(|_, window, cx| window.close_dialog(cx))),
            )
    });
}
