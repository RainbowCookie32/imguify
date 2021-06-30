use crate::ui::AppState;

use imgui::*;

pub fn build(ui: &Ui, app_state: &mut AppState) {
    Window::new(im_str!("Playlist")).size([800.0, 500.0], Condition::FirstUseEver).build(&ui, || {
        ui.text("Songs");

        if let Some(plist) = app_state.playlist_data.as_ref() {
            if let Ok(lock) = plist.entries_data().try_read() {
                let data_len = lock.len();
                let entries_len = plist.entries().len();

                if data_len != entries_len {
                    ui.same_line(70.0);
                    ui.text_colored([1.0, 0.1, 0.1, 1.0], format!("Loading songs: {}/{}", data_len, entries_len));
                }

                ui.separator();

                ui.columns(5, im_str!("Columns?"), true);

                for entry in lock.iter() {
                    ui.text(entry.title());
                    ui.next_column();

                    ui.text(entry.artist());
                    ui.next_column();

                    let seconds = entry.duration() / 1000;
                    let minutes = seconds / 60;
                    let seconds = seconds % 60;

                    ui.text(format!("{}:{:02}", minutes, seconds));
                    ui.next_column();

                    let label = ImString::from(format!("Play##{}", entry.id()));
                    if ui.button(&label, [0.0, 0.0]) {
                        if let Some(handler) = app_state.spotify_handler.as_mut() {
                            app_state.player_state.show = true;
                            handler.play_song_on_playlist(plist.id().to_base62(), entry.id());
                        }
                    }

                    ui.next_column();

                    let label = ImString::from(format!("Remove##{}", entry.id()));
                    if ui.button(&label, [0.0, 0.0]) {
                        if let Some(handler) = app_state.spotify_handler.as_mut() {
                            handler.remove_track_from_playlist(&plist.id().to_base62(), entry.id());
                        }
                    }

                    ui.next_column();
                }
            }
        }
    });
}