mod ui;
mod spotify;

fn main() {
    let system = ui::App::new();

    dotenv::from_filename("tokens.env").expect("Failed to load tokens.env");
    system.render_loop();
}
