use crate::ui::AppState;
use crate::spotify::player::PlayerCommand;

use imgui::*;

pub struct PlayerWindowState {
    pub show: bool,

    current_track: String,
    current_artist: String,

    next_track: String,
    next_artist: String
}

impl PlayerWindowState {
    pub fn init() -> PlayerWindowState {
        PlayerWindowState {
            show: false,

            current_track: String::from("No tracks loaded"),
            current_artist: String::from("No tracks loaded"),

            next_track: String::from("No tracks loaded"),
            next_artist: String::from("No tracks loaded")
        }
    }
}

pub fn build(ui: &Ui, app_state: &mut AppState) {
    if let Some(handler) = app_state.spotify_handler.as_ref() {
        if handler.is_loaded() {
            if let Some(track) = handler.get_current_song() {
                app_state.player_state.current_track = track.title().to_string();
                app_state.player_state.current_artist = track.artist().to_string();
            }

            if let Some(track) = handler.get_next_song() {
                app_state.player_state.next_track = track.title().to_string();
                app_state.player_state.next_artist = track.artist().to_string();
            }
        }
    }

    Window::new("Player").size([420.0, 300.0], Condition::FirstUseEver).build(ui, || {
        ui.text_colored([0.2, 1.0, 0.0, 1.0], "Currently Playing:");

        ui.text(app_state.player_state.current_track.to_string());
        ui.text(app_state.player_state.current_artist.to_string());

        ui.separator();

        ui.text_colored([1.0, 0.5, 0.0, 1.0], "Next Track:");

        ui.text(app_state.player_state.next_track.to_string());
        ui.text(app_state.player_state.next_artist.to_string());

        ui.separator();

        if ui.button("«") {
            if let Some(tx) = app_state.player_tx.as_ref() {
                if let Err(error) = tx.send(PlayerCommand::PrevTrack) {
                    println!("{}", error.to_string());
                }
            }
        }

        ui.same_line();

        let label = {
            if let Some(handler) = app_state.spotify_handler.as_ref() {
                if handler.is_playing() {
                    "Pause"
                }
                else {
                    "Play"
                }
            }
            else {
                "Play"
            }
        };

        if ui.button(label) {
            if let Some(tx) = app_state.player_tx.as_ref() {
                if let Err(error) = tx.send(PlayerCommand::PlayPause) {
                    println!("{}", error.to_string());
                }
            }
        }

        ui.same_line();

        if ui.button("»") {
            if let Some(tx) = app_state.player_tx.as_ref() {
                if let Err(error) = tx.send(PlayerCommand::SkipTrack) {
                    println!("{}", error.to_string());
                }
            }
        }
    });
}