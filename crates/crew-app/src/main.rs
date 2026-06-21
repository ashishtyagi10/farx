mod app;
mod layout;
mod session;

fn main() -> anyhow::Result<()> {
    app::run()
}
