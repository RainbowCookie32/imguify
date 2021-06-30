use crate::ui::AppState;
use crate::spotify::SpotifyHandler;

use imgui::*;

pub struct LoginWindowState {
    pub username: ImString,
    password: ImString,
    
    login_failed: bool,
    save_username: bool,
    keyring_login: bool,

    saved_usernames: Vec<String>
}

impl LoginWindowState {
    pub fn init() -> LoginWindowState {
        let saved_usernames = {
            if let Some(mut cache_path) = dirs::cache_dir() {
                cache_path.push("imguify/data/usernames");

                if let Ok(data) = std::fs::read_to_string(cache_path) {
                    data.lines().map(|s| s.to_string()).collect()
                }
                else {
                    Vec::new()
                }
            }
            else {
                Vec::new()
            }
        };

        LoginWindowState {
            username: ImString::new(""),
            password: ImString::new(""),

            login_failed: false,
            save_username: false,
            keyring_login: false,

            saved_usernames
        }
    }
}

pub fn build(ui: &Ui, app_state: &mut AppState) {
    if app_state.login_state.login_failed {
        ui.open_popup(im_str!("Couldn't log in"));
    }

    ui.popup_modal(im_str!("Couldn't log in")).build(|| {
        ui.text("Error logging in, please try again.");

        if ui.button(im_str!("Ok"), [0.0, 0.0]) {
            ui.close_current_popup();
            app_state.login_state.login_failed = false;
        }
    });

    Window::new(im_str!("Login to Spotify")).size([600.0, 130.0], Condition::Always).build(&ui, || {
        ui.columns(2, im_str!("login_cols"), true);

        ui.bullet_text(im_str!("Login"));

        let mut submitted = {
            ui.input_text(im_str!("Username"), &mut app_state.login_state.username)
                .resize_buffer(true)
                .enter_returns_true(true)
                .build()
            ||
            ui.input_text(im_str!("Password"), &mut app_state.login_state.password)
                .password(true)
                .resize_buffer(true)
                .enter_returns_true(true)
                .build()
            ||
            ui.button(im_str!("Login"), [45.0, 20.0])
        };

        ui.same_line(0.0);
        ui.checkbox(im_str!("Remember me"), &mut app_state.login_state.save_username);

        ui.next_column();

        ui.bullet_text(im_str!("Saved usernames"));

        ListBox::new(im_str!("")).size([250.0, 50.0]).build(&ui, || {
            for username in app_state.login_state.saved_usernames.iter() {
                if Selectable::new(&ImString::from(username.to_string())).build(&ui) {
                    let keyring = keyring::Keyring::new("imguify", username);

                    if let Ok(password) = keyring.get_password() {
                        app_state.login_state.keyring_login = true;
                        app_state.login_state.username = ImString::from(username.to_string());
                        app_state.login_state.password = ImString::from(password);

                        submitted = true;
                    }
                }
            }
        });

        if submitted {
            let username = app_state.login_state.username.to_string();
            let password = app_state.login_state.password.to_string();

            if username.is_empty() || password.is_empty() {
                app_state.login_state.login_failed = true;
                return;
            }

            let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();

            if let Some(handler) = SpotifyHandler::init(username.clone(), password.clone(), cmd_tx.clone(), cmd_rx) {
                app_state.player_tx = Some(cmd_tx);
                app_state.spotify_handler = Some(handler);

                if app_state.login_state.save_username && !app_state.login_state.keyring_login {
                    let keyring = keyring::Keyring::new("imguify", &username);

                    if let Err(error) = keyring.set_password(&password) {
                        println!("Error saving data to keyring: {}", error.to_string());
                    }

                    app_state.login_state.saved_usernames.push(username);

                    if let Some(mut cache_path) = dirs::cache_dir() {
                        cache_path.push("imguify/data/usernames");

                        let data = {
                            let mut res = String::new();

                            for username in app_state.login_state.saved_usernames.iter() {
                                res.push_str(username);
                                res.push('\n');
                            }

                            res
                        };
        
                        if let Err(error) = std::fs::write(cache_path, data) {
                            println!("Error saving usernames: {}", error.to_string());
                        }
                    }
                }
            }
            else {
                app_state.login_state.login_failed = true;
            }

            app_state.login_state.password.clear();
        }
    });
}