use std::sync::Arc;

use crate::ui::AppState;
use crate::spotify::PlaylistData;

use imgui::*;

pub struct PlaylistWindow {
    playlist: Arc<PlaylistData>
}

impl PlaylistWindow {
    pub fn init(playlist: Arc<PlaylistData>) -> PlaylistWindow {
        PlaylistWindow {
            playlist
        }
    }

    pub fn draw(&mut self, ui: &Ui, app_state: &mut AppState) {
        let mut show_window = app_state.show_playlist_window;

        Window::new("Playlist").size([800.0, 500.0], Condition::FirstUseEver).opened(&mut show_window).build(ui, || {
            let mut play_song = None;
            let mut remove_song = None;
    
            if let Ok(mut entries) = self.playlist.entries_data().try_write() {
                let token = ui.begin_table_header_with_flags(
                    "Playlist Table",
                    [
                        TableColumnSetup::new("Title"),
                        TableColumnSetup::new("Artist"),
                        TableColumnSetup::new("Duration"),
                        TableColumnSetup::new("Actions")
                    ],
                    TableFlags::BORDERS | TableFlags::RESIZABLE | TableFlags::SORTABLE
                );
                
                if let Some(_t) = token {
                    if let Some(data) = ui.table_sort_specs_mut() {
                        data.conditional_sort(|specs| {
                            if let Some(spec) = specs.iter().next() {
                                if let Some(direction) = spec.sort_direction() {
                                    match direction {
                                        TableSortDirection::Ascending => match spec.column_idx() {
                                            0 => entries.sort_by_key(|e| e.title().clone()),
                                            1 => entries.sort_by_key(|e| e.artist().clone()),
                                            2 => entries.sort_by_key(|e| *e.duration()),
                                            _ => {}
                                        }
                                        TableSortDirection::Descending => match spec.column_idx() {
                                            0 => entries.sort_by_key(|e| std::cmp::Reverse(e.title().clone())),
                                            1 => entries.sort_by_key(|e| std::cmp::Reverse(e.artist().clone())),
                                            2 => entries.sort_by_key(|e| std::cmp::Reverse(*e.duration())),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        });
                    }

                    for entry in entries.iter() {
                        ui.table_next_column();
                        ui.text(entry.title());

                        ui.table_next_column();
                        ui.text(entry.artist());
    
                        let seconds = entry.duration() / 1000;
                        let minutes = seconds / 60;
                        let seconds = seconds % 60;
    
                        ui.table_next_column();
                        ui.text(format!("{}:{:02}", minutes, seconds));

                        ui.table_next_column();
                        if ui.button(format!("Play##{}", entry.id())) {
                            play_song = Some(entry.id().clone());
                        }
    
                        ui.same_line();
                        if ui.button(format!("Remove##{}", entry.id())) {
                            remove_song = Some(entry.id().clone());
                        }
                    }
                }
            }

            if let Some(track_to_play) = play_song {
                if let Some(handler) = app_state.spotify_handler.as_mut() {
                    app_state.show_player_window = true;
                    handler.play_song_on_playlist(self.playlist.id().to_base62(), &track_to_play);
                }
            }

            if let Some(track_to_remove) = remove_song {
                if let Some(handler) = app_state.spotify_handler.as_mut() {
                    handler.remove_track_from_playlist(&self.playlist.id().to_base62(), &track_to_remove);
                }
            }
        });
    
        app_state.show_playlist_window = show_window;
    }
}
