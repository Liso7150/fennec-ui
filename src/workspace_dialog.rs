//! The shared workspace-creation dialog: name input, source-folder picker,
//! emoji icon grid, color grid, and an index toggle. It is what the native
//! shell and the PWA agree on, so a workspace created in either has the same
//! fields.
//!
//! `on_create` receives `(name, source_folders, icon, color, indexed)` plus
//! the `&mut App`, so each app updates its own workspace store.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui_kit::component::{
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    label::Label,
    notification::Notification,
    ActiveTheme, Sizable, WindowExt,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub fn open_workspace_dialog<F>(window: &mut Window, cx: &mut App, on_create: F)
where
    F: Fn(String, Vec<String>, String, String, bool, &mut App) + 'static,
{
    let name_input = cx.new(|cx| InputState::new(window, cx));
    // The dialog builder may run again when the dialog layer is rebuilt, so
    // the callback is shared behind an Rc rather than moved into one build.
    let on_create = Rc::new(on_create);
    // Selection state lives OUTSIDE the builder: the builder re-runs on every
    // render, so state created inside it would reset on each window.refresh().
    let selected_icon: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let selected_color: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let indexed: Rc<Cell<bool>> = Rc::new(Cell::new(true));
    let source_folders: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));

    let icons: Vec<&'static str> = vec![
        "📁", "💻", "🔬", "🧪", "🎨", "🤖", "⚙️", "🔧", "📦", "🚀",
        "🌐", "📚", "🗂", "📝", "🧭", "🛠", "📡", "🔥", "🧠", "🎯",
    ];
    let colors: Vec<(f32, f32, f32)> = vec![
        (0.08, 0.7, 0.5),   // orange
        (0.33, 0.6, 0.45),  // green
        (0.6, 0.5, 0.5),    // blue
        (0.12, 0.8, 0.5),   // yellow
        (0.0, 0.7, 0.5),    // red
        (0.83, 0.6, 0.6),   // pink
        (0.75, 0.5, 0.5),   // purple
        (0.5, 0.6, 0.5),    // teal
    ];

    window.open_dialog(cx, move |dialog, _, _| {
        let name_input = name_input.clone();
        let on_create = on_create.clone();
        let selected_icon = selected_icon.clone();
        let selected_color = selected_color.clone();
        let indexed = indexed.clone();
        let source_folders = source_folders.clone();
        let icons = icons.clone();
        let colors = colors.clone();

        dialog
            .title("New Workspace")
            .content({
                let name_input = name_input.clone();
                let selected_icon = selected_icon.clone();
                let selected_color = selected_color.clone();
                let indexed = indexed.clone();
                let source_folders = source_folders.clone();
                let icons = icons.clone();
                let colors = colors.clone();
                move |content, _window, cx| {
                    let t = cx.theme().clone();
                    content
                        .flex().flex_col().gap_4()
                        // Workspace name
                        .child(div().flex().flex_col().gap_2()
                            .child(Label::new("Workspace name").font_weight(FontWeight::SEMIBOLD).text_size(px(12.0)))
                            .child(Input::new(&name_input).id("ws-name-input").h(px(32.0)))
                        )
                        // Source folders
                        .child(div().flex().flex_col().gap_2()
                            .child(Label::new("Source folders").font_weight(FontWeight::SEMIBOLD).text_size(px(12.0)))
                            .child(Button::new("add-folder-btn").w_full().justify_start()
                                .border_1().border_color(t.border).border_dashed()
                                .on_click({
                                    let source_folders = source_folders.clone();
                                    move |_, window, cx| {
                                        let rx = cx.prompt_for_paths(PathPromptOptions {
                                            files: false, directories: true, multiple: true, prompt: None,
                                        });
                                        let source_folders = source_folders.clone();
                                        window.spawn(cx, async move |cx| {
                                            if let Ok(Ok(Some(paths))) = rx.await {
                                                let folder_paths: Vec<String> = paths.iter()
                                                    .map(|p| p.to_string_lossy().to_string())
                                                    .collect();
                                                let _ = cx.update(|window, _| {
                                                    source_folders.borrow_mut().extend(folder_paths);
                                                    window.refresh();
                                                });
                                            }
                                        }).detach();
                                    }
                                })
                                .child(div().p_4().flex().items_center().justify_center()
                                    .child(Label::new("+ Click to add a readable and editable folder")
                                        .text_size(px(12.0)).text_color(t.muted_foreground))))
                            .child({
                                let folders = source_folders.borrow().clone();
                                div().flex().flex_col().gap_1().mt_1()
                                    .children(folders.iter().enumerate().map(|(i, f)| {
                                        let t = t.clone();
                                        let source_folders = source_folders.clone();
                                        // The row id keeps each folder's label on its own a11y
                                        // path; one `Label::new` in a loop shares an id otherwise.
                                        div().id(ElementId::NamedInteger("ws-folder".into(), i as u64))
                                            .p_2().rounded_sm().bg(t.muted).flex().items_center().gap_2()
                                            .child(Label::new(f.as_str()).text_size(px(11.0)).font_family("monospace")
                                                .flex_1().truncate())
                                            .child(Button::new(format!("rm-folder-{i}")).ghost().xsmall()
                                                .child(div().w(px(18.0)).h(px(18.0)).rounded_sm()
                                                    .flex().items_center().justify_center()
                                                    .child(Label::new("×").text_size(px(13.0))
                                                        .text_color(t.muted_foreground)))
                                                .on_click(move |_, window, _cx| {
                                                    source_folders.borrow_mut().remove(i);
                                                    window.refresh();
                                                }))
                                    }))
                            })
                        )
                        // Workspace icon
                        .child(div().flex().flex_col().gap_2()
                            .child(Label::new("Workspace icon").font_weight(FontWeight::SEMIBOLD).text_size(px(12.0)))
                            .child(div().flex().flex_wrap().gap_2()
                                .children(icons.iter().enumerate().map(|(i, icon)| {
                                    let t = t.clone();
                                    let selected_icon = selected_icon.clone();
                                    let is_selected = selected_icon.get() == i;
                                    Button::new(format!("icon-{}", i))
                                        .when(is_selected, |b| b.bg(t.accent.opacity(0.2)).border_color(t.accent))
                                        .on_click(move |_, window, _cx| {
                                            selected_icon.set(i);
                                            window.refresh();
                                        })
                                        .child(div().w(px(36.0)).h(px(36.0)).rounded_md()
                                            .border_1().border_color(if is_selected { t.accent } else { t.border })
                                            .flex().items_center().justify_center()
                                            .child(Label::new(*icon).text_size(px(18.0))))
                                })))
                        )
                        // Workspace color
                        .child(div().flex().flex_col().gap_2()
                            .child(Label::new("Workspace color").font_weight(FontWeight::SEMIBOLD).text_size(px(12.0)))
                            .child(div().flex().flex_wrap().gap_2()
                                .children(colors.iter().enumerate().map(|(i, color)| {
                                    let t = t.clone();
                                    let selected_color = selected_color.clone();
                                    let is_selected = selected_color.get() == i;
                                    let color_hsla = hsla(color.0, color.1, color.2, 1.0);
                                    Button::new(format!("color-{}", i))
                                        .on_click(move |_, window, _cx| {
                                            selected_color.set(i);
                                            window.refresh();
                                        })
                                        .child(div().w(px(28.0)).h(px(28.0)).rounded_full()
                                            .bg(color_hsla)
                                            .border_2().border_color(if is_selected { t.foreground } else { Hsla::transparent_black() }))
                                })))
                        )
                        // Workspace index toggle
                        .child(div().flex().items_center().justify_between().p_3().rounded_md().bg(t.muted).border_1().border_color(t.border)
                            .child(div().flex().flex_col().flex_1()
                                .child(Label::new("Workspace index").font_weight(FontWeight::SEMIBOLD).text_size(px(12.0)))
                                .child(Label::new("Building a workspace index gives the agent more context.")
                                    .text_size(px(11.0)).text_color(t.muted_foreground)))
                            .child({
                                let indexed = indexed.clone();
                                let is_on = indexed.get();
                                div().flex().flex_shrink_0().child(
                                Button::new("index-toggle")
                                    .on_click(move |_, window, _cx| {
                                        indexed.set(!indexed.get());
                                        window.refresh();
                                    })
                                    .child(div().w(px(40.0)).h(px(22.0)).rounded_full()
                                        .bg(if is_on { t.green } else { t.muted_foreground })
                                        .flex().items_center()
                                        .child(div().w(px(18.0)).h(px(18.0)).rounded_full().bg(Hsla::white())
                                            .ml(if is_on { px(20.0) } else { px(2.0) }))))
                            })
                        )
                }
            })
            .footer(div().flex().justify_end().gap_2().p_3()
                .child(Button::new("new-ws-cancel").label("Cancel").ghost()
                    .on_click(|_, window, cx| { window.close_dialog(cx); }))
                .child(Button::new("new-ws-create").label("Create").primary()
                    .on_click({
                        let name_input = name_input.clone();
                        let on_create = on_create.clone();
                        let selected_icon = selected_icon.clone();
                        let selected_color = selected_color.clone();
                        let indexed = indexed.clone();
                        let source_folders = source_folders.clone();
                        let icons = icons.clone();
                        move |_, window, cx| {
                            let name = name_input.read(cx).value().trim().to_string();
                            if name.is_empty() {
                                window.push_notification(Notification::error("Please enter a workspace name."), cx);
                                return;
                            }
                            let icon = icons[selected_icon.get()].to_string();
                            let color = match selected_color.get() {
                                0 => "orange", 1 => "green", 2 => "blue", 3 => "yellow",
                                4 => "red", 5 => "pink", 6 => "purple", _ => "teal",
                            }.to_string();
                            let idx = indexed.get();
                            let folders = source_folders.borrow().clone();
                            on_create(name, folders, icon, color, idx, cx);
                            window.close_dialog(cx);
                        }
                    })))
    });
}