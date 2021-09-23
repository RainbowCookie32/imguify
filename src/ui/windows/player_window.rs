use crate::ui::AppState;
use crate::spotify::player::PlayerCommand;

use imgui::*;

pub struct PlayerWindow {
    current_track: String,
    current_artist: String,

    next_track: String,
    next_artist: String
}

impl PlayerWindow {
    pub fn init() -> PlayerWindow {
        PlayerWindow {
            current_track: String::from("No tracks loaded"),
            current_artist: String::from("No tracks loaded"),

            next_track: String::from("No tracks loaded"),
            next_artist: String::from("No tracks loaded")
        }
    }

    pub fn draw(&mut self, ui: &Ui, app_state: &mut AppState) {
        if let Some(handler) = app_state.spotify_handler.as_ref() {
            if handler.is_loaded() {
                if let Some(track) = handler.get_current_song() {
                    self.current_track = track.title().to_string();
                    self.current_artist = track.artist().to_string();
                }
    
                if let Some(track) = handler.get_next_song() {
                    self.next_track = track.title().to_string();
                    self.next_artist = track.artist().to_string();
                }
            }
        }

        Window::new("Player").size([420.0, 300.0], Condition::FirstUseEver).build(ui, || {
            ui.text_colored([0.2, 1.0, 0.0, 1.0], "Currently Playing:");
    
            ui.text(&self.current_track);
            ui.text(&self.current_artist);
    
            ui.separator();
    
            ui.text_colored([1.0, 0.5, 0.0, 1.0], "Next Track:");
    
            ui.text(&self.next_track);
            ui.text(&self.next_artist);
    
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
}
