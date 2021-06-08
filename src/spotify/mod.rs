pub mod api;
pub mod cache;
pub mod player;

use api::SpotifyAPIHandler;
use cache::APICacheHandler;
use player::{PlayerCommand, PlayerHandler};

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};

use tokio::runtime::Runtime;

use librespot::core::cache::Cache;
use librespot::core::session::Session;
use librespot::core::config::SessionConfig;
use librespot::core::spotify_id::SpotifyId;
use librespot::core::authentication::Credentials;

use librespot::metadata::{Metadata, Playlist};

use rspotify::model::track::FullTrack;
use rspotify::model::artist::FullArtist;

use cache::TrackCacheUnit;

pub struct SpotifyHandler {
    rt: Runtime,
    spotify_session: Session,

    api_handler: Arc<SpotifyAPIHandler>,
    playlist_data: Vec<Arc<PlaylistData>>,
    player_handler: Arc<Mutex<PlayerHandler>>
}

impl SpotifyHandler {
    pub fn init(username: String, password: String, cmd_rx: Receiver<PlayerCommand>) -> Option<SpotifyHandler> {
        let rt = Runtime::new().unwrap();

        let player_cache_path = format!("{}/imguify/audio", dirs::cache_dir().unwrap().to_str().unwrap());
        let player_cache = Cache::new(None, Some(player_cache_path), None).unwrap();

        let api_cache_handler = Arc::new(Mutex::new(APICacheHandler::init()));

        if let Some(api_handler) = SpotifyAPIHandler::init(api_cache_handler.clone()) {
            let api_handler = Arc::new(api_handler);

            let mut session_cfg = SessionConfig::default();
            session_cfg.device_id = String::from("imguify-cookie");

            let credentials = Credentials::with_password(username, password);

            if let Ok(session) = rt.block_on(Session::connect(session_cfg, credentials, Some(player_cache))) {
                let spotify_session = session;
                let player_handler = PlayerHandler::init(spotify_session.clone(), cmd_rx);
                
                return Some(
                    SpotifyHandler {
                        rt,
                        spotify_session,
                        
                        api_handler,
                        playlist_data: Vec::new(),
                        player_handler
                    }
                )
            }
        }

        None
    }

    pub fn get_playlist(&mut self, plist: usize) -> Option<Arc<PlaylistData>> {
        if let Some(plist) = self.playlist_data.get(plist) {
            Some(plist.clone())
        }
        else {
            None
        }
    }

    pub fn get_playlists_names(&self) -> Vec<String> {
        let mut results = Vec::new();
        
        for playlist in self.playlist_data.iter() {
            results.push(format!("{} - {} tracks", playlist.title, playlist.entries.len()));
        }

        results
    }

    pub fn get_current_song(&self) -> Option<PlaylistEntry> {
        if let Ok(lock) = self.player_handler.try_lock() {
            lock.get_current_song()
        }
        else {
            None
        }
    }

    pub fn get_playback_status(&self) -> bool {
        if let Ok(lock) = self.player_handler.try_lock() {
            lock.is_queue_loaded()
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

                if let Ok(list) = self.rt.block_on(Playlist::get(&self.spotify_session, id.clone())) {
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

    pub fn play_song_on_playlist(&mut self, playlist: String, track: &String) {
        if let Some(plist) = self.playlist_data.iter().find(|p| p.id().to_base62() == playlist) {
            if let Ok(mut lock) = self.player_handler.lock() {
                let sid = SpotifyId::from_base62(track).unwrap();

                lock.play_track_from_playlist(plist.clone(), sid);
            }
        }
    }

    pub fn remove_track_from_playlist(&mut self, playlist_id: &String, track_id: &String) {
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
                if let Some(data) = self.api_handler.get_album(album.id.unwrap()) {
                    for track in data.tracks() {
                        if let Some(track) = self.api_handler.get_track(track.clone()) {
                            results.push(track);
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
            if let Some(track_data) = api_handler.get_track(id.to_base62()) {
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

    /// Get a reference to the playlist data's entries.
    pub fn entries(&self) -> &Vec<SpotifyId> {
        &self.entries
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
