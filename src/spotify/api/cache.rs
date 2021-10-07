use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use rspotify::model::album::FullAlbum;
use rspotify::model::track::FullTrack;


#[derive(Default, Deserialize, Serialize)]
pub struct APICacheHandler {
    album_cache: HashMap<String, AlbumInfo>,
    track_cache: HashMap<String, TrackInfo>
}

impl APICacheHandler {
    pub fn init() -> APICacheHandler {
        let mut cache_path = dirs::cache_dir().expect("Couldn't get cache dir");
        cache_path.push("imguify/data/cache.ron");

        if let Ok(deserialized) = serde_any::from_file(cache_path) {
            deserialized
        }
        else {
            APICacheHandler::default()
        }
    }

    pub fn try_get_album(&self, id: &str) -> Option<AlbumInfo> {
        self.album_cache.get(id).cloned()
    }

    pub fn add_album_unit(&mut self, album: FullAlbum) -> AlbumInfo {
        let id = album.id.clone();
        let unit = AlbumInfo::from_api_data(album);

        self.album_cache.insert(id, unit.clone());
        self.write_cache_data();

        unit
    }

    pub fn try_get_track(&self, id: &str) -> Option<TrackInfo> {
        self.track_cache.get(id).cloned()
    }

    pub fn add_track_unit(&mut self, track: FullTrack) -> Option<TrackInfo> {
        let id = track.id.clone().unwrap_or_else(String::new);
        let unit = TrackInfo::from_api_data(track);

        if let Some(unit) = unit.as_ref() {
            self.track_cache.insert(id, unit.clone());
            self.write_cache_data();
        }

        unit
    }

    fn write_cache_data(&self) {
        let mut cache_path = dirs::cache_dir().expect("Couldn't get cache dir");
        cache_path.push("imguify/data/cache.ron");

        if let Err(error) = std::fs::create_dir_all(&cache_path) {
            match error.kind() {
                std::io::ErrorKind::AlreadyExists => {},
                _ => panic!("{}", error.to_string())
            }
        }
        
        serde_any::to_file_pretty(cache_path, self).expect("Failed to write cache data");
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct AlbumInfo {
    id: String,
    name: String,
    
    tracks: Vec<String>,
    artists: Vec<String>
}

impl AlbumInfo {
    pub fn from_api_data(album: FullAlbum) -> AlbumInfo {
        AlbumInfo {
            id: album.id,
            name: album.name,
            
            tracks: album.tracks.items.into_iter().map(|t| t.id.unwrap_or_else(String::new)).collect(),
            artists: album.artists.into_iter().map(|a| a.name).collect()
        }
    }

    pub fn tracks(&self) -> &Vec<String> {
        &self.tracks
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct TrackInfo {
    id: String,
    name: String,
    duration: u32,
    popularity: u32,
    album: String,
    artists: Vec<String>
}

impl TrackInfo {
    pub fn from_api_data(track: FullTrack) -> Option<TrackInfo> {
        if let (Some(id), Some(album)) = (track.id, track.album.id) {
            Some(
                TrackInfo {
                    id,
                    name: track.name,
                    duration: track.duration_ms,
                    popularity: track.popularity,
                    album,
                    artists: track.artists.into_iter().map(|a| a.name).collect()
                }
            )
        }
        else {
            None
        }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn duration(&self) -> &u32 {
        &self.duration
    }

    pub fn artists(&self) -> &Vec<String> {
        &self.artists
    }

    pub fn popularity(&self) -> &u32 {
        &self.popularity
    }
}
