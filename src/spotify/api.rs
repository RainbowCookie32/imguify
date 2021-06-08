use std::sync::{Arc, Mutex};

use rspotify::model::page::Page;
use rspotify::model::track::FullTrack;
use rspotify::model::artist::FullArtist;
use rspotify::model::search::SearchResult;
use rspotify::model::playlist::SimplifiedPlaylist;
use rspotify::model::album::{FullAlbum, SimplifiedAlbum};

use rspotify::senum::SearchType;
use rspotify::blocking::client::Spotify;
use rspotify::blocking::oauth2::{SpotifyClientCredentials, SpotifyOAuth};

use super::cache::*;

pub struct SpotifyAPIHandler {
    api_client: Spotify,
    cache_handler: Arc<Mutex<APICacheHandler>>
}

impl SpotifyAPIHandler {
    pub fn init(cache_handler: Arc<Mutex<APICacheHandler>>) -> Option<SpotifyAPIHandler> {
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

            let res = SpotifyAPIHandler {
                api_client,
                cache_handler
            };

            Some(res)
        }
        else {
            None
        }
    }

    pub fn get_user_playlists(&self) -> Option<Page<SimplifiedPlaylist>> {
        self.api_client.current_user_playlists(10, 0).ok()
    }

    pub fn remove_track_from_playlist(&self, playlist_id: &String, track_id: &String) -> bool {
        let user_id = self.api_client.me().unwrap().id;
        self.api_client.user_playlist_remove_all_occurrences_of_tracks(&user_id, playlist_id, &[track_id.clone()], None).is_ok()
    }

    pub fn get_track(&self, track_id: String) -> Option<TrackCacheUnit> {
        if let Ok(mut lock) = self.cache_handler.lock() {
            let cache_result = lock.try_get_track(&track_id);

            if cache_result.is_some() {
                cache_result
            }
            else {                
                if let Some(track_data) = self.api_lookup_track(track_id) {
                    Some(lock.add_track_unit(track_data))
                }
                else {
                    None
                }
            }
        }
        else {
            None
        }
    }

    fn api_lookup_track(&self, track_id: String) -> Option<FullTrack> {
        self.api_client.track(&track_id).ok()
    }

    pub fn get_album(&self, album_id: String) -> Option<AlbumCacheUnit> {
        if let Ok(mut lock) = self.cache_handler.lock() {
            let cache_result = lock.try_get_album(&album_id);

            if cache_result.is_some() {
                cache_result
            }
            else {                
                if let Some(album_data) = self.api_lookup_album(album_id) {
                    Some(lock.add_album_unit(album_data))
                }
                else {
                    None
                }
            }
        }
        else {
            None
        }
    }

    fn api_lookup_album(&self, album_id: String) -> Option<FullAlbum> {
        self.api_client.album(&album_id).ok()
    }

    pub fn get_all_albums_for_artist(&self, artist_id: String) -> Option<Page<SimplifiedAlbum>> {
        self.api_client.artist_albums(&artist_id, None, None, Some(10), None).ok()
    }

    pub fn search_tracks(&self, query: String) -> Option<Page<FullTrack>> {
        if let Ok(results) = self.api_client.search(&query, SearchType::Track, 10, 0, None, None) {
            match results {
                SearchResult::Tracks(data) => {
                    Some(data)
                }
                _ => {
                    None
                }
            }
        }
        else {
            None
        }
    }

    pub fn search_artists(&self, query: String) -> Option<Page<FullArtist>> {
        if let Ok(results) = self.api_client.search(&query, SearchType::Artist, 10, 0, None, None) {
            match results {
                SearchResult::Artists(data) => {
                    Some(data)
                }
                _ => {
                    None
                }
            }
        }
        else {
            None
        }
    }
}