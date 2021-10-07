use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

use rand::prelude::*;
use futures::FutureExt;

use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::playback::audio_backend;
use librespot::playback::player::{Player, PlayerEvent};
use librespot::playback::config::{AudioFormat, Bitrate, PlayerConfig};

use tokio::sync::mpsc::UnboundedReceiver;

use crate::spotify::api::cache::TrackCacheUnit;
use crate::spotify::{PlaylistData, PlaylistEntry};

pub enum PlayerCommand {
    PlayPause,
    PrevTrack,
    SkipTrack,

    StartPlaylist(Arc<PlaylistData>)
}

#[derive(Default)]
pub struct PlayerQueue {
    position: usize,
    playlist_id: String,

    tracks: Vec<PlaylistEntry>
}

impl PlayerQueue {
    pub fn fill_data(&mut self, playlist: Arc<PlaylistData>) {
        if self.playlist_id == playlist.id().to_base62() {
            return;
        }

        let mut tracks = {
            if let Ok(data) = playlist.entries_data.read() {
                data.clone()
            }
            else {
                Vec::new()
            }
        };
        
        tracks.shuffle(&mut thread_rng());
        
        self.position = 0;
        self.playlist_id = playlist.id().to_base62();

        self.tracks = tracks;
    }

    pub fn set_position_with_id(&mut self, id: SpotifyId) {
        let result = self.tracks
            .iter()
            .enumerate()
            .find(|(_, p)| *p.id() == id.to_base62())
        ;

        if let Some((pos, _)) = result {
            self.position = pos;
        }
    }

    pub fn reshuffle_tracks(&mut self) {
        self.position = 0;
        self.tracks.shuffle(&mut thread_rng());
    }
}

pub struct PlayerHandler {
    pub player: Player,
    pub player_queue: PlayerQueue,
    
    pub track_playing: bool,

    pub cmd_rx: Receiver<PlayerCommand>,
    pub player_events: UnboundedReceiver<PlayerEvent>
}

impl PlayerHandler {
    pub fn init(session: Session, cmd_rx: Receiver<PlayerCommand>) -> Arc<Mutex<PlayerHandler>> {
        let player_cfg = PlayerConfig {
            bitrate: Bitrate::Bitrate320,
            ..Default::default()
        };

        let backend = audio_backend::find(None).unwrap();
        let (player, _) = Player::new(player_cfg, session, None, move || {
            backend(None, AudioFormat::default())
        });

        let player_events = player.get_player_event_channel();

        let handler = PlayerHandler {
            player,
            player_queue: PlayerQueue::default(),

            track_playing: false,

            cmd_rx,
            player_events
        };

        let handler = Arc::from(Mutex::from(handler));
        let player_handler = handler.clone();

        std::thread::spawn(move || {
            let player = player_handler;

            loop {
                if let Ok(mut lock) = player.lock() {
                    if let Some(Some(event)) = lock.player_events.recv().now_or_never() {
                        lock.handle_player_event(event);
                    }

                    if let Ok(command) = lock.cmd_rx.try_recv() {
                        lock.handle_player_command(command);
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        handler
    }

    fn handle_player_event(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::Stopped { .. } => {
                self.track_playing = false;
            }
            PlayerEvent::Started { .. } => {
                self.track_playing = true;
            }
            PlayerEvent::EndOfTrack { .. } => {
                if self.player_queue.position >= self.player_queue.tracks.len() {
                    self.player_queue.reshuffle_tracks();
                }
                else {
                    self.player_queue.position += 1;
                }

                self.load_track_and_play();
            }
            _ => {}
        }
    }

    fn handle_player_command(&mut self, command: PlayerCommand) {
        match command {
            PlayerCommand::PlayPause => {
                if self.track_playing {
                    self.player.pause();
                    self.track_playing = false;
                }
                else {
                    self.player.play();
                    self.track_playing = true;
                }
            }
            PlayerCommand::PrevTrack => {
                if self.player_queue.position == 0 {
                    self.player_queue.position = self.player_queue.tracks.len() - 1;
                }
                else {
                    self.player_queue.position -= 1;
                }

                self.load_track_and_play();
            }
            PlayerCommand::SkipTrack => {
                if self.player_queue.position >= self.player_queue.tracks.len() {
                    self.player_queue.reshuffle_tracks();
                }
                else {
                    self.player_queue.position += 1;
                }

                self.load_track_and_play();
            }
            PlayerCommand::StartPlaylist(p) => {
                self.player_queue.position = 0;
                self.player_queue.fill_data(p);

                self.load_track_and_play();
            }
        }
    }

    pub fn get_next_song(&self) -> Option<PlaylistEntry> {
        self.player_queue.tracks.get(self.player_queue.position + 1).cloned()
    }

    pub fn get_current_song(&self) -> Option<PlaylistEntry> {
        self.player_queue.tracks.get(self.player_queue.position).cloned()
    }

    pub fn is_queue_loaded(&self) -> bool {
        !self.player_queue.tracks.is_empty()
    }

    pub fn play_single_track(&mut self, track: TrackCacheUnit) {
        self.player_queue.tracks = vec![
            PlaylistEntry {
                track,
                artist: String::new()
            }
        ];

        self.load_track_and_play();
    }

    pub fn play_track_from_playlist(&mut self, playlist: Arc<PlaylistData>, track: SpotifyId) {
        self.player_queue.fill_data(playlist);
        self.player_queue.set_position_with_id(track);

        self.load_track_and_play();
    }

    fn load_track_and_play(&mut self) {
        if let Some(track_id) = self.player_queue.tracks.get(self.player_queue.position) {
            let track_id = track_id.id();

            if let Ok(track_id) = SpotifyId::from_base62(track_id) {
                self.player.load(track_id, true, 0);
                self.player.play();
    
                self.track_playing = true;
            }
        }
    }
}
