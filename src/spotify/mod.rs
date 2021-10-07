#[cfg(target_os = "linux")]
mod dbus;

pub mod api;
pub mod player;

use api::SpotifyAPIHandler;
use player::{PlayerCommand, PlayerHandler};
use api::cache::{APICacheHandler, TrackCacheUnit};

use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc::{Sender, Receiver};

use anyhow::{Context, Result};
use tokio::runtime::Runtime;

use librespot::core::cache::Cache;
use librespot::core::session::Session;
use librespot::core::config::SessionConfig;
use librespot::core::spotify_id::SpotifyId;
use librespot::core::authentication::Credentials;

use librespot::metadata::{Metadata, Playlist};

use rspotify::model::track::FullTrack;
use rspotify::model::artist::FullArtist;

pub struct SpotifyHandler {
    rt: Runtime,
    spotify_session: Session,

    api_handler: Arc<SpotifyAPIHandler>,
    playlist_data: Vec<Arc<PlaylistData>>,
    player_handler: Arc<Mutex<PlayerHandler>>
}

impl SpotifyHandler {
    pub fn init(username: String, password: String, cmd_tx: Sender<PlayerCommand>, cmd_rx: Receiver<PlayerCommand>) -> Result<SpotifyHandler> {
        let rt = Runtime::new().unwrap();
        let cache_path = {
            let mut path = dirs::cache_dir().context("Failed to get system cache path")?;
            path.push("imguify/audio");

            path
        };

        let player_cache = Cache::new(None, Some(cache_path), None)?;
        let api_cache_handler = Arc::new(Mutex::new(APICacheHandler::init()));
        let api_handler = Arc::new(SpotifyAPIHandler::init(api_cache_handler)?);

        let session_cfg = SessionConfig {
            device_id: String::from("imguify-cookie"),
            ..Default::default()
        };

        let credentials = Credentials::with_password(username, password);
        let spotify_session = rt.block_on(Session::connect(session_cfg, credentials, Some(player_cache)))?;
        let player_handler = PlayerHandler::init(spotify_session.clone(), cmd_rx);

        if cfg!(target_os = "linux") {
            dbus::init_connection(cmd_tx, player_handler.clone());
        }

        let spotify_handler = SpotifyHandler {
            rt,
            spotify_session,
            
            api_handler,
            playlist_data: Vec::new(),
            player_handler
        };
        
        Ok(spotify_handler)
    }

    pub fn get_playlist(&mut self, plist: usize) -> Option<Arc<PlaylistData>> {
        self.playlist_data.get(plist).cloned()
    }

    pub fn get_playlists_names(&self) -> Vec<String> {
        let mut results = Vec::new();
        
        for playlist in self.playlist_data.iter() {
            results.push(format!("{} - {} tracks", playlist.title, playlist.entries.len()));
        }

        results
    }

    pub fn get_next_song(&self) -> Option<PlaylistEntry> {
        if let Ok(lock) = self.player_handler.try_lock() {
            lock.get_next_song()
        }
        else {
            None
        }
    }

    pub fn get_current_song(&self) -> Option<PlaylistEntry> {
        if let Ok(lock) = self.player_handler.try_lock() {
            lock.get_current_song()
        }
        else {
            None
        }
    }

    pub fn is_loaded(&self) -> bool {
        if let Ok(lock) = self.player_handler.try_lock() {
            lock.is_queue_loaded()
        }
        else {
            true
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Ok(lock) = self.player_handler.try_lock() {
            lock.track_playing
        }
        else {
            true
        }
    }

    pub fn get_api_handler(&self) -> Arc<SpotifyAPIHandler> {
        self.api_handler.clone()
    }

    pub fn fetch_user_playlists(&mut self) {
        if let Some(playlists) = self.api_handler.get_user_playlists() {
            self.playlist_data.clear();

            for item in playlists.items {
                let id = SpotifyId::from_base62(&item.id).expect("Failed to parse id");

                if let Ok(list) = self.rt.block_on(Playlist::get(&self.spotify_session, id)) {
                    let entries = list.tracks;

                    self.playlist_data.push(Arc::new(
                        PlaylistData {
                            id,
                            title: list.name,

                            entries,
                            entries_data: Arc::new(RwLock::new(Vec::new())),

                            data_fetched: RwLock::new(false),
                            data_fetching: RwLock::new(false)
                        }
                    ));
                }
            }
        }
    }

    pub fn play_single_track(&mut self, track: TrackCacheUnit) {
        if let Ok(mut lock) = self.player_handler.lock() {
            lock.play_single_track(track);
        }
    }

    pub fn play_song_on_playlist(&mut self, playlist: String, track: &str) {
        if let Some(plist) = self.playlist_data.iter().find(|p| p.id().to_base62() == playlist) {
            if let Ok(mut lock) = self.player_handler.lock() {
                if let Ok(track) = SpotifyId::from_base62(track) {
                    lock.play_track_from_playlist(plist.clone(), track);
                }
            }
        }
    }

    pub fn remove_track_from_playlist(&mut self, playlist_id: &str, track_id: &str) {
        if self.api_handler.remove_track_from_playlist(playlist_id, track_id) {
            self.fetch_user_playlists();
        }
    }

    pub fn search_tracks(&self, query: String) -> Vec<FullTrack> {
        if let Some(results) = self.api_handler.search_tracks(query) {
            return results.items;
        }

        Vec::new()
    }

    pub fn search_artists(&self, query: String) -> Vec<FullArtist> {
        if let Some(results) = self.api_handler.search_artists(query) {
            return results.items;
        }

        Vec::new()
    }

    pub fn get_artist_data(&mut self, artist: String) -> Vec<TrackCacheUnit> {
        let mut results = Vec::new();

        if let Some(albums) = self.api_handler.get_all_albums_for_artist(artist) {
            for album in albums.items {
                if let Some(id) = album.id {
                    if let Some(data) = self.api_handler.get_album(id) {
                        for track in data.tracks() {
                            if let Ok(track) = self.api_handler.get_track(track.clone()) {
                                results.push(track);
                            }
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| b.popularity().cmp(a.popularity()));
        results
    }
}

pub struct PlaylistData {
    id: SpotifyId,
    title: String,

    entries: Vec<SpotifyId>,
    entries_data: Arc<RwLock<Vec<PlaylistEntry>>>,

    data_fetched: RwLock<bool>,
    data_fetching: RwLock<bool>
}

impl PlaylistData {
    pub fn fetch_data(&self, api_handler: Arc<SpotifyAPIHandler>) {
        if let (Ok(mut fetching), Ok(fetched)) = (self.data_fetching.write(), self.data_fetched.read()) {
            if *fetched || *fetching {
                return;
            }
            else {
                *fetching = true;
            }
        }

        for id in self.entries.iter() {            
            if let Ok(track_data) = api_handler.get_track(id.to_base62()) {
                if let Ok(mut lock) = self.entries_data.write() {
                    let artist_data = track_data.artists()[0].clone();

                    let entry = PlaylistEntry {
                        track: track_data,
                        artist: artist_data
                    };
    
                    lock.push(entry);
                }
            }
        }

        if let (Ok(mut fetching), Ok(mut fetched)) = (self.data_fetching.write(), self.data_fetched.write()) {
            *fetched = true;
            *fetching = false;
        }
    }

    /// Get a reference to the playlist data's entries data.
    pub fn entries_data(&self) -> &Arc<RwLock<Vec<PlaylistEntry>>> {
        &self.entries_data
    }

    /// Get a reference to the playlist data's id.
    pub fn id(&self) -> &SpotifyId {
        &self.id
    }
}

#[derive(Clone)]
pub struct PlaylistEntry {
    track: TrackCacheUnit,
    artist: String
}

impl PlaylistEntry {
    pub fn id(&self) -> &String {
        self.track.id()
    }

    pub fn title(&self) -> &String {
        self.track.name()
    }

    pub fn artist(&self) -> &String {
        &self.artist
    }

    pub fn duration(&self) -> &u32 {
        self.track.duration()
    }
}
