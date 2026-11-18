//! Shared message widgets for the dense workflow layout both Fennec shells
//! render: right-aligned user bubbles, small "Fennec <time> [🔊]" AI headers,
//! bordered action/attachment/edit cards, and the tight scrollable queue strip.
//!
//! Click handlers are app-generic: they receive `(&mut Window, &mut App)` so
//! each app can capture a `WeakEntity` of its own state and update it.

use gpui_kit::component::{
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    label::Label,
    notification::Notification,
    scroll::ScrollableElement,
    Disableable, Sizable, Theme, WindowExt,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::markdown::rich_div;

/// First line of `s`, trimmed and bounded to `n` chars with an ellipsis.
pub fn truncate(s: &str, n: usize) -> String {
    let one = s.lines().next().unwrap_or(s).trim();
    if one.chars().count() > n {
        format!("{}…", one.chars().take(n).collect::<String>())
    } else {
        one.to_string()
    }
}

pub fn avatar(letter: &str, bg: Hsla) -> impl IntoElement {
    div().w(px(28.0)).h(px(28.0)).rounded_full().bg(bg).flex().items_center().justify_center().flex_shrink_0()
        .child(Label::new(letter).text_size(px(12.0)).text_color(Hsla::white()))
}

/// Speak `text` aloud off the UI thread. On Windows the PowerShell SAPI host
/// runs with CREATE_NO_WINDOW so no console flashes next to the app; macOS
/// uses `say`, Linux `espeak`.
pub fn speak_tts(text: &str, window: &mut Window, cx: &mut App) {
    if text.is_empty() {
        return;
    }
    let text = text.to_string();
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let script = format!(
                "Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak('{}')",
                text.replace("'", "''")
            );
            let _ = std::process::Command::new("powershell")
                .args(["-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output();
        }
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("say").arg(&text).output();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("espeak").arg(&text).output();
        let _ = &text;
    });
    window.push_notification(Notification::info("Speaking…"), cx);
}

/// The user's own words sit on the right in a rounded bubble: no avatar, no
/// header, just the message. `msg_idx` must be unique per message on screen.
pub fn user_bubble(t: &Theme, text: &str, msg_idx: usize) -> Stateful<Div> {
    div().id(ElementId::NamedInteger("user-msg".into(), msg_idx as u64))
        .flex().justify_end()
        .child(div().max_w(px(560.0)).rounded_lg().bg(t.muted).px_3().py_2()
            .child(rich_div(t, text, false, msg_idx)))
}

/// Small "Fennec <time> [🔊]" header above an AI turn. The speak button is
/// only rendered when a handler is given; `msg_id` keeps its element id
/// unique across messages.
pub fn ai_header<F>(t: &Theme, msg_id: u64, time: &str, on_speak: Option<F>) -> Div
where
    F: Fn(&mut Window, &mut App) + 'static,
{
    let mut row = div().flex().items_center().gap_2()
        .child(Label::new("Fennec").font_weight(FontWeight::SEMIBOLD).text_size(px(11.5)))
        .children((!time.is_empty()).then(|| {
            Label::new(time.to_string()).font_family("monospace").text_size(px(10.5)).text_color(t.muted_foreground)
        }));
    if let Some(on_speak) = on_speak {
        row = row.child(
            Button::new(ElementId::NamedInteger("tts".into(), msg_id)).ghost().xsmall().px_1()
                .child(Label::new("🔊").text_size(px(12.0)).text_color(t.muted_foreground))
                .on_click(move |_, window, cx| on_speak(window, cx)),
        );
    }
    row
}

/// The task-queue strip: a "Task queue · N items" header with a "View all"
/// affordance above tight, dark, scrollable rows — status dot + title left,
/// "Next"/"Queued" right. Rows stay compact (py_1, gap_0) but the list keeps
/// its scrollbar when many prompts wait.
pub fn queue_strip<F>(t: &Theme, items: &[String], on_view_all: F) -> Div
where
    F: Fn(&mut Window, &mut App) + 'static,
{
    let len = items.len();
    let header = div().flex().items_center().justify_between().px_4().py_1()
        .child(div().flex().items_center().gap_2()
            .child(Label::new("☰").text_size(px(13.0)).text_color(t.muted_foreground))
            .child(Label::new("Task queue").font_weight(FontWeight::SEMIBOLD).text_size(px(12.0)))
            .child(Label::new(format!("{len} item{}", if len == 1 { "" } else { "s" }))
                .text_size(px(11.0)).text_color(t.muted_foreground)))
        .child(Button::new("queue-view-all").label("View all").ghost().xsmall()
            .text_color(t.link)
            .on_click(move |_, window, cx| on_view_all(window, cx)));

    let mut rows = div().flex().flex_col().gap_0().max_h(px(120.0)).overflow_y_scrollbar();
    for (i, text) in items.iter().enumerate() {
        let is_next = i == 0;
        rows = rows.child(
            div().id(ElementId::NamedInteger("queue-item".into(), i as u64))
                .flex().items_center().justify_between().px_4().py_1()
                .border_t_1().border_color(t.border).bg(t.background)
                .child(div().flex().items_center().gap_2().overflow_hidden()
                    .child(div().w(px(6.0)).h(px(6.0)).flex_shrink_0().rounded_full()
                        .bg(if is_next { t.success } else { t.muted_foreground }))
                    .child(Label::new(truncate(text, 80)).text_size(px(12.0)).truncate()))
                .child(Label::new(if is_next { "Next" } else { "Queued" })
                    .text_size(px(10.5))
                    .text_color(t.muted_foreground)),
        );
    }

    div().border_t_1().border_color(t.border).bg(t.background).flex().flex_col()
        .child(header)
        .child(rows)
}

/// A bordered action row: "🔍 <title>" left, "Complete · 0.8s"/"Running…"
/// right, monospace detail behind the toggle. Collapsed by default; the whole
/// header expands it on click.
#[allow(clippy::too_many_arguments)]
pub fn action_card<F>(
    t: &Theme,
    id: u64,
    icon: &str,
    title: &str,
    detail: &str,
    status_text: &str,
    status_color: Hsla,
    open: bool,
    on_toggle: F,
) -> Stateful<Div>
where
    F: Fn(&mut Window, &mut App) + 'static,
{
    let header = div().id(ElementId::NamedInteger("action-h".into(), id))
        .flex().items_center().justify_between().px_3().py_1().cursor_pointer()
        .on_click(move |_, window, cx| on_toggle(window, cx))
        .child(div().flex().items_center().gap_2().overflow_hidden()
            .child(Label::new(if open { "▾" } else { "▸" }).text_size(px(10.0)).text_color(t.muted_foreground))
            .child(Label::new(icon.to_string()).text_size(px(13.0)).text_color(t.primary))
            .child(Label::new(title.to_string()).text_size(px(12.5)).font_weight(FontWeight::MEDIUM).truncate()))
        .child(Label::new(status_text.to_string()).text_size(px(11.0)).text_color(status_color));

    let mut card = div().id(ElementId::NamedInteger("action".into(), id))
        .rounded_md().border_1().border_color(t.border).bg(t.muted)
        .flex().flex_col()
        .child(header);
    if open && !detail.is_empty() {
        card = card.child(div().border_t_1().border_color(t.border).px_3().py_2()
            .font_family("monospace").max_h(px(160.0)).overflow_y_scrollbar()
            .child(Label::new(detail.to_string()).text_size(px(11.0)).text_color(t.muted_foreground)));
    }
    card
}

/// A delivered file: the type badge in a bordered tile, the name in semibold,
/// "<dir> · Markdown attachment" under it in mono, and a trailing "…" (the
/// menu affordance) or a size. When `on_open` is given the whole card opens it.
pub fn attachment_card<F>(
    t: &Theme,
    id: u64,
    name: &str,
    subtitle: &str,
    trailing: &str,
    on_open: Option<F>,
) -> Stateful<Div>
where
    F: Fn(&mut Window, &mut App) + 'static,
{
    // The badge is the extension, not a generic page: two delivered files side
    // by side have to read as different kinds at a glance.
    let badge = crate::markdown::artifact_badge(name);
    let mut card = div().id(ElementId::NamedInteger("attach".into(), id))
        .rounded_md().border_1().border_color(t.border).bg(t.muted).px_3().py_2()
        .flex().items_center().justify_between()
        .child(div().flex().items_center().gap_2().overflow_hidden()
            .child(div().w(px(26.0)).h(px(26.0)).rounded_md().border_1().border_color(t.border)
                .bg(t.background)
                .flex().items_center().justify_center().flex_shrink_0()
                .child(Label::new(badge.to_string()).font_family("monospace")
                    .text_size(px(8.5)).text_color(t.primary)))
            .child(div().flex().flex_col().overflow_hidden()
                .child(Label::new(name.to_string()).font_weight(FontWeight::SEMIBOLD).text_size(px(12.5)).truncate())
                .child(Label::new(subtitle.to_string()).font_family("monospace").text_size(px(10.5)).text_color(t.muted_foreground).truncate())))
        .child(Label::new(trailing.to_string()).text_size(px(12.0)).text_color(t.muted_foreground));
    if let Some(on_open) = on_open {
        card = card.cursor_pointer().on_click(move |_, window, cx| on_open(window, cx));
    }
    card
}

/// An edit card: "Edited 1 file" + "+42 −6" + [Review] on the header row, a
/// monospace path row with the same counts underneath, and the diff/output
/// behind the toggle.
#[allow(clippy::too_many_arguments)]
pub fn edit_card<F, G>(
    t: &Theme,
    id: u64,
    title: &str,
    adds: usize,
    dels: usize,
    path: &str,
    detail: &str,
    open: bool,
    on_toggle: F,
    on_review: G,
) -> Stateful<Div>
where
    F: Fn(&mut Window, &mut App) + 'static,
    G: Fn(&mut Window, &mut App) + 'static,
{
    let header = div().id(ElementId::NamedInteger("edit-h".into(), id))
        .flex().items_center().justify_between().px_3().py_1().cursor_pointer()
        .on_click(move |_, window, cx| on_toggle(window, cx))
        .child(div().flex().items_center().gap_2().overflow_hidden()
            .child(Label::new(if open { "▾" } else { "▸" }).text_size(px(10.0)).text_color(t.muted_foreground))
            .child(Label::new("✎").text_size(px(13.0)).text_color(t.primary))
            .child(Label::new(title.to_string()).text_size(px(12.5)).font_weight(FontWeight::MEDIUM))
            .child(div().flex().items_center().gap_1().flex_shrink_0()
                .child(Label::new(format!("+{adds}")).font_family("monospace").text_size(px(10.5)).text_color(t.success))
                .child(Label::new(format!("−{dels}")).font_family("monospace").text_size(px(10.5)).text_color(t.red))))
        .child(Button::new(ElementId::NamedInteger("edit-rev".into(), id)).label("Review").ghost().small()
            .on_click(move |_, window, cx| on_review(window, cx)));

    let mut card = div().id(ElementId::NamedInteger("edit".into(), id))
        .rounded_md().border_1().border_color(t.border).bg(t.muted)
        .flex().flex_col()
        .child(header);
    if !path.is_empty() {
        card = card.child(div().flex().items_center().justify_between()
            .border_t_1().border_color(t.border).px_3().py_1()
            .child(Label::new(path.to_string()).font_family("monospace").text_size(px(10.5)).text_color(t.muted_foreground).truncate())
            .child(div().flex().items_center().gap_1().flex_shrink_0()
                .child(Label::new(format!("+{adds}")).font_family("monospace").text_size(px(10.5)).text_color(t.success))
                .child(Label::new(format!("−{dels}")).font_family("monospace").text_size(px(10.5)).text_color(t.red))));
    }
    if open && !detail.is_empty() {
        card = card.child(div().border_t_1().border_color(t.border).px_3().py_2()
            .font_family("monospace").max_h(px(160.0)).overflow_y_scrollbar()
            .child(Label::new(detail.to_string()).text_size(px(11.0)).text_color(t.foreground)));
    }
    card
}

/// The composer input dock: a muted hint line above the multiline input.
pub fn composer_dock(t: &Theme, input: &Entity<InputState>, hint: &str) -> Stateful<Div> {
    div().id("composer-dock").rounded_md().border_1().border_color(t.border).p_3()
        .child(Label::new(hint.to_string()).text_size(px(13.0)).text_color(t.muted_foreground))
        .child(div().mt_2().child(Input::new(input)))
}

/// Attached-file chips shown above the composer; each has a remove ×.
pub fn attachment_chips<F>(t: &Theme, items: &[(String, bool)], on_remove: F) -> Div
where
    F: Fn(usize, &mut Window, &mut App) + Clone + 'static,
{
    let mut row = div().flex().flex_wrap().gap_1().px_2().py_1();
    for (i, (name, is_image)) in items.iter().enumerate() {
        let cb = on_remove.clone();
        row = row.child(
            div().flex().items_center().gap_1().px_2().py_1().rounded_md().bg(t.muted)
                .child(Label::new(if *is_image { "🖼" } else { "📄" }).text_size(px(12.0)))
                .child(Label::new(truncate(name, 40)).text_size(px(11.0)))
                .child(Button::new(ElementId::NamedInteger("att-remove".into(), i as u64))
                    .label("×").ghost().xsmall()
                    .on_click(move |_, window, cx| cb(i, window, cx))),
        );
    }
    row
}

/// The composer toolbar: attach + mode-colored permission trigger on the left,
/// model selector + mic + send on the right. No "+" new-task button. Optional
/// tools (attach, mic) render only when their plugin is enabled.
#[allow(clippy::too_many_arguments)]
pub fn composer_toolbar<A, P, M, C, S>(
    t: &Theme,
    plugins: &[crate::plugin::Plugin],
    perm_icon: &str,
    perm_label: &str,
    perm_color: Hsla,
    model_name: &str,
    stt_recording: bool,
    send_enabled: bool,
    on_attach: A,
    on_perm: P,
    on_model: M,
    on_mic: C,
    on_send: S,
) -> Div
where
    A: Fn(&mut Window, &mut App) + 'static,
    P: Fn(&mut Window, &mut App) + 'static,
    M: Fn(&mut Window, &mut App) + 'static,
    C: Fn(&mut Window, &mut App) + 'static,
    S: Fn(&mut Window, &mut App) + 'static,
{
    use crate::plugin::enabled;
    div().flex().items_center().justify_between()
        .child(div().flex().items_center().gap_1()
            .when(enabled(plugins, "attach"), |d| d.child(
                Button::new("composer-attach").ghost().small()
                    .child(Label::new("📎").text_size(px(15.0)))
                    .on_click(move |_, window, cx| on_attach(window, cx))))
            .child(Button::new("permission-trigger").ghost().small()
                .child(Label::new(format!("{perm_icon} {perm_label}")).text_size(px(12.0)).text_color(perm_color))
                .on_click(move |_, window, cx| on_perm(window, cx))))
        .child(div().flex().items_center().gap_2()
            .child(Button::new("model-trigger").ghost().small()
                .child(Label::new(format!("⚡ {model_name}")).text_size(px(12.0)))
                .on_click(move |_, window, cx| on_model(window, cx)))
            .when(enabled(plugins, "stt"), |d| d.child(
                Button::new("composer-mic").ghost().small()
                    .child(Label::new(if stt_recording { "⏹" } else { "🎤" }).text_size(px(15.0)))
                    .on_click(move |_, window, cx| on_mic(window, cx))))
            .child(Button::new("send").label("↑").primary().small()
                .when(!send_enabled, |b| b.disabled(true))
                .on_click(move |_, window, cx| on_send(window, cx))))
}