mod windows;

use std::sync::Arc;
use std::sync::mpsc::Sender;

use imgui::*;

use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};

use glium::glutin;
use glium::{Display, Surface};
use glium::glutin::window::WindowBuilder;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};

use rspotify::model::track::FullTrack;
use rspotify::model::artist::FullArtist;

use windows::artist_window::ArtistWindow;
use windows::login_window::LoginWindow;
use windows::main_window::MainWindow;
use windows::player_window::PlayerWindow;
use windows::playlist_window::PlaylistWindow;

use crate::spotify::player::PlayerCommand;
use crate::spotify::api::cache::TrackCacheUnit;
use crate::spotify::{SpotifyHandler, PlaylistData};

pub struct AppState {
    search_query: String,
    search_results_tracks: Vec<FullTrack>,
    search_results_artists: Vec<FullArtist>,
    search_artist_page_tracks: Vec<TrackCacheUnit>,

    show_artist_window: bool,
    show_player_window: bool,
    show_search_window: bool,
    show_playlist_window: bool,

    playlist_data: Option<Arc<PlaylistData>>,
    spotify_handler: Option<SpotifyHandler>,
    player_tx: Option<Sender<PlayerCommand>>
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            search_query: String::new(),
            search_results_tracks: Vec::new(),
            search_results_artists: Vec::new(),
            search_artist_page_tracks: Vec::new(),

            show_artist_window: false,
            show_player_window: false,
            show_search_window: false,
            show_playlist_window: false,

            playlist_data: None,
            spotify_handler: None,
            player_tx: None
        }
    }
}

pub struct App {
    pub event_loop: EventLoop<()>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
}

impl App {
    pub fn new() -> App {
        let event_loop = EventLoop::new();
        let context = glutin::ContextBuilder::new().with_vsync(true);
        let builder = WindowBuilder::new()
            .with_title("imguify")
            .with_inner_size(glutin::dpi::LogicalSize::new(800, 600));
        let display =
            Display::new(builder, context, &event_loop).expect("Failed to initialize display");

        let mut imgui = Context::create();
        let mut platform = WinitPlatform::init(&mut imgui);

        {
            let gl_window = display.gl_window();
            let window = gl_window.window();
            platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);
        }

        let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

        App {
            event_loop,
            display,
            imgui,
            platform,
            renderer,
        }
    }

    pub fn render_loop(self) {
        let App {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
        } = self;

        let mut app_state = AppState::new();

        let mut login_window = LoginWindow::init();
        
        let mut artist_window: Option<ArtistWindow> = None;
        let mut main_window: Option<MainWindow> = None;
        let mut player_window: Option<PlayerWindow> = None;
        let mut playlist_window: Option<PlaylistWindow> = None;

        let ch_font = std::fs::read("fonts/chinese.otf").unwrap_or_else(|_| Vec::new());
        let jp_font = std::fs::read("fonts/japanese.otf").unwrap_or_else(|_| Vec::new());
        let kr_font = std::fs::read("fonts/korean.otf").unwrap_or_else(|_| Vec::new());

        let mut fonts = vec![
            FontSource::DefaultFontData {
                config: Some(
                    FontConfig {
                        size_pixels: 13.0,
                        ..FontConfig::default()
                    }
                )
            }
        ];

        if !ch_font.is_empty() {
            fonts.push(
                FontSource::TtfData {
                    data: &ch_font,
                    size_pixels: 13.0,
                    config: Some(
                        FontConfig {
                            rasterizer_multiply: 1.75,
                            size_pixels: 13.0,
                            glyph_ranges: FontGlyphRanges::chinese_simplified_common(),
                            ..FontConfig::default()
                        }
                    )
                }
            );
        }

        if !jp_font.is_empty() {
            fonts.push(
                FontSource::TtfData {
                    data: &jp_font,
                    size_pixels: 13.0,
                    config: Some(
                        FontConfig {
                            rasterizer_multiply: 1.75,
                            size_pixels: 13.0,
                            glyph_ranges: FontGlyphRanges::japanese(),
                            ..FontConfig::default()
                        }
                    )
                }
            )
        }

        if !kr_font.is_empty() {
            fonts.push(
                FontSource::TtfData {
                    data: &kr_font,
                    size_pixels: 13.0,
                    config: Some(
                        FontConfig {
                            rasterizer_multiply: 1.75,
                            size_pixels: 13.0,
                            glyph_ranges: FontGlyphRanges::korean(),
                            ..FontConfig::default()
                        }
                    )
                }
            )
        }

        let id = imgui.fonts().add_font(fonts.as_slice());
        renderer.reload_font_texture(&mut imgui).unwrap();

        event_loop.run(move |event, _, control_flow| match event {
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();

                platform.prepare_frame(imgui.io_mut(), gl_window.window()).unwrap();
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let ui = imgui.frame();
                
                let token = ui.push_font(id);

                if app_state.spotify_handler.is_none() {
                    let username = login_window.draw(&ui, &mut app_state);

                    if !username.is_empty() {
                        let playlists = {
                            if let Some(handler) = app_state.spotify_handler.as_mut() {
                                handler.fetch_user_playlists();
                                handler.get_playlists_names()
                            }
                            else {
                                Vec::new()
                            }
                        };

                        main_window = Some(MainWindow::init(username, playlists));
                    }
                }
                else {
                    if let Some(window) = main_window.as_mut() {
                        window.draw(&ui, &mut app_state);
                    }

                    if app_state.show_artist_window {
                        if let Some(window) = artist_window.as_mut() {
                            window.draw(&ui, &mut app_state);
                        }
                        else {
                            artist_window = Some(ArtistWindow::init(app_state.search_query.clone(), app_state.search_artist_page_tracks.clone()));
                        }
                    }

                    if app_state.show_search_window {
                        windows::search_window::build(&ui, &mut app_state);
                    }

                    if app_state.show_player_window {
                        if let Some(window) = player_window.as_mut() {
                            window.draw(&ui, &mut app_state);
                        }
                        else {
                            player_window = Some(PlayerWindow::init());
                        }
                    }

                    if app_state.show_playlist_window {
                        if let Some(window) = playlist_window.as_mut() {
                            window.draw(&ui, &mut app_state);
                        }
                        else if let Some(playlist) = app_state.playlist_data.as_ref() {
                            playlist_window = Some(PlaylistWindow::init(playlist.clone()));
                        }
                    }
                }

                token.pop();

                let gl_window = display.gl_window();
                let mut target = display.draw();

                target.clear_color_srgb(0.2, 0.2, 0.2, 1.0);
                platform.prepare_render(&ui, gl_window.window());

                let draw_data = ui.render();

                renderer.render(&mut target, draw_data).unwrap();
                target.finish().unwrap();
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, ..} => {
                *control_flow = ControlFlow::Exit
            }
            event => {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        });
    }
}