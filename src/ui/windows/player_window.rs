use crate::ui::AppState;
use crate::spotify::player::PlayerCommand;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    Window::new(im_str!("Player")).size([420.0, 300.0], Condition::FirstUseEver).build(&ui, || {
        ui.text_colored([0.2, 1.0, 0.0, 1.0], im_str!("Currently Playing:"));
        ui.separator();

        if let Some(handler) = app_state.spotify_handler.as_ref() {
            if handler.get_playback_status() {
                if let Some(track) = handler.get_current_song() {
                    ui.text(format!("{}", track.title()));
                    ui.text(format!("{}", track.artist()));
                }
                else {
                    ui.text("No data");
                    ui.text("No data");
                }
            }
            else {
                ui.text("Player's empty.");
                ui.text("Waiting for user...");
            }
        }
        else {
            ui.text("The sound of silence.");
            ui.text("Brought to you by... a bug!");
        }

        ui.separator();

        if ui.button(im_str!("«"), [0.0, 0.0]) {
            if let Some(tx) = app_state.player_tx.as_ref() {
                if let Err(error) = tx.send(PlayerCommand::PrevTrack) {
                    println!("{}", error.to_string());
                }
            }
        }

        ui.same_line(0.0);

        if ui.button(im_str!("►"), [0.0, 0.0]) {
            if let Some(tx) = app_state.player_tx.as_ref() {
                if let Err(error) = tx.send(PlayerCommand::PlayPause) {
                    println!("{}", error.to_string());
                }
            }
        }

        ui.same_line(0.0);

        if ui.button(im_str!("»"), [0.0, 0.0]) {
            if let Some(tx) = app_state.player_tx.as_ref() {
                if let Err(error) = tx.send(PlayerCommand::SkipTrack) {
                    println!("{}", error.to_string());
                }
            }
        }
    });
}