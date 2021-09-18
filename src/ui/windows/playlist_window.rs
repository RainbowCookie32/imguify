use crate::ui::AppState;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    Window::new("Playlist").size([800.0, 500.0], Condition::FirstUseEver).build(ui, || {
        if let Some(plist) = app_state.playlist_data.as_ref() {
            if let Ok(mut entries) = plist.entries_data().try_write() {
                let token = ui.begin_table_header_with_flags(
                    "Playlist Table",
                    [
                        TableColumnSetup::new("Title"),
                        TableColumnSetup::new("Artist"),
                        TableColumnSetup::new("Duration"),
                        TableColumnSetup::new("Actions")
                    ],
                    TableFlags::RESIZABLE | TableFlags::SORTABLE | TableFlags::BORDERS
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
                            if let Some(handler) = app_state.spotify_handler.as_mut() {
                                app_state.player_state.show = true;
                                handler.play_song_on_playlist(plist.id().to_base62(), entry.id());
                            }
                        }
    
                        ui.same_line();
                        if ui.button(format!("Remove##{}", entry.id())) {
                            if let Some(handler) = app_state.spotify_handler.as_mut() {
                                handler.remove_track_from_playlist(&plist.id().to_base62(), entry.id());
                            }
                        }
                    }
                }
            }
        }
    });
}