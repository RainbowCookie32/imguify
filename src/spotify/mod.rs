mod cache;
pub mod player;

use cache::DataCacheHandler;
use player::{PlayerCommand, PlayerHandler};

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};

use tokio::runtime::Runtime;

use librespot::core::cache::Cache;
use librespot::core::session::Session;
use librespot::core::config::SessionConfig;
use librespot::core::spotify_id::SpotifyId;
use librespot::core::authentication::Credentials;

use librespot::metadata::{Metadata, Artist, Playlist, Track};

use rspotify::blocking::client::Spotify;
use rspotify::blocking::oauth2::{SpotifyClientCredentials, SpotifyOAuth};

use cache::{ArtistCacheUnit, TrackCacheUnit};

pub struct SpotifyHandler {
    rt: Runtime,
    api_client: Spotify,
    spotify_session: Session,

    playlist_data: Vec<Arc<PlaylistData>>,
    player_handler: Arc<Mutex<PlayerHandler>>,
    cache_handler: Arc<Mutex<DataCacheHandler>>
}

impl SpotifyHandler {
    pub fn init(username: String, password: String, cmd_rx: Receiver<PlayerCommand>) -> Option<SpotifyHandler> {
        let rt = Runtime::new().unwrap();
        let cache_path = format!("{}/imguify/audio", dirs::cache_dir().unwrap().to_str().unwrap());
        let cache = Cache::new(None, Some(cache_path)).unwrap();

        let mut oauth = SpotifyOAuth::default()
            .scope("playlist-read-private")
            .redirect_uri("http://localhost:8888/callback")
            .client_id(&std::env::var("CLIENT_ID").expect("Failed to load CLIENT_ID value."))
            .client_secret(&std::env::var("CLIENT_SECRET").expect("Failed to load CLIENT_SECRET value."))
            .build()
        ;

        if let Some(token) = rspotify::blocking::util::get_token(&mut oauth) {
            let api_credentials = SpotifyClientCredentials::default().token_info(token);
            let api_client = Spotify::default().client_credentials_manager(api_credentials).build();

            let mut session_cfg = SessionConfig::default();
            session_cfg.device_id = String::from("imguify-cookie");

            let credentials = Credentials::with_password(username, password);

            if let Ok(session) = rt.block_on(Session::connect(session_cfg, credentials, Some(cache))) {
                let spotify_session = session;
                let player_handler = PlayerHandler::init(spotify_session.clone(), cmd_rx);
                let cache_handler = Arc::new(Mutex::new(DataCacheHandler::init()));
                
                return Some(
                    SpotifyHandler {
                        rt,
                        api_client,
                        spotify_session,
        
                        playlist_data: Vec::new(),
                        player_handler,
                        cache_handler
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

    pub fn get_cache_handler(&self) -> Arc<Mutex<DataCacheHandler>> {
        self.cache_handler.clone()
    }

    pub fn fetch_user_playlists(&mut self) {
        if let Ok(playlists) = self.api_client.current_user_playlists(5, 0) {
            self.playlist_data.clear();

            for item in playlists.items {
                let id = SpotifyId::from_base62(&item.id).expect("Failed to parse id");

                if let Ok(list) = self.rt.block_on(Playlist::get(&self.spotify_session, id.clone())) {
                    let entries = list.tracks;

                    self.playlist_data.push(Arc::new(
                        PlaylistData {
                            id,
                            title: list.name,
                            session: self.spotify_session.clone(),

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

    pub fn play_song_on_playlist(&mut self, playlist: String, track: &String) {
        if let Some(plist) = self.playlist_data.iter().find(|p| p.id().to_base62() == playlist) {
            if let Ok(mut lock) = self.player_handler.lock() {
                let sid = SpotifyId::from_base62(track).unwrap();

                lock.play_track_from_playlist(plist.clone(), sid);
            }
        }
    }

    pub fn remove_track_from_playlist(&mut self, playlist: String, track: &String) {
        if let Some(_) = self.playlist_data.iter().find(|p| p.id().to_base62() == playlist) {
            let user_id = self.api_client.me().unwrap().id;
            let track_ids = [track.clone()];
            
            if let Ok(_) = self.api_client.user_playlist_remove_all_occurrences_of_tracks(&user_id, &playlist, &track_ids, None) {
                self.fetch_user_playlists();
            }
        }
    }
}

pub struct PlaylistData {
    id: SpotifyId,
    title: String,
    session: Session,

    entries: Vec<SpotifyId>,
    entries_data: Arc<RwLock<Vec<PlaylistEntry>>>,

    data_fetched: RwLock<bool>,
    data_fetching: RwLock<bool>
}

impl PlaylistData {
    pub fn fetch_data(&self, cache: Arc<Mutex<DataCacheHandler>>) {
        if let (Ok(mut fetching), Ok(fetched)) = (self.data_fetching.write(), self.data_fetched.read()) {
            if *fetched || *fetching {
                return;
            }
            else {
                *fetching = true;
            }
        }

        let rt = Runtime::new().unwrap();

        for id in self.entries.iter() {
            if let Ok(mut lock) = cache.lock() {
                let track_data = {
                    if let Some(cached_track) = lock.try_get_track(&id.to_base62()) {
                        cached_track
                    }
                    else {
                        lock.add_track_unit(rt.block_on(Track::get(&self.session, *id)).unwrap())
                    }
                };

                let artist_data = {
                    if let Some(cached_artist) = lock.try_get_artist(&track_data.artists()[0]) {
                        cached_artist
                    }
                    else {
                        let id = SpotifyId::from_base62(&track_data.artists()[0]).unwrap();
                        lock.add_artist_unit(rt.block_on(Artist::get(&self.session, id)).unwrap())
                    }
                };

                if let Ok(mut lock) = self.entries_data.write() {
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
    artist: ArtistCacheUnit
}

impl PlaylistEntry {
    pub fn id(&self) -> &String {
        self.track.id()
    }

    pub fn title(&self) -> &String {
        self.track.name()
    }

    pub fn artist(&self) -> &String {
        self.artist.name()
    }

    pub fn duration(&self) -> &i32 {
        self.track.duration()
    }
}
