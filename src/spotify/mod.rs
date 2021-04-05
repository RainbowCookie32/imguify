mod cache;

use cache::DataCacheHandler;

use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};

use rand::prelude::*;

use futures::FutureExt;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::UnboundedReceiver;

use librespot::core::cache::Cache;
use librespot::core::session::Session;
use librespot::core::config::SessionConfig;
use librespot::core::spotify_id::SpotifyId;
use librespot::core::authentication::Credentials;

use librespot::playback::audio_backend;
use librespot::playback::player::{Player, PlayerEvent};
use librespot::playback::config::{Bitrate, PlayerConfig, NormalisationType};

use librespot::metadata::{Metadata, Artist, Playlist, Track};

use rspotify::blocking::client::Spotify;
use rspotify::blocking::oauth2::{SpotifyClientCredentials, SpotifyOAuth};

use cache::{ArtistCacheUnit, TrackCacheUnit};

pub enum PlayerCommand {
    PlayPause,
    PrevTrack,
    SkipTrack,

    StartPlaylist(Arc<PlaylistData>)
}

pub struct PlayerHandler {
    player: Player,
    
    loaded: bool,
    playing: bool,
    queued_playlist: Option<Arc<PlaylistData>>,

    song_in_player_id: Option<SpotifyId>,
    song_in_player_idx: usize,

    cmd_rx: Receiver<PlayerCommand>,
    player_events: UnboundedReceiver<PlayerEvent>
}

impl PlayerHandler {
    pub fn init(session: Session, cmd_rx: Receiver<PlayerCommand>) -> Arc<Mutex<PlayerHandler>> {
        let mut player_cfg = PlayerConfig::default();

        player_cfg.bitrate = Bitrate::Bitrate320;
        
        player_cfg.normalisation = true;
        player_cfg.normalisation_type = NormalisationType::Track;

        let backend = audio_backend::find(None).unwrap();
        let (player, _) = Player::new(player_cfg, session, None, move || {
            backend(None)
        });

        let player_events = player.get_player_event_channel();

        let handler = PlayerHandler {
            player,

            loaded: false,
            playing: false,
            queued_playlist: None,

            song_in_player_id: None,
            song_in_player_idx: 0,

            cmd_rx,
            player_events
        };

        let handler = Arc::from(Mutex::from(handler));
        let player_handler = handler.clone();

        std::thread::spawn(move || {
            let player = player_handler;

            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));

                if let Ok(mut lock) = player.lock() {
                    if let Some(event) = lock.player_events.recv().now_or_never() {
                        if let Some(event) = event {
                            lock.handle_player_event(event);
                        }
                    }

                    if let Ok(command) = lock.cmd_rx.try_recv() {
                        lock.handle_player_command(command);
                    }
                }
            }
        });

        handler
    }

    pub fn handle_player_event(&mut self, event: PlayerEvent) {
        match event {
            PlayerEvent::Stopped { .. } => {
                self.loaded = false;
                self.playing = false;
            }
            PlayerEvent::Started { .. } => {
                self.loaded = true;
                self.playing = true;
            }
            PlayerEvent::EndOfTrack { .. } => {
                if let Some(playlist) = self.queued_playlist.as_ref() {
                    self.song_in_player_idx += 1;

                    if self.song_in_player_idx >= playlist.entries_shuffled.len() {
                        self.song_in_player_idx = 0;
                    }

                    self.song_in_player_id = Some(playlist.entries_shuffled[self.song_in_player_idx]);

                    self.player.load(playlist.entries_shuffled[self.song_in_player_idx], true, 0);
                    self.player.play();
                }
            }
            _ => {}
        }
    }

    pub fn handle_player_command(&mut self, command: PlayerCommand) {
        match command {
            PlayerCommand::PlayPause => {
                if self.playing {
                    self.player.pause();
                    self.playing = false;
                }
                else {
                    self.player.play();
                    self.playing = true;
                }
            }
            PlayerCommand::PrevTrack => {
                if let Some(playlist) = self.queued_playlist.as_ref() {
                    if self.song_in_player_idx == 0 {
                        self.song_in_player_idx = playlist.entries_shuffled.len() - 1;
                    }
                    else {
                        self.song_in_player_idx -= 1;
                    }

                    self.song_in_player_id = Some(playlist.entries_shuffled[self.song_in_player_idx]);

                    self.player.load(playlist.entries_shuffled[self.song_in_player_idx], true, 0);
                    self.player.play();
                }
            }
            PlayerCommand::SkipTrack => {
                if let Some(playlist) = self.queued_playlist.as_ref() {
                    self.song_in_player_idx += 1;

                    if self.song_in_player_idx >= playlist.entries_shuffled.len() {
                        self.song_in_player_idx = 0;
                    }

                    self.song_in_player_id = Some(playlist.entries_shuffled[self.song_in_player_idx]);

                    self.player.load(playlist.entries_shuffled[self.song_in_player_idx], true, 0);
                    self.player.play();
                }
            }
            PlayerCommand::StartPlaylist(p) => {
                self.loaded = true;
                self.song_in_player_idx = 0;

                self.song_in_player_id = Some(p.entries_shuffled[self.song_in_player_idx]);

                self.player.load(p.entries_shuffled[self.song_in_player_idx], true, 0);
                self.player.play();

                self.queued_playlist = Some(p);
            }
        }
    }

    pub fn get_current_song(&self) -> Option<PlaylistEntry> {
        if let (Some(plist), Some(sid)) = (self.queued_playlist.as_ref(), self.song_in_player_id.as_ref()) {
            if let Ok(lock) = plist.entries_data.lock() {
                for song in lock.iter() {
                    if song.id() == &sid.to_base62() {
                        return Some(song.clone());
                    }
                }
            }
        }

        None
    }
}

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
            lock.loaded
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
            let mut rng = thread_rng();

            self.playlist_data.clear();

            for item in playlists.items {
                let id = SpotifyId::from_base62(&item.id).expect("Failed to parse id");

                if let Ok(list) = self.rt.block_on(Playlist::get(&self.spotify_session, id.clone())) {
                    let entries = list.tracks;
                    let mut entries_shuffled = entries.clone();

                    entries_shuffled.shuffle(&mut rng);

                    self.playlist_data.push(Arc::new(
                        PlaylistData {
                            id,
                            title: list.name,
                            session: self.spotify_session.clone(),

                            entries,
                            entries_shuffled,
                            entries_data: Arc::new(Mutex::new(Vec::new())),

                            data_fetched: RwLock::new(false),
                            data_fetching: RwLock::new(false)
                        }
                    ));
                }
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
    entries_shuffled: Vec<SpotifyId>,
    entries_data: Arc<Mutex<Vec<PlaylistEntry>>>,

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

                if let Ok(mut lock) = self.entries_data.lock() {
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
    pub fn entries_data(&self) -> &Arc<Mutex<Vec<PlaylistEntry>>> {
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
