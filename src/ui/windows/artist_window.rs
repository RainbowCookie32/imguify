use crate::ui::AppState;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    Window::new(im_str!("Artist Info")).size([420.0, 300.0], Condition::FirstUseEver).build(&ui, || {
        ui.columns(4, im_str!("artist_tracks_columns"), true);

        for entry in app_state.search_artist_tracks.iter() {
            ui.text(entry.name());
            ui.next_column();

            ui.text(entry.artists()[0].clone());
            ui.next_column();

            let seconds = entry.duration() / 1000;
            let minutes = seconds / 60;
            let seconds = seconds % 60;

            ui.text(format!("{}:{:02}", minutes, seconds));
            ui.next_column();

            let label = ImString::from(format!("Play##{}", entry.id()));
            if ui.button(&label, [0.0, 0.0]) {
                if let Some(handler) = app_state.spotify_handler.as_mut() {
                    app_state.show_player_window = true;
                    handler.play_single_track(entry.clone());
                }
            }

            ui.next_column();
        }
    });
}