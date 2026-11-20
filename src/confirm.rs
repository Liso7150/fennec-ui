//! The shared destructive-action confirmation. Anything that throws work
//! away — a task, a workspace — asks through this instead of the platform's
//! own dialog, which arrives unthemed and can't be styled.
//!
//! The native shell and the PWA agree on the words: a title, one line saying
//! what is lost, the name of the thing in a mono inset, then Cancel and the
//! colored action.

use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme, WindowExt,
    button::{Button, ButtonVariants},
    dialog::DialogContent,
    label::Label,
};
use gpui_kit::prelude::*;
use gpui_kit::*;

pub struct Confirm {
    /// Dialog heading, e.g. `Delete Task`.
    pub title: &'static str,
    /// What happens if the user goes through with it.
    pub detail: &'static str,
    /// The thing being acted on, shown in a monospace inset.
    pub subject: String,
    /// Verb on the action button, e.g. `Delete`.
    pub action: &'static str,
    /// Danger paints the action red; false keeps it accent-colored.
    pub danger: bool,
}

type Action = Rc<dyn Fn(&mut App)>;

pub fn open_confirm<F>(window: &mut Window, cx: &mut App, confirm: Confirm, on_action: F)
where
    F: Fn(&mut App) + 'static,
{
    let Confirm { title, detail, subject, action, danger } = confirm;
    // The builder re-runs whenever the dialog layer redraws, so everything it
    // reads has to survive being cloned out of the closure.
    let subject: Rc<String> = Rc::new(subject);
    let on_action: Action = Rc::new(on_action);

    window.open_dialog(cx, move |dialog, _, _| {
        let body = body_builder(detail, subject.clone());
        let footer = footer_row(danger, action, on_action.clone());
        dialog
            .title(title)
            .content(body)
            .footer(footer)
            .on_close(|_, window, cx| { window.close_dialog(cx); })
    });
}

fn body_builder(
    detail: &'static str,
    subject: Rc<String>,
) -> impl Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static {
    move |content, _window, cx| {
        let t = cx.theme().clone();
        let inset = div().p_3().rounded_md().bg(t.muted).border_1().border_color(t.border)
            .child(Label::new(subject.as_str()).text_size(px(12.0))
                .font_family("monospace").text_color(t.foreground));
        content
            .flex().flex_col().gap_3()
            .child(Label::new(detail).text_size(px(12.5)).text_color(t.muted_foreground))
            .child(inset)
    }
}

fn footer_row(danger: bool, action: &'static str, on_action: Action) -> Div {
    let cancel = Button::new("fennec-confirm-cancel").label("Cancel").ghost()
        .on_click(|_, window, cx| { window.close_dialog(cx); });
    let verdict = Button::new("fennec-confirm-action").label(action);
    let verdict = if danger { verdict.danger() } else { verdict.primary() };
    let verdict = verdict.on_click(move |_, window, cx| {
        on_action(cx);
        window.close_dialog(cx);
    });
    div().flex().justify_end().gap_2().p_3().child(cancel).child(verdict)
}
