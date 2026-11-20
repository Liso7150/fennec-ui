//! fennec-ui — shared presentational code for the Fennec gpui-kit shell.
//!
//! The native GUI (`fennec-desktop-gui`) renders one workflow: theme presets,
//! the model's markdown, user/AI message rows, action/attachment/edit cards,
//! the collapsed block of actions between two answers, the task-queue strip,
//! the My Attention rail, and the workspace creation dialog. Those pieces live
//! here once; app state, backend dispatch, and persistence stay in the app. The
//! browser PWA in `fennec-coding-agent/web` is the other surface of the same
//! workflow, so the words and the grouping written here are what it mirrors.

pub mod attention;
pub mod confirm;
pub mod controls;
pub mod inspector;
pub mod markdown;
pub mod plugin;
pub mod theme;
pub mod timeline;
pub mod widgets;
pub mod workspace_dialog;