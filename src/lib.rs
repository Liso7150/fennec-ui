//! fennec-ui — shared presentational code for the Fennec gpui-kit shells.
//!
//! Both native GUIs (`fennec-desktop-gui` and `fennec-agent-gui`) render the
//! same workflow: theme presets, the model's markdown, user/AI message rows,
//! action/attachment/edit cards, the task-queue strip, the My Attention rail,
//! and the workspace creation dialog. Those pieces live here once; app state,
//! backend dispatch, and persistence stay in each app.

pub mod attention;
pub mod controls;
pub mod inspector;
pub mod markdown;
pub mod plugin;
pub mod theme;
pub mod widgets;
pub mod workspace_dialog;