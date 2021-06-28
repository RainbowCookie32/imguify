use std::convert::{TryFrom, TryInto};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::collections::HashMap;

use zvariant::*;
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

    #[dbus_interface(property, name = "Metadata")]
    fn metadata(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();

        if let Ok(lock) = self.player_handler.lock() {
            if let Some(track) = lock.get_current_song() {
                let title = Value::Str(Str::from(track.title().clone()));
                let artist = Value::Array(Array::from(vec![track.artist().clone()]));
                let track_id = Value::ObjectPath(ObjectPath::try_from("/org/mpris/MediaPlayer2/imguify").unwrap());
                let track_length = Value::U64(*track.duration() as u64 * 1000);

                map.insert(String::from("mpris:trackid"), track_id);
                map.insert(String::from("mpris:length"), track_length);
                map.insert(String::from("xesam:title"), title);
                map.insert(String::from("xesam:artist"), artist);
                map.insert(String::from("xesam:album"), Value::Str(Str::from("")));
            }
        }

        map
    }

    pub fn update_metadata(&self) -> zbus::Result<()> {
        let invalidated: Vec<String> = vec![];
        let mut changed = HashMap::new();
        let mut metadata_dict = Dict::new(Str::signature(), Value::signature());

        if let Ok(lock) = self.player_handler.lock() {
            if let Some(track) = lock.get_current_song() {
                let title = Value::Value(Box::new(Value::Str(Str::from(track.title().clone()))));
                let artist = Value::Value(Box::new(Value::Array(Array::from(vec![track.artist().clone()]))));
                let track_id = Value::Value(Box::new(Value::ObjectPath(ObjectPath::try_from("/org/mpris/MediaPlayer2/imguify").unwrap())));
                let track_length = Value::Value(Box::new(Value::U64(*track.duration() as u64 * 1000)));

                metadata_dict.append(Value::Str(Str::from("mpris:trackid")), track_id).unwrap();
                metadata_dict.append(Value::Str(Str::from("mpris:length")), track_length).unwrap();
                metadata_dict.append(Value::Str(Str::from("xesam:title")), title).unwrap();
                metadata_dict.append(Value::Str(Str::from("xesam:artist")), artist.clone()).unwrap();
                metadata_dict.append(Value::Str(Str::from("xesam:albumArtist")), artist).unwrap();
            }
        }

        changed.insert("Metadata", Value::from(metadata_dict));

        zbus::ObjectServer::local_node_emit_signal(
            None,
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
            &("org.mpris.MediaPlayer2.Player", changed, invalidated)
        )
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

                            object_server.with(&"/org/mpris/MediaPlayer2".try_into().unwrap(), |i: &MPRISHandler| {
                                i.update_metadata()
                            }).unwrap();
                        }
                    }
                }
            }
        });
    }
}
