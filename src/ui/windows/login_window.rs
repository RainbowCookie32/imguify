use crate::ui::AppState;
use crate::spotify::SpotifyHandler;

use imgui::*;

pub struct LoginWindow {
    username: String,
    password: String,
    
    login_failed: bool,
    save_username: bool,
    keyring_login: bool,

    saved_usernames: Vec<String>
}

impl LoginWindow {
    pub fn init() -> LoginWindow {
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

        LoginWindow {
            username: String::new(),
            password: String::new(),

            login_failed: false,
            save_username: false,
            keyring_login: false,

            saved_usernames
        }
    }

    pub fn draw(&mut self, ui: &Ui, app_state: &mut AppState) -> String {
        let mut username_to_use = String::new();

        Window::new("Login to Spotify").size([600.0, 130.0], Condition::Always).build(ui, || {
            ui.columns(2, "login_cols", true);
    
            ui.bullet_text("Login");
    
            let mut submitted = {
                ui.input_text("Username", &mut self.username)
                    .enter_returns_true(true)
                    .build()
                ||
                ui.input_text("Password", &mut self.password)
                    .password(true)
                    .enter_returns_true(true)
                    .build()
                ||
                ui.button("Login")
            };
    
            ui.same_line();
            ui.checkbox("Remember me", &mut self.save_username);
    
            ui.next_column();
    
            ui.bullet_text("Saved usernames");
    
            ListBox::new("").size([250.0, 50.0]).build(ui, || {
                for username in self.saved_usernames.iter() {
                    if Selectable::new(&username).build(ui) {
                        let keyring = keyring::Keyring::new("imguify", username);
    
                        if let Ok(password) = keyring.get_password() {
                            self.keyring_login = true;
                            self.username = username.clone();
                            self.password = password;
    
                            submitted = true;
                        }
                    }
                }
            });
    
            if submitted {
                let username = self.username.clone();
                let password = self.password.clone();
    
                if username.is_empty() || password.is_empty() {
                    self.login_failed = true;
                    return;
                }
    
                let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    
                if let Ok(handler) = SpotifyHandler::init(username.clone(), password.clone(), cmd_tx.clone(), cmd_rx) {
                    username_to_use = username.clone();
                    app_state.player_tx = Some(cmd_tx);
                    app_state.spotify_handler = Some(handler);
    
                    if self.save_username && !self.keyring_login {
                        let keyring = keyring::Keyring::new("imguify", &username);
    
                        if let Err(error) = keyring.set_password(&password) {
                            println!("Error saving data to keyring: {}", error.to_string());
                        }
    
                        self.saved_usernames.push(username);
    
                        if let Some(mut cache_path) = dirs::cache_dir() {
                            cache_path.push("imguify/data/usernames");
    
                            let data = {
                                let mut res = String::new();
    
                                for username in self.saved_usernames.iter() {
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
                    self.login_failed = true;
                }
    
                self.password.clear();
            }
        });

        if self.login_failed {
            ui.open_popup("Couldn't log in");
        }
    
        PopupModal::new("Couldn't log in").build(ui, || {
            ui.text("Error logging in, please try again.");
    
            if ui.button("Ok") {
                ui.close_current_popup();
                self.login_failed = false;
            }
        });

        username_to_use
    }
}
