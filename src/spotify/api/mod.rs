pub mod cache;

use std::env;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

use rspotify::model::page::Page;
use rspotify::model::track::FullTrack;
use rspotify::model::artist::FullArtist;
use rspotify::model::search::SearchResult;
use rspotify::model::playlist::SimplifiedPlaylist;
use rspotify::model::album::{FullAlbum, SimplifiedAlbum};

use rspotify::senum::SearchType;
use rspotify::blocking::client::Spotify;
use rspotify::blocking::oauth2::{SpotifyClientCredentials, SpotifyOAuth};

use cache::*;

pub struct SpotifyAPIHandler {
    api_client: Spotify,
    cache_handler: Arc<Mutex<APICacheHandler>>
}

impl SpotifyAPIHandler {
    pub fn init(cache_handler: Arc<Mutex<APICacheHandler>>) -> Result<SpotifyAPIHandler> {
        let client_id = env::var("CLIENT_ID")?;
        let client_secret = env::var("CLIENT_SECRET")?;

        let mut oauth = SpotifyOAuth::default()
            .scope("playlist-read-private")
            .redirect_uri("http://localhost:8888/callback")
            .client_id(&client_id)
            .client_secret(&client_secret)
            .build()
        ;

        let token = rspotify::blocking::util::get_token(&mut oauth).context("Failed to get API token")?;
        let api_credentials = SpotifyClientCredentials::default().token_info(token);
        let api_client = Spotify::default().client_credentials_manager(api_credentials).build();

        let handler = SpotifyAPIHandler {
            api_client,
            cache_handler
        };

        Ok(handler)
    }

    pub fn get_user_playlists(&self) -> Option<Page<SimplifiedPlaylist>> {
        self.api_client.current_user_playlists(10, 0).ok()
    }

    pub fn remove_track_from_playlist(&self, playlist_id: &str, track_id: &str) -> bool {
        let user_id = self.api_client.me().unwrap().id;
        self.api_client.user_playlist_remove_all_occurrences_of_tracks(&user_id, playlist_id, &[track_id.to_string()], None).is_ok()
    }

    pub fn get_track(&self, track_id: String) -> Result<TrackCacheUnit> {
        if let Ok(mut lock) = self.cache_handler.lock() {
            if let Some(unit) = lock.try_get_track(&track_id) {
                Ok(unit)
            }
            else {
                let track_data = self.api_lookup_track(track_id).context("Couldn't find track on API")?;
                lock.add_track_unit(track_data).context("Failed to add track to cache")
            }
        }
        else {
            Err(anyhow::Error::msg("Couldn't lock API cache handler"))
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
                self.api_lookup_album(album_id).map(|album_data| lock.add_album_unit(album_data))
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
        if let Ok(SearchResult::Tracks(data)) = self.api_client.search(&query, SearchType::Track, 10, 0, None, None) {
            Some(data)
        }
        else {
            None
        }
    }

    pub fn search_artists(&self, query: String) -> Option<Page<FullArtist>> {
        if let Ok(SearchResult::Artists(data)) = self.api_client.search(&query, SearchType::Artist, 10, 0, None, None) {
            Some(data)
        }
        else {
            None
        }
    }
}