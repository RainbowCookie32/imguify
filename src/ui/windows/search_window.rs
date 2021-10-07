use imgui::*;
use librespot::core::spotify_id::SpotifyId;

use crate::ui::AppState;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    let mut show_window = app_state.show_search_window;

    Window::new("Search").size([800.0, 500.0], Condition::FirstUseEver).opened(&mut show_window).build(ui, || {
        if ui.input_text("Search Query", &mut app_state.search_query).enter_returns_true(true).build() {
            if let Some(handler) = app_state.spotify_handler.as_ref() {
                app_state.search_results_artists = handler.search_artists(app_state.search_query.to_string());
                app_state.search_results_tracks = handler.search_tracks(app_state.search_query.to_string());
            }
        }

        ui.separator();

        ui.text_colored([0.0, 1.0, 0.0, 1.0], "Artists");
        ui.separator();

        ui.columns(3, "results_columns_artists", true);
        
        for artist in app_state.search_results_artists.iter() {
            ui.text(artist.name.to_string());
            ui.next_column();

            ui.text(format!("{} followers", artist.followers.get("total").unwrap().as_ref().unwrap()));
            ui.next_column();

            if ui.button(format!("View##{}", artist.id)) {
                if let Some(handler) = app_state.spotify_handler.as_mut() {
                    app_state.show_artist_window = true;
                    app_state.search_artist_page_tracks = handler.get_artist_data(artist.id.clone());
                }
            }

            ui.next_column();
        }

        ui.columns(1, "yeet", false);

        ui.separator();
        ui.text_colored([0.0, 1.0, 0.0, 1.0], "Tracks");
        ui.separator();

        ui.columns(4, "results_columns_tracks", true);

        for track in app_state.search_results_tracks.iter() {
            ui.text(track.name.to_string());
            ui.next_column();

            ui.text(track.artists[0].name.to_string());
            ui.next_column();

            ui.text(track.album.name.to_string());
            ui.next_column();

            if let Some(id) = track.id.as_ref() {
                if ui.button(format!("Play##{}", id)) {
                    if let Some(handler) = app_state.spotify_handler.as_mut() {
                        let id = SpotifyId::from_base62(id).unwrap();
                        
                        handler.play_single_track(id);
                        app_state.show_player_window = true;
                    }
                }
            }

            ui.next_column();
        }
    });

    app_state.show_search_window = show_window;
}