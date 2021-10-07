use std::convert::TryInto;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use zbus::{fdo, dbus_interface};

use crate::spotify::player::{PlayerCommand, PlayerHandler};

const DBUS_NAME: &str = "org.mpris.MediaPlayer2.imguify";

#[cfg(target_os = "linux")]
pub struct MPRISHandler {
    events_tx: Sender<PlayerCommand>,
    player_handler: Arc<Mutex<PlayerHandler>>
}

#[cfg(target_os = "linux")]
#[dbus_interface(name = "org.mpris.MediaPlayer2.Player")]
impl MPRISHandler {
    fn next(&self) {
        if let Err(error) = self.events_tx.send(PlayerCommand::SkipTrack) {
            println!("{}", error.to_string());
        }
    }

    fn previous(&self) {
        if let Err(error) = self.events_tx.send(PlayerCommand::PrevTrack) {
            println!("{}", error.to_string());
        }
    }

    fn pause(&self) {
        if let Err(error) = self.events_tx.send(PlayerCommand::PlayPause) {
            println!("{}", error.to_string());
        }
    }

    fn play_pause(&self) {
        if let Err(error) = self.events_tx.send(PlayerCommand::PlayPause) {
            println!("{}", error.to_string());
        }
    }

    fn stop(&self) {

    }

    fn play(&self) {
        if let Err(error) = self.events_tx.send(PlayerCommand::PlayPause) {
            println!("{}", error.to_string());
        }
    }

    #[dbus_interface(property, name = "PlaybackStatus")]
    fn playback_status(&self) -> String {
        if let Ok(lock) = self.player_handler.lock() {
            if lock.track_playing {
                String::from("Playing")
            }
            else {
                String::from("Paused")
            }
        }
        else {
            String::from("Stopped")
        }
    }

    #[dbus_interface(property, name = "Volume")]
    fn volume(&self) -> f32 {
        1.0
    }

    #[dbus_interface(property, name = "LoopStatus")]
    fn loop_status(&self) -> String {
        String::from("Playlist")
    }

    #[dbus_interface(property, name = "Rate")]
    fn rate(&self) -> f32 {
        1.0
    }

    #[dbus_interface(property, name = "Shuffle")]
    fn shuffle(&self) -> bool {
        true
    }

    #[dbus_interface(property, name = "CanGoNext")]
    fn can_go_next(&self) -> bool {
        true
    }

    #[dbus_interface(property, name = "CanGoPrevious")]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[dbus_interface(property, name = "CanPlay")]
    fn can_play(&self) -> bool {
        true
    }

    #[dbus_interface(property, name = "CanPause")]
    fn can_pause(&self) -> bool {
        true
    }

    #[dbus_interface(property, name = "CanSeek")]
    fn can_seek(&self) -> bool {
        false
    }

    #[dbus_interface(property, name = "CanControl")]
    fn can_control(&self) -> bool {
        true
    }
}

#[cfg(target_os = "linux")]
pub fn init_connection(events_tx: Sender<PlayerCommand>, player_handler: Arc<Mutex<PlayerHandler>>) {
    if let Ok(connection) = zbus::Connection::new_session() {
        std::thread::spawn(move || {
            let events_tx = events_tx;
    
            if let Ok(proxy) = fdo::DBusProxy::new(&connection) {
                if proxy.request_name(DBUS_NAME, fdo::RequestNameFlags::ReplaceExisting.into()).is_ok() {
                    let iface = MPRISHandler { events_tx, player_handler };
                    let mut object_server = zbus::ObjectServer::new(&connection);
                    
                    if object_server.at(&"/org/mpris/MediaPlayer2".try_into().unwrap(), iface).is_ok() {
                        loop {
                            if let Err(err) = object_server.try_handle_next() {
                                println!("{}", err);
                            }

                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                }
            }
        });
    }
}
