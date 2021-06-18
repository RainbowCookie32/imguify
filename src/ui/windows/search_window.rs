use crate::ui::AppState;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    Window::new(im_str!("Search")).size([800.0, 500.0], Condition::FirstUseEver).build(&ui, || {
        if ui.input_text(im_str!("Search Query"), &mut app_state.search_query).enter_returns_true(true).resize_buffer(true).build() {
            if let Some(handler) = app_state.spotify_handler.as_ref() {
                app_state.search_results_artists = handler.search_artists(app_state.search_query.to_string());
                app_state.search_results_tracks = handler.search_tracks(app_state.search_query.to_string());
            }
        }

        ui.separator();

        ui.text_colored([0.0, 1.0, 0.0, 1.0], "Artists");
        ui.separator();

        ui.columns(3, im_str!("results_columns_artists"), true);
        
        for artist in app_state.search_results_artists.iter() {
            ui.text(format!("{}", artist.name));
            ui.next_column();

            ui.text(format!("{} followers", artist.followers.get("total").unwrap().as_ref().unwrap()));
            ui.next_column();

            let label = ImString::from(format!("View##{}", artist.id));
            if ui.button(&label, [0.0, 0.0]) {
                if let Some(handler) = app_state.spotify_handler.as_mut() {
                    app_state.show_artist_window = true;
                    app_state.search_artist_page_tracks = handler.get_artist_data(artist.id.clone());
                }
            }

            ui.next_column();
        }

        ui.columns(1, im_str!("yeet"), false);

        ui.separator();
        ui.text_colored([0.0, 1.0, 0.0, 1.0], "Tracks");
        ui.separator();

        ui.columns(4, im_str!("results_columns_tracks"), true);

        for track in app_state.search_results_tracks.iter() {
            ui.text(format!("{}", track.name));
            ui.next_column();

            ui.text(format!("{}", track.artists[0].name));
            ui.next_column();

            ui.text(format!("{}", track.album.name));
            ui.next_column();

            if let Some(id) = track.id.as_ref() {
                let label = ImString::from(format!("Play##{}", id));

                if ui.button(&label, [0.0, 0.0]) {
                    if let Some(handler) = app_state.spotify_handler.as_mut() {
                        if let Some(track) = handler.get_api_handler().get_track(id.clone()) {
                            app_state.show_player_window = true;
                            handler.play_single_track(track);
                        }
                    }
                }
            }

            ui.next_column();
        }
    });
}