mod app;
mod layout;
mod pane;
mod session;

fn main() -> anyhow::Result<()> {
    app::run()
}
