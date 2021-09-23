use crate::ui::AppState;
use crate::spotify::cache::TrackCacheUnit;

use imgui::*;

pub struct ArtistWindow {
    artist_name: String,
    artist_tracks: Vec<TrackCacheUnit>
}

impl ArtistWindow {
    pub fn init(artist_name: String, artist_tracks: Vec<TrackCacheUnit>) -> ArtistWindow {
        ArtistWindow {
            artist_name,
            artist_tracks
        }
    }

    pub fn draw(&mut self, ui: &Ui, app_state: &mut AppState) {
        let mut show_window = app_state.show_artist_window;

        Window::new(&self.artist_name).size([420.0, 300.0], Condition::FirstUseEver).opened(&mut show_window).build(ui, || {
            ui.bullet_text("Tracks");

            let token = ui.begin_table_header_with_flags(
                "Artist Tracks",
                [
                    TableColumnSetup::new("Title"),
                    TableColumnSetup::new("Artist"),
                    TableColumnSetup::new("Duration"),
                    TableColumnSetup::new("Actions")
                ],
                TableFlags::BORDERS | TableFlags::RESIZABLE
            );
            
            if let Some(_t) = token {
                for entry in self.artist_tracks.iter() {
                    let seconds = entry.duration() / 1000;
                    let minutes = seconds / 60;
                    let seconds = seconds % 60;
                    
                    ui.table_next_column();
                    ui.text(entry.name());

                    ui.table_next_column();
                    ui.text(&entry.artists()[0]);

                    ui.table_next_column();
                    ui.text(format!("{}:{:02}", minutes, seconds));

                    ui.table_next_column();

                    if ui.button(format!("Play##{}", entry.id())) {
                        if let Some(handler) = app_state.spotify_handler.as_mut() {
                            app_state.show_player_window = true;
                            handler.play_single_track(entry.clone());
                        }
                    }
                }
            }
        });
    
        app_state.show_artist_window = show_window;
    }
}
