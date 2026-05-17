//! Dispatch routing. The top-level `App::dispatch` (in the parent module's
//! `mod.rs`) walks a chain of per-category routers; the first one that
//! claims the action returns `true` and execution stops.

mod analysis;
mod archives;
mod bulk_ops;
mod clipboard;
mod cmdline;
mod control;
mod file_dialogs;
mod nav;
mod selection;
mod term_palette;
mod tree_nav;
