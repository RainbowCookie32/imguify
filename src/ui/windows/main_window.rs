use crate::ui::AppState;
use crate::spotify::player::PlayerCommand;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    if let Some(handler) = app_state.spotify_handler.as_mut() {
        if handler.get_playlists_names().is_empty() {
            handler.fetch_user_playlists();
        }
    }
    
    Window::new(im_str!("Main Window")).size([800.0, 500.0], Condition::FirstUseEver).build(&ui, || {
        let mut show_tree_separator = true;

        ui.text_colored([0.0, 1.0, 0.0, 1.0], format!("Connected to Spotify as {}", app_state.login_state.username));
        ui.separator();

        TreeNode::new(im_str!("User Playlists")).build(&ui, || {
            if let Some(handler) = app_state.spotify_handler.as_mut() {
                let plists = handler.get_playlists_names();

                if !plists.is_empty() {
                    show_tree_separator = false;
                }

                for (idx, plist) in plists.iter().enumerate() {
                    ui.text(plist);
                    ui.same_line(200.0);
    
                    let label = ImString::from(format!("Play##{}", plist));
                    if ui.button(&label, [40.0, 20.0]) {
                        if let Some(tx) = app_state.player_tx.as_ref() {
                            // A bit of a gamble, but should be fine.
                            let plist = handler.get_playlist(idx).unwrap();
                            let api_handler = handler.get_api_handler();
                            
                            if let Err(error) = tx.send(PlayerCommand::StartPlaylist(plist.clone())) {
                                println!("{}", error.to_string());
                            }
    
                            std::thread::spawn(move || {
                                plist.fetch_data(api_handler);
                            });

                            app_state.player_state.show = true;
                        }
                    }
                
                    ui.same_line(250.0);
    
                    let label = ImString::from(format!("View##{}", plist));
                    if ui.button(&label, [40.0, 20.0]) {
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
                    
                    ui.separator();
                }
            }
        });

        if show_tree_separator {
            ui.separator();
        }

        if ui.button(im_str!("Search in Spotify"), [0.0, 0.0]) {
            app_state.show_search_window = true;
        }
    });
}