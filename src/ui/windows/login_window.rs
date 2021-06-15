use crate::ui::AppState;
use crate::spotify::SpotifyHandler;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    if app_state.login_failed {
        ui.open_popup(im_str!("Couldn't log in"));
    }

    ui.popup_modal(im_str!("Couldn't log in")).build(|| {
        ui.text("Error logging in, please try again.");

        if ui.button(im_str!("Ok"), [0.0, 0.0]) {
            ui.close_current_popup();
            app_state.login_failed = false;
        }
    });

    Window::new(im_str!("Login to Spotify")).size([350.0, 110.0], Condition::Always).build(&ui, || {
        let submitted = {
            ui.input_text(im_str!("Username"), &mut app_state.username)
                .resize_buffer(true)
                .enter_returns_true(true)
                .build()
            ||
            ui.input_text(im_str!("Password"), &mut app_state.password)
                .password(true)
                .resize_buffer(true)
                .enter_returns_true(true)
                .build()
            ||
            ui.button(im_str!("Login"), [45.0, 20.0])
        };

        if submitted {
            let username = app_state.username.to_string();
            let password = app_state.password.to_string();

            if username.is_empty() || password.is_empty() {
                app_state.login_failed = true;
                return;
            }

            let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();

            if let Some(handler) = SpotifyHandler::init(username, password, cmd_tx.clone(), cmd_rx) {
                app_state.player_tx = Some(cmd_tx);
                app_state.spotify_handler = Some(handler);
            }
            else {
                app_state.login_failed = true;
            }

            app_state.password.clear();
        }
    });
}