use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use librespot::metadata::{Album, Artist, Track};

#[derive(Default, Deserialize, Serialize)]
pub struct DataCacheHandler {
    album_cache: HashMap<String, AlbumCacheUnit>,
    artist_cache: HashMap<String, ArtistCacheUnit>,
    track_cache: HashMap<String, TrackCacheUnit>
}

impl DataCacheHandler {
    pub fn init() -> DataCacheHandler {
        let cache_path = format!("{}/imguify/data/cache.ron", dirs::cache_dir().unwrap().to_str().unwrap());

        if let Ok(deserialized) = serde_any::from_file(cache_path) {
            deserialized
        }
        else {
            DataCacheHandler::default()
        }
    }

    pub fn try_get_album(&self, id: String) -> Option<AlbumCacheUnit> {
        if let Some(album) = self.album_cache.get(&id) {
            Some(album.clone())
        }
        else {
            None
        }
    }

    pub fn try_get_artist(&self, id: String) -> Option<ArtistCacheUnit> {
        if let Some(artist) = self.artist_cache.get(&id).clone() {
            Some(artist.clone())
        }
        else {
            None
        }
    }

    pub fn try_get_track(&self, id: String) -> Option<TrackCacheUnit> {
        if let Some(track) = self.track_cache.get(&id).clone() {
            Some(track.clone())
        }
        else {
            None
        }
    }

    pub fn add_album_unit(&mut self, album: Album) -> AlbumCacheUnit {
        let id = album.id.to_base62();
        let unit = AlbumCacheUnit::from_spotify_album(album);

        self.album_cache.insert(id, unit.clone());
        self.write_cache_data();
        
        unit
    }

    pub fn add_artist_unit(&mut self, artist: Artist) -> ArtistCacheUnit {
        let id = artist.id.to_base62();
        let unit = ArtistCacheUnit::from_spotify_artist(artist);

        self.artist_cache.insert(id, unit.clone());
        self.write_cache_data();
        
        unit
    }

    pub fn add_track_unit(&mut self, track: Track) -> TrackCacheUnit {
        let id = track.id.to_base62();
        let unit = TrackCacheUnit::from_spotify_track(track);

        self.track_cache.insert(id, unit.clone());
        self.write_cache_data();

        unit
    }

    fn write_cache_data(&self) {
        let cache_path = format!("{}/imguify/data", dirs::cache_dir().unwrap().to_str().unwrap());
        let cache_file = format!("{}/cache.ron", cache_path);

        std::fs::create_dir_all(cache_path).expect("Failed to create cache dir");
        serde_any::to_file_pretty(cache_file, self).expect("Failed to write cache data");
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct AlbumCacheUnit {
    id: String,
    name: String,
    artists: Vec<String>,
    tracks: Vec<String>,
    //covers: Vec<FileId>
}

impl AlbumCacheUnit {
    pub fn from_spotify_album(album: Album) -> AlbumCacheUnit {
        AlbumCacheUnit {
            id: album.id.to_base62(),
            name: album.name,
            artists: album.artists.iter().map(|a| a.to_base62()).collect(),
            tracks: album.tracks.iter().map(|t| t.to_base62()).collect()
        }
    }

    /// Get a reference to the album cache unit's id.
    pub fn id(&self) -> &String {
        &self.id
    }

    /// Get a reference to the album cache unit's name.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Get a reference to the album cache unit's artists.
    pub fn artists(&self) -> &Vec<String> {
        &self.artists
    }

    /// Get a reference to the album cache unit's tracks.
    pub fn tracks(&self) -> &Vec<String> {
        &self.tracks
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct ArtistCacheUnit {
    id: String,
    name: String
}

impl ArtistCacheUnit {
    pub fn from_spotify_artist(artist: Artist) -> ArtistCacheUnit {
        ArtistCacheUnit {
            id: artist.id.to_base62(),
            name: artist.name
        }
    }

    /// Get a reference to the artist cache unit's id.
    pub fn id(&self) -> &String {
        &self.id
    }

    /// Get a reference to the artist cache unit's name.
    pub fn name(&self) -> &String {
        &self.name
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct TrackCacheUnit {
    id: String,
    name: String,
    duration: i32,
    album: String,
    artists: Vec<String>
}

impl TrackCacheUnit {
    pub fn from_spotify_track(track: Track) -> TrackCacheUnit {
        TrackCacheUnit {
            id: track.id.to_base62(),
            name: track.name,
            duration: track.duration,
            album: track.album.to_base62(),
            artists: track.artists.iter().map(|a| a.to_base62()).collect()
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
    pub fn duration(&self) -> &i32 {
        &self.duration
    }

    /// Get a reference to the track cache unit's album.
    pub fn album(&self) -> &String {
        &self.album
    }

    /// Get a reference to the track cache unit's artists.
    pub fn artists(&self) -> &Vec<String> {
        &self.artists
    }
}
