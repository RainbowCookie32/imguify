use crate::ui::AppState;
use crate::spotify::player::PlayerCommand;

use imgui::*;

pub struct MainWindow {
    username: String,
    playlists: Vec<String>
}

impl MainWindow {
    pub fn init(username: String, playlists: Vec<String>) -> MainWindow {
        MainWindow {
            username,
            playlists
        }
    }

    pub fn draw(&mut self, ui: &Ui, app_state: &mut AppState) {
        Window::new("Main Window").size([800.0, 500.0], Condition::FirstUseEver).build(ui, || {
            ui.text_colored([0.0, 1.0, 0.0, 1.0], format!("Connected to Spotify as {}", self.username));
            ui.separator();
    
            TreeNode::new("User Playlists").build(ui, || {
                for (idx, plist) in self.playlists.iter().enumerate() {
                    ui.text(plist);
                    ui.same_line_with_pos(200.0);
    
                    if ui.button(format!("Play##{}", plist)) {
                        if let Some(tx) = app_state.player_tx.as_ref() {
                            if let Some(handler) = app_state.spotify_handler.as_mut() {
                                // A bit of a gamble, but should be fine.
                                let plist = handler.get_playlist(idx).unwrap();
                                let api_handler = handler.get_api_handler();
                            
                                if let Err(error) = tx.send(PlayerCommand::StartPlaylist(plist.clone())) {
                                    println!("{}", error.to_string());
                                }
    
                                std::thread::spawn(move || {
                                    plist.fetch_data(api_handler);
                                });

                                app_state.show_player_window = true;
                            }
                        }
                    }
                
                    ui.same_line_with_pos(250.0);
    
                    if ui.button(format!("View##{}", plist)) {
                        if let Some(handler) = app_state.spotify_handler.as_mut() {
                            let playlist = handler.get_playlist(idx);
                            let playlist_fetch = playlist.clone();
    
                            app_state.playlist_data = playlist;
                        
                            if let Some(playlist_fetch) = playlist_fetch {
                                let api_handler = handler.get_api_handler();
    
                                std::thread::spawn(move || {
                                    playlist_fetch.fetch_data(api_handler);
                                });
                            }
    
                            app_state.show_playlist_window = true;
                        }
                    }
                    
                    ui.separator();
                }
            });
    
            if !self.playlists.is_empty() {
                ui.separator();
            }
    
            if ui.button("Search in Spotify") {
                app_state.show_search_window = true;
            }
        });
    }
}
