mod windows;

use crate::spotify::cache::TrackCacheUnit;
use crate::spotify::player::PlayerCommand;
use crate::spotify::{SpotifyHandler, PlaylistData};

use windows::login_window::LoginWindowState;
use windows::player_window::PlayerWindowState;

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

pub struct AppState {
    search_query: String,
    search_results_tracks: Vec<FullTrack>,
    search_results_artists: Vec<FullArtist>,
    search_artist_page_tracks: Vec<TrackCacheUnit>,

    show_artist_window: bool,
    show_search_window: bool,
    show_playlist_window: bool,

    login_state: LoginWindowState,
    player_state: PlayerWindowState,

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
            show_search_window: false,
            show_playlist_window: false,

            login_state: LoginWindowState::init(),
            player_state: PlayerWindowState::init(),

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

        let id = imgui.fonts().add_font(&[
            FontSource::DefaultFontData {
                config: Some(
                    FontConfig {
                        size_pixels: 13.0,
                        ..FontConfig::default()
                    }
                )
            },
            FontSource::TtfData {
                data: include_bytes!("../../NotoSansCJKsc-Regular.otf"),
                size_pixels: 13.0,
                config: Some(
                    FontConfig {
                        rasterizer_multiply: 1.75,
                        size_pixels: 13.0,
                        glyph_ranges: FontGlyphRanges::chinese_simplified_common(),
                        ..FontConfig::default()
                    }
                )
            },
            FontSource::TtfData {
                data: include_bytes!("../../NotoSansCJKjp-Regular.otf"),
                size_pixels: 13.0,
                config: Some(
                    FontConfig {
                        rasterizer_multiply: 1.75,
                        size_pixels: 13.0,
                        glyph_ranges: FontGlyphRanges::japanese(),
                        ..FontConfig::default()
                    }
                )
            },
            FontSource::TtfData {
                data: include_bytes!("../../NotoSansCJKkr-Regular.otf"),
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
        ]);

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
                    windows::login_window::build(&ui, &mut app_state);
                }
                else {
                    windows::main_window::build(&ui, &mut app_state);

                    if app_state.show_artist_window {
                        windows::artist_window::build(&ui, &mut app_state);
                    }

                    if app_state.show_search_window {
                        windows::search_window::build(&ui, &mut app_state);
                    }

                    if app_state.player_state.show {
                        windows::player_window::build(&ui, &mut app_state);
                    }

                    if app_state.show_playlist_window {
                        windows::playlist_window::build(&ui, &mut app_state);
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