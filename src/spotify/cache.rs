use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use rspotify::model::album::FullAlbum;
use rspotify::model::track::FullTrack;


#[derive(Default, Deserialize, Serialize)]
pub struct APICacheHandler {
    album_cache: HashMap<String, AlbumCacheUnit>,
    track_cache: HashMap<String, TrackCacheUnit>
}

impl APICacheHandler {
    pub fn init() -> APICacheHandler {
        let mut cache_path = dirs::cache_dir().expect("Couldn't get cache dir");
        cache_path.push("/imguify/data/cache.ron");

        if let Ok(deserialized) = serde_any::from_file(cache_path) {
            deserialized
        }
        else {
            APICacheHandler::default()
        }
    }

    pub fn try_get_album(&self, id: &String) -> Option<AlbumCacheUnit> {
        if let Some(album) = self.album_cache.get(id).clone() {
            Some(album.clone())
        }
        else {
            None
        }
    }

    pub fn add_album_unit(&mut self, album: FullAlbum) -> AlbumCacheUnit {
        let id = album.id.clone();
        let unit = AlbumCacheUnit::from_api_data(album);

        self.album_cache.insert(id, unit.clone());
        self.write_cache_data();

        unit
    }

    pub fn try_get_track(&self, id: &String) -> Option<TrackCacheUnit> {
        if let Some(track) = self.track_cache.get(id).clone() {
            Some(track.clone())
        }
        else {
            None
        }
    }

    pub fn add_track_unit(&mut self, track: FullTrack) -> Option<TrackCacheUnit> {
        let id = track.id.clone().unwrap_or_else(|| String::new());
        let unit = TrackCacheUnit::from_api_data(track);

        if let Some(unit) = unit.as_ref() {
            self.track_cache.insert(id, unit.clone());
            self.write_cache_data();
        }

        unit
    }

    fn write_cache_data(&self) {
        let mut cache_path = dirs::cache_dir().expect("Couldn't get cache dir");
        cache_path.push("/imguify/data/cache.ron");

        std::fs::create_dir_all(&cache_path).expect("Failed to create cache dir");
        serde_any::to_file_pretty(cache_path, self).expect("Failed to write cache data");
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct AlbumCacheUnit {
    id: String,
    name: String,
    
    tracks: Vec<String>,
    artists: Vec<String>
}

impl AlbumCacheUnit {
    pub fn from_api_data(album: FullAlbum) -> AlbumCacheUnit {
        AlbumCacheUnit {
            id: album.id,
            name: album.name,
            
            tracks: album.tracks.items.into_iter().map(|t| t.id.unwrap_or_else(|| String::new())).collect(),
            artists: album.artists.into_iter().map(|a| a.name).collect()
        }
    }

    /// Get a reference to the album cache unit's tracks.
    pub fn tracks(&self) -> &Vec<String> {
        &self.tracks
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct TrackCacheUnit {
    id: String,
    name: String,
    duration: u32,
    popularity: u32,
    album: String,
    artists: Vec<String>
}

impl TrackCacheUnit {
    pub fn from_api_data(track: FullTrack) -> Option<TrackCacheUnit> {
        if let (Some(id), Some(album)) = (track.id, track.album.id) {
            Some(
                TrackCacheUnit {
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

    /// Get a reference to the track cache unit's id.
    pub fn id(&self) -> &String {
        &self.id
    }

    /// Get a reference to the track cache unit's name.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Get a reference to the track cache unit's duration.
    pub fn duration(&self) -> &u32 {
        &self.duration
    }

    /// Get a reference to the track cache unit's artists.
    pub fn artists(&self) -> &Vec<String> {
        &self.artists
    }

    /// Get a reference to the track cache unit's popularity.
    pub fn popularity(&self) -> &u32 {
        &self.popularity
    }
}
