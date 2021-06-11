use std::convert::TryInto;
use std::sync::mpsc::Sender;

use zbus::{fdo, dbus_interface};

use crate::spotify::player::PlayerCommand;

const DBUS_NAME: &'static str = "org.mpris.MediaPlayer2.imguify";

#[cfg(target_os = "linux")]
pub struct MPRISHandler {
    events_tx: Sender<PlayerCommand>
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
}

#[cfg(target_os = "linux")]
pub fn init_connection(events_tx: Sender<PlayerCommand>) {
    std::thread::spawn(move || {
        let events_tx = events_tx;

        if let Ok(connection) = zbus::Connection::new_session() {
            if let Ok(proxy) = fdo::DBusProxy::new(&connection) {
                if proxy.request_name(DBUS_NAME, fdo::RequestNameFlags::ReplaceExisting.into()).is_ok() {
                    let iface = MPRISHandler { events_tx };
                    let mut object_server = zbus::ObjectServer::new(&connection);
                    
                    if object_server.at(&"/org/mpris/MediaPlayer2".try_into().unwrap(), iface).is_ok() {
                        loop {
                            if let Err(err) = object_server.try_handle_next() {
                                println!("{}", err);
                            }
                        }
                    }
                }
            }
        }
    });
}
