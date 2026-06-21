mod app;
mod chat;
mod chatlayout;
pub mod config;
mod handler;
mod layout;
mod pane;
mod session;
mod spawn;

fn main() -> anyhow::Result<()> {
    handler::run()
}
