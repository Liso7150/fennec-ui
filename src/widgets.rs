//! Shared message widgets for the dense workflow layout the Fennec desktop
//! shell renders: the user's accent-tinted card, a `<time> [🔊]` AI header with
//! no identity chip, bordered action/attachment/edit cards, and the tight
//! scrollable queue strip.
//!
//! Click handlers are app-generic: they receive `(&mut Window, &mut App)` so
//! each app can capture a `WeakEntity` of its own state and update it.

use std::cell::RefCell;
use std::collections::HashMap;

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
use crate::timeline::{GroupRow, VISIBLE_ROWS};

/// The 20px fennec face, painted from a few filled polygons so it stays crisp
/// at the native shell's small header size and does not depend on a font glyph.
pub fn brand_fox(t: &Theme) -> impl IntoElement {
    let fur = t.accent;
    let shadow = t.background;
    gpui_kit::canvas(
        |bounds, _window, _cx| bounds,
        move |bounds, _state, window, _cx| {
            let face = [
                point(px(2.0), px(4.0)),
                point(px(6.0), px(1.0)),
                point(px(10.0), px(4.0)),
                point(px(14.0), px(1.0)),
                point(px(18.0), px(4.0)),
                point(px(17.0), px(13.0)),
                point(px(10.0), px(20.0)),
                point(px(3.0), px(13.0)),
            ];
            let left_ear = [
                point(px(4.0), px(5.0)),
                point(px(6.0), px(3.0)),
                point(px(8.0), px(6.0)),
            ];
            let right_ear = [
                point(px(12.0), px(6.0)),
                point(px(14.0), px(3.0)),
                point(px(16.0), px(5.0)),
            ];
            let left_eye = [
                point(px(5.0), px(10.0)),
                point(px(8.0), px(9.0)),
                point(px(8.0), px(11.0)),
            ];
            let right_eye = [
                point(px(12.0), px(9.0)),
                point(px(15.0), px(10.0)),
                point(px(12.0), px(11.0)),
            ];
            let nose = [
                point(px(9.0), px(14.0)),
                point(px(11.0), px(14.0)),
                point(px(10.0), px(16.0)),
            ];
            for points in [&face[..], &left_ear[..], &right_ear[..], &left_eye[..], &right_eye[..], &nose[..]] {
                if let Some(path) = gpui_kit::component::plot::polygon(points, &bounds) {
                    window.paint_path(path, if points.len() == 8 { fur } else { shadow });
                }
            }
        },
    )
    .w(px(20.0))
    .h(px(20.0))
    .flex_shrink_0()
}

/// First line of `s`, trimmed and bounded to `n` chars with an ellipsis.
pub fn truncate(s: &str, n: usize) -> String {
    let one = s.lines().next().unwrap_or(s).trim();
    if one.chars().count() > n {
        format!("{}…", one.chars().take(n).collect::<String>())
    } else {
        one.to_string()
    }
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

/// The PWA's `.message-card`: one 10px-rounded, hairline-bordered `--surface`
/// box with 16px of padding per turn, so a run reads as cards rather than as
/// loose text on the window ground.
pub fn msg_card(t: &Theme, body: Div) -> Div {
    div().rounded_lg().border_1().border_color(t.border).bg(t.muted).p_4()
        .flex().flex_col().gap_3().child(body)
}

/// The user's own turn, as `.message.user .message-card` draws it: the card is
/// tinted by the accent rather than filled with a surface, its hairline is the
/// accent at 15%, and its time closes the message on the left. The identity
/// chips are gone and no header row is left behind them. `msg_idx` must be
/// unique per message on screen.
pub fn user_card(t: &Theme, time: &str, text: &str, msg_idx: usize) -> Stateful<Div> {
    div().id(ElementId::NamedInteger("user-msg".into(), msg_idx as u64))
        .rounded_lg().border_1().border_color(t.accent.opacity(0.15)).bg(t.accent.opacity(0.08)).p_4()
        .flex().flex_col().gap_2()
        .child(rich_div(t, text, false, msg_idx))
        .child(msg_footer(t, msg_idx as u64, time, None))
}

/// The line that closes a message with no header to carry one: the time and,
/// where the card speaks, the 🔊 button — both read down the left edge, with no
/// rule above them and nothing pushed to the right. `msg_id` keeps the button's
/// element id unique across messages.
pub fn msg_footer(
    t: &Theme,
    msg_id: u64,
    time: &str,
    on_speak: Option<Box<dyn Fn(&mut Window, &mut App)>>,
) -> Div {
    let mut row = div().flex().items_center().gap_2();
    if !time.is_empty() {
        row = row.child(Label::new(time.to_string()).font_family("monospace").text_size(px(11.0)).text_color(t.muted_foreground));
    }
    if let Some(on_speak) = on_speak {
        row = row.child(
            Button::new(ElementId::NamedInteger("tts".into(), msg_id)).ghost().xsmall().px_2().py_1()
                .rounded_md().border_1().border_color(t.border)
                .child(Label::new("🔊").text_size(px(12.0)).text_color(t.muted_foreground))
                .on_click(move |_, window, cx| on_speak(window, cx)),
        );
    }
    row
}

/// The header row a collapsible card starts with: any status words in
/// monospace, then the time pushed to the right. The avatar and author name are
/// deliberately not drawn — the user asked for the identity chips gone. It
/// carries the hairline under itself (`padding-bottom: 12px; margin-bottom:
/// 12px; border-bottom`), and returns a plain `Div` so the app can make the row
/// itself the toggle (`.id().cursor_pointer().on_click()`), which is how the
/// thoughts of an answer open and shut.
///
/// `msg_id` keeps any speak button's element id unique across messages:
/// `Label::new` is `#[track_caller]`, so two messages built from one call site
/// would hand the a11y tree the same id, which aborts a debug build and silently
/// drops the row in a release one.
pub fn header_row(
    t: &Theme,
    msg_id: u64,
    status: &[(&str, Hsla)],
    time: &str,
    on_speak: Option<Box<dyn Fn(&mut Window, &mut App)>>,
) -> Div {
    let mut row = div().flex().items_center().gap_3();
    for (text, color) in status {
        row = row.child(Label::new(text.to_string()).font_family("monospace").text_size(px(11.0)).text_color(*color));
    }
    row = row.child(div().flex_1());
    if !time.is_empty() {
        row = row.child(Label::new(time.to_string()).font_family("monospace").text_size(px(11.0)).text_color(t.muted_foreground));
    }
    if let Some(on_speak) = on_speak {
        row = row.child(
            Button::new(ElementId::NamedInteger("tts".into(), msg_id)).ghost().xsmall().px_2().py_1()
                .rounded_md().border_1().border_color(t.border)
                .child(Label::new("🔊").text_size(px(12.0)).text_color(t.muted_foreground))
                .on_click(move |_, window, cx| on_speak(window, cx)),
        );
    }
    row.pb_3().mb_3().border_b_1().border_color(t.border)
}

/// The task-queue strip, `.task-queue`: a hairline above the composer, an 11px
/// header ("☰ Task queue" with the count pushed right), then tight surface rows
/// — warn dot + prompt left, blue "Next"/"Queued" and a bordered × right. The ×
/// is what the PWA row has too: a queued prompt is the user's, so taking it back
/// out is never the run's decision.
pub fn queue_strip<F, R>(t: &Theme, items: &[String], on_view_all: F, on_remove: R) -> Div
where
    F: Fn(&mut Window, &mut App) + 'static,
    R: Fn(usize, &mut Window, &mut App) + Clone + 'static,
{
    let len = items.len();
    let header = div().flex().items_center().gap_2().pb_1()
        .child(Label::new("☰").text_size(px(12.0)).text_color(t.muted_foreground))
        .child(Label::new("Task queue").font_weight(FontWeight::SEMIBOLD).text_size(px(11.0)))
        .child(Label::new(format!("{len} item{}", if len == 1 { "" } else { "s" }))
            .text_size(px(11.0)).text_color(t.muted_foreground))
        .child(Button::new("queue-view-all").label("View all").ghost().xsmall()
            .text_color(t.link)
            .on_click(move |_, window, cx| on_view_all(window, cx)));

    let mut rows = div().flex().flex_col().gap_1().max_h(px(100.0)).overflow_y_scrollbar();
    for (i, text) in items.iter().enumerate() {
        let is_next = i == 0;
        let cb = on_remove.clone();
        rows = rows.child(
            div().id(ElementId::NamedInteger("queue-item".into(), i as u64))
                .flex().items_start().gap_2().px_3().py_1().rounded_md().bg(t.muted)
                .child(div().w(px(6.0)).h(px(6.0)).mt(px(6.0)).flex_shrink_0().rounded_full().bg(t.yellow))
                // No character cap and no `.truncate()`: a queued subtask is a whole
                // instruction the user has to be able to read, and the ones the loop
                // hands on for a skipped work phase are sentences. `flex_1` gives the
                // text a zero-width basis, so it wraps inside the row instead of cutting.
                .child(div().flex_1().child(Label::new(text.clone()).text_size(px(12.0))))
                .child(Label::new(if is_next { "Next" } else { "Queued" })
                    .font_family("monospace").text_size(px(10.0)).text_color(t.blue).mt(px(3.0)))
                .child(Button::new(ElementId::NamedInteger("queue-remove".into(), i as u64))
                    .label("×").ghost().xsmall().w(px(22.0)).h(px(22.0)).rounded_sm().border_1().border_color(t.border)
                    .on_click(move |_, window, cx| cb(i, window, cx))),
        );
    }

    div().border_t_1().border_color(t.border).bg(crate::theme::panel()).px_6().py_2().flex().flex_col().gap_1()
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
    // The running call's description wraps rather than being cut, as the
    // PWA's .action-label does; that is why the left group has no
    // .overflow_hidden() and the label no .truncate().
    let header = div().id(ElementId::NamedInteger("action-h".into(), id))
        .flex().items_center().gap_3().px_4().py_3().cursor_pointer()
        .on_click(move |_, window, cx| on_toggle(window, cx))
        .child(Label::new(icon.to_string()).text_size(px(13.0)).text_color(t.primary))
        .child(div().flex().flex_1().child(
            Label::new(title.to_string()).text_size(px(12.5)).font_weight(FontWeight::MEDIUM)))
        .child(Label::new(status_text.to_string()).text_size(px(11.0)).text_color(status_color).ml_auto().flex_shrink_0());

    let mut card = div().id(ElementId::NamedInteger("action".into(), id))
        .rounded_md().border_1().border_color(t.border).bg(t.muted)
        .flex().flex_col()
        .child(header);
    // A call opened before it has printed still has to show a body — an empty
    // card reads as a broken toggle, not as a tool that has not run yet.
    if open {
        let body = match (detail.is_empty(), status_text.starts_with("Running")) {
            (true, true) => "No output yet.",
            (true, false) => "This call returned no output.",
            (false, _) => detail,
        };
        card = card.child(div().mx_4().mb_4().rounded_md().border_1().border_color(t.border)
            .bg(crate::theme::panel())
            .font_family("monospace").px_3().py_2().max_h(px(240.0)).overflow_y_scrollbar()
            .child(Label::new(body.to_string()).text_size(px(11.5)).text_color(t.muted_foreground)));
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
        .rounded_md().border_1().border_color(t.border).bg(t.muted).px_3().py_2().min_h(px(44.0))
        .flex().items_center().justify_between().gap_3()
        .child(div().flex().items_center().gap_3().flex_1()
            .child(div().w(px(28.0)).h(px(28.0)).rounded_sm().border_1().border_color(t.border)
                .bg(t.background)
                .flex().items_center().justify_center().flex_shrink_0()
                .child(Label::new(badge.to_string()).font_family("monospace")
                    .text_size(px(9.0)).text_color(t.accent)))
            .child(div().flex().flex_col().flex_1()
                // No .truncate(): a delivered file's name and directory are its
                // identity, so a long one wraps inside the card instead of being
                // cut — the same treatment the folded group rows give a path.
                .child(Label::new(name.to_string()).font_weight(FontWeight::SEMIBOLD).text_size(px(13.0)))
                .child(Label::new(subtitle.to_string()).font_family("monospace").text_size(px(11.5)).text_color(t.muted_foreground))))
        .child(Label::new(trailing.to_string()).text_size(px(12.0)).text_color(t.muted_foreground).flex_shrink_0());
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
            .child(div().flex().flex_1().overflow_hidden()
                .child(Label::new(path.to_string()).font_family("monospace").text_size(px(10.5)).text_color(t.muted_foreground)))
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

/// The height one row of a folded block actually occupies: the label's line box
/// plus the row's `py_1`. `Label` sets its line height in `rem`, and the root
/// publishes the theme's 16px font size as the rem size, so an 11.5px mono label
/// still stands 20px tall — 20 + 4 + 4 = 28. Measured on screen as a 28px pitch.
const GROUP_ROW_H: f32 = 28.0;

/// Body height that shows [`VISIBLE_ROWS`] single-line rows, plus the body's own
/// `py_1`.
const GROUP_BODY_H: f32 = VISIBLE_ROWS as f32 * GROUP_ROW_H + 8.0;

thread_local! {
    /// Scroll state per block, held outside the frame: gpui only honours a
    /// `ScrollHandle` that survives a redraw, and a live block has to follow its
    /// own tail as calls land. The `usize` is how many rows the block drew last,
    /// which is what tells a new call apart from a clock tick.
    static GROUP_SCROLL: RefCell<HashMap<u64, (ScrollHandle, usize)>> = RefCell::new(HashMap::new());
}

/// The handle of one block, and whether the view should move with it. Only a
/// block the run is still inside follows its tail, and only when a row actually
/// arrived — otherwise every tick of the timer would drag a user who scrolled up
/// back to the bottom.
fn group_scroll(id: u64, rows: usize, live: bool) -> (ScrollHandle, bool) {
    GROUP_SCROLL.with(|cell| {
        let mut map = cell.borrow_mut();
        // Blocks are keyed by the id of their first call, so this only grows while
        // one window is open; clearing it costs a restored scroll position, which
        // is what a redraw of an old block is anyway.
        if map.len() > 512 {
            map.clear();
        }
        let entry = map.entry(id).or_insert_with(|| (ScrollHandle::new(), 0));
        let follow = live && rows > entry.1;
        entry.1 = rows;
        (entry.0.clone(), follow)
    })
}

/// The actions of one stretch of a run — everything between two answers — folded
/// into one block titled by [`crate::timeline::block_title`], with the calls
/// themselves behind the toggle. `open` comes from
/// [`crate::timeline::block_open`]: the block the run is still inside is drawn
/// open and the answer folds it, unless the user's own click said otherwise.
/// `live` says the run is still inside, which is what makes
/// the block follow its own tail. `on_row` is what a row click means to this app
/// (open the file, review the change), so a block whose rows lead nowhere is built
/// without it.
#[allow(clippy::too_many_arguments)]
pub fn tool_group<F, G>(
    t: &Theme,
    id: u64,
    icon: &str,
    title: &str,
    rows: &[GroupRow],
    open: bool,
    live: bool,
    on_toggle: F,
    on_row: Option<G>,
) -> Stateful<Div>
where
    F: Fn(&mut Window, &mut App) + 'static,
    G: Fn(usize, &mut Window, &mut App) + Clone + 'static,
{
    let failed = rows.iter().filter(|r| r.failed).count();
    let header = div().id(ElementId::NamedInteger("group-h".into(), id))
        .flex().items_center().gap_2().px_4().py_3().cursor_pointer()
        .on_click(move |_, window, cx| on_toggle(window, cx))
        .child(Label::new(if open { "▾" } else { "▸" }).text_size(px(10.0)).text_color(t.muted_foreground))
        .child(Label::new(icon.to_string()).text_size(px(12.0)).text_color(t.accent))
        .child(Label::new(title.to_string()).text_size(px(12.5)).font_weight(FontWeight::SEMIBOLD))
        // A failed call has to be legible without expanding the block: the count
        // is the one thing on this row that is not the user's own doing.
        .children((failed > 0).then(|| Label::new(format!("{failed} failed"))
            .text_size(px(11.0)).text_color(t.red).ml_auto().flex_shrink_0()));

    let mut card = div().id(ElementId::NamedInteger("group".into(), id))
        .rounded_md().border_1().border_color(t.border).bg(t.muted)
        .flex().flex_col().overflow_hidden()
        .child(header);
    if open {
        let (handle, follow) = group_scroll(id, rows.len(), live);
        // Asked for before layout, so this is the frame the new row lands in.
        if follow {
            handle.scroll_to_bottom();
        }
        let mut body = div().id(ElementId::NamedInteger("group-b".into(), id))
            .border_t_1().border_color(t.border).flex().flex_col().py_1()
            // Around VISIBLE_ROWS lines are on screen; what the run did before
            // them stays reachable by scrolling up.
            .max_h(px(GROUP_BODY_H)).track_scroll(&handle).overflow_y_scrollbar();
        for (i, row) in rows.iter().enumerate() {
            let status_color = if row.failed { t.red } else { t.muted_foreground };
            // The row id carries its block's id: two blocks open at once would
            // otherwise both claim `group-row 0` and a debug build aborts on the
            // duplicate a11y id.
            // A live progress line is dimmer than the call it belongs to and
            // never leads anywhere: it says what the run is doing right now, and
            // the next report replaces it, so there is nothing to open.
            let label_color = if row.live { t.muted_foreground } else { t.foreground };
            let mut line = div().id(ElementId::NamedInteger(format!("group-row-{id}").into(), i as u64))
                .flex().items_start().gap_2().px_4().py_1()
                // A row exists to say which file or command was called, so a long
                // path wraps instead of being cut off — the same rule the approved
                // commands list follows. The label needs its own column: as a direct
                // child of this row its min-content width is one unbroken path, which
                // pushes past the pane and gets clipped instead of wrapping.
                .child(div().min_w(px(26.0)).flex_shrink_0().justify_center().child(
                    Label::new(row.icon.clone()).font_family("monospace").text_size(px(9.5)).text_color(t.accent)))
                .child(div().flex().flex_col().flex_1().overflow_hidden()
                    .child(Label::new(row.label.clone()).font_family("monospace").text_size(px(11.5)).text_color(label_color)))
                .child(div().flex().items_center().gap_2().flex_shrink_0()
                    .children((row.adds > 0).then(|| Label::new(format!("+{}", row.adds))
                        .font_family("monospace").text_size(px(10.5)).text_color(t.success)))
                    .children((row.dels > 0).then(|| Label::new(format!("−{}", row.dels))
                        .font_family("monospace").text_size(px(10.5)).text_color(t.red)))
                    .children((!row.status.is_empty()).then(|| Label::new(row.status.clone())
                        .text_size(px(10.5)).text_color(status_color))));
            if let Some(on_row) = &on_row {
                let cb = on_row.clone();
                line = line.cursor_pointer().hover(|this| this.bg(crate::theme::hover()))
                    .on_click(move |_, window, cx| cb(i, window, cx));
            }
            body = body.child(line);
        }
        card = card.child(body);
    }
    card
}

/// One call's own output, on demand. The folded block keeps the timeline short;
/// this keeps the text reachable, which is what a user wants when a row reads
/// `Failed`. Every surface needs it, so it lives here once.
pub fn show_output(title: &str, text: &str, window: &mut Window, cx: &mut App) {
    let title = title.to_string();
    let body = if text.trim().is_empty() {
        "This call returned no output.".to_string()
    } else {
        text.trim().to_string()
    };
    window.open_dialog(cx, move |dialog, _, _| {
        let (title, body) = (title.clone(), body.clone());
        dialog.title(title).content(move |content, _, _| {
            content.child(div().max_h(px(420.0)).overflow_y_scrollbar().font_family("monospace")
                .child(Label::new(body.clone()).text_size(px(11.5))))
        })
    });
}

/// The composer input, as `.composer-input` is: no border, no box, 14px on the
/// panel ground, and the prompt in grey inside it rather than on a line above —
/// which is why `show_hint` paints the placeholder over the field instead of
/// beside it.
pub fn composer_dock(t: &Theme, input: &Entity<InputState>, hint: &str, show_hint: bool) -> Stateful<Div> {
    div().id("composer-dock").relative().flex().min_h(px(40.0))
        .child(
            div().flex_1().child(
                Input::new(input).bordered(false).appearance(false).text_size(px(14.0)).h_full(),
            ),
        )
        .when(show_hint, |d| d.child(
            div().absolute().top_0().left_0().child(
                Label::new(hint.to_string()).text_size(px(14.0)).text_color(t.muted_foreground),
            ),
        ))
}

/// Attached-file chips shown above the composer; each has a remove ×.
pub fn attachment_chips<F>(t: &Theme, items: &[(String, bool)], on_remove: F) -> Div
where
    F: Fn(usize, &mut Window, &mut App) + Clone + 'static,
{
    let mut row = div().flex().flex_wrap().gap_2().px_2().py_1();
    for (i, (name, is_image)) in items.iter().enumerate() {
        let cb = on_remove.clone();
        row = row.child(
            div().flex().items_center().gap_2().px_2().py_1().rounded_lg().max_w(px(260.0))
                .bg(t.muted).border_1().border_color(t.border)
                .child(Label::new(if *is_image { "🖼" } else { "📄" }).text_size(px(12.0)))
                .child(Label::new(truncate(name, 40)).text_size(px(11.0)).text_color(t.foreground))
                .child(Button::new(ElementId::NamedInteger("att-remove".into(), i as u64))
                    .label("×").ghost().xsmall().w(px(18.0)).h(px(18.0)).rounded_full()
                    .on_click(move |_, window, cx| cb(i, window, cx))),
        );
    }
    row
}

/// The composer toolbar, one hairline under the input as `.composer-toolbar` is:
/// attach + mode-colored permission trigger on the left, model selector + mic +
/// send on the right. The triggers are bordered `--surface` chips, not pills;
/// send is a 36px accent circle with a dark glyph. No "+" new-task button —
/// optional tools (attach, mic) render only when their plugin is enabled.
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
    // A trigger is `.permission-trigger` / `.model-trigger`: 6px 10px, an 8px
    // radius, the surface ground and a hairline around it.
    let trigger = |id: &'static str, body: Div, on_click: Box<dyn Fn(&mut Window, &mut App)>| {
        Button::new(id).ghost()
            .flex().items_center().gap_2().px_3().py_1().rounded_md()
            .bg(t.muted).border_1().border_color(t.border)
            .child(body)
            .child(Label::new("▾").text_size(px(10.0)).text_color(t.muted_foreground))
            .on_click(move |_, window, cx| on_click(window, cx))
    };
    div().flex().items_center().justify_between().gap_2()
        .mt_2().pt_2().border_t_1().border_color(t.border)
        .child(div().flex().items_center().gap_2().overflow_hidden()
            .when(enabled(plugins, "attach"), |d| d.child(
                Button::new("composer-attach").ghost()
                    .w(px(32.0)).h(px(32.0)).rounded_md().justify_center()
                    .child(Label::new("📎").text_size(px(14.0)))
                    .on_click(move |_, window, cx| on_attach(window, cx))))
            .child(trigger("permission-trigger",
                div().flex().items_center().gap_2().overflow_hidden()
                    .child(Label::new(perm_icon.to_string()).text_size(px(14.0)).text_color(perm_color))
                    .child(Label::new(perm_label.to_string()).text_size(px(12.0)).font_weight(FontWeight::MEDIUM).text_color(perm_color)),
                Box::new(move |window, cx| on_perm(window, cx)))))
        .child(div().flex().items_center().gap_2().flex_shrink_0()
            .child(trigger("model-trigger",
                div().flex().items_center().gap_2().overflow_hidden()
                    .child(Label::new("⚡").text_size(px(13.0)).text_color(t.yellow))
                    .child(Label::new(model_name.to_string()).text_size(px(12.0)).font_weight(FontWeight::MEDIUM)),
                Box::new(move |window, cx| on_model(window, cx))))
            .when(enabled(plugins, "stt"), |d| d.child(
                Button::new("composer-mic").ghost()
                    .w(px(32.0)).h(px(32.0)).rounded_md().justify_center()
                    .child(Label::new(if stt_recording { "⏹" } else { "🎤" }).text_size(px(14.0)))
                    .on_click(move |_, window, cx| on_mic(window, cx))))
            .child(Button::new("send").w(px(36.0)).h(px(36.0)).rounded_full().justify_center()
                .bg(t.accent)
                .when(!send_enabled, |b| b.disabled(true))
                .child(Label::new("↑").text_size(px(16.0)).text_color(t.background))
                .on_click(move |_, window, cx| on_send(window, cx))))
}