use crate::ui::AppState;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    Window::new(im_str!("Search")).size([800.0, 500.0], Condition::FirstUseEver).build(&ui, || {
        if ui.input_text(im_str!("Search Query"), &mut app_state.search_query).enter_returns_true(true).resize_buffer(true).build() {
            if let Some(handler) = app_state.spotify_handler.as_ref() {
                app_state.search_results = handler.search_artists(app_state.search_query.to_string());
            }
        }

        ui.text("Artists");
        ui.separator();

        ui.columns(3, im_str!("results_columns"), true);
        
        for artist in app_state.search_results.iter() {
            ui.text(format!("{}", artist.name));
            ui.next_column();

            ui.text(format!("{} followers", artist.followers.get("total").unwrap().as_ref().unwrap()));
            ui.next_column();

            let label = ImString::from(format!("View##{}", artist.id));
            if ui.button(&label, [0.0, 0.0]) {
                if let Some(handler) = app_state.spotify_handler.as_mut() {
                    app_state.show_artist_window = true;
                    app_state.search_artist_tracks = handler.get_artist_data(artist.id.clone());
                }
            }

            ui.next_column();
        }

        ui.separator();

        ui.text("Tracks");
    });
}