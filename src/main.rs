mod spotify;

use std::sync::Arc;
use std::sync::mpsc::Sender;

use spotify::*;

use imgui::*;

use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};

use glium::glutin::{self, event::ElementState};
use glium::glutin::window::WindowBuilder;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};

use glium::{Display, Surface};

fn main() {
    let system = App::new();

    dotenv::from_filename("tokens.env").expect("Failed to load tokens.env");
    system.render_loop();
}

struct AppState {
    username: ImString,
    password: ImString,

    playlist_data: Option<Arc<PlaylistData>>,
    spotify_handler: Option<SpotifyHandler>,
    player_sender: Option<Sender<PlayerCommand>>
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            username: ImString::new(""),
            password: ImString::new(""),

            playlist_data: None,
            spotify_handler: None,
            player_sender: None
        }
    }
}

struct App {
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

        event_loop.run(move |event, _, control_flow| match event {
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();

                platform.prepare_frame(imgui.io_mut(), &gl_window.window()).unwrap();
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let ui = imgui.frame();

                if app_state.spotify_handler.is_none() {
                    App::login_window(&ui, &mut app_state);
                }
                else {
                    App::main_window(&ui, &mut app_state);

                    if app_state.playlist_data.is_some() {
                        App::playlist_view_window(&ui, &mut app_state);
                    }
                }

                let gl_window = display.gl_window();
                let mut target = display.draw();

                target.clear_color_srgb(0.2, 0.2, 0.2, 1.0);
                platform.prepare_render(&ui, gl_window.window());

                let draw_data = ui.render();

                renderer.render(&mut target, draw_data).unwrap();
                target.finish().unwrap();
            }
            Event::DeviceEvent { event: glium::glutin::event::DeviceEvent::Key(input), ..} => {
                if input.state == ElementState::Pressed {
                    match input.scancode {
                        163 => {
                            if let Some(tx) = app_state.player_sender.as_ref() {
                                tx.send(PlayerCommand::SkipTrack).unwrap();
                            }
                        }
                        164 => {
                            if let Some(tx) = app_state.player_sender.as_ref() {
                                tx.send(PlayerCommand::PlayPause).unwrap();
                            }
                        }
                        165 => {
                            if let Some(tx) = app_state.player_sender.as_ref() {
                                tx.send(PlayerCommand::PrevTrack).unwrap();
                            }
                        }
                        _ => {}
                    }
                }

                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
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

    fn login_window(ui: &Ui, app_state: &mut AppState) {
        Window::new(im_str!("Login to Spotify")).size([350.0, 110.0], Condition::Always).build(&ui, || {
            let submitted = {
                ui.input_text(im_str!("Username"), &mut app_state.username)
                    .resize_buffer(true)
                    .enter_returns_true(true)
                    .build()
                ||
                ui.input_text(im_str!("Password"), &mut app_state.password)
                    .password(true)
                    .resize_buffer(true)
                    .enter_returns_true(true)
                    .build()
                ||
                ui.button(im_str!("Login"), [45.0, 20.0])
            };

            if submitted {
                let (tx, rx) = std::sync::mpsc::channel();
                let handler = spotify::SpotifyHandler::init(app_state.username.to_string(), app_state.password.to_string(), rx);

                app_state.password.clear();
                app_state.player_sender = Some(tx);
                app_state.spotify_handler = Some(handler);
            }
        });
    }

    fn main_window(ui: &Ui, app_state: &mut AppState) {
        if let Some(handler) = app_state.spotify_handler.as_mut() {
            if handler.get_playlists_names().len() == 0 {
                handler.fetch_user_playlists();
            }
        }
        
        Window::new(im_str!("Main Window")).size([800.0, 500.0], Condition::FirstUseEver).build(&ui, || {
            if let Some(handler) = app_state.spotify_handler.as_mut() {
                let plists = handler.get_playlists_names();

                ui.text("Playlists");
                ui.separator();

                for idx in 0..plists.len() {
                    let plist = &plists[idx];

                    ui.text(plist);
                    ui.same_line(200.0);

                    if ui.button(im_str!("Play"), [40.0, 20.0]) {
                        if let Some(tx) = app_state.player_sender.as_ref() {
                            // A bit of a gamble, but should be fine.
                            let plist = handler.get_playlist(idx).unwrap();
                            let cache_handler = handler.get_cache_handler();
                            
                            tx.send(PlayerCommand::StartPlaylist(plist.clone())).unwrap();

                            std::thread::spawn(move || {
                                plist.fetch_data(cache_handler);
                            });
                        }
                    }
                
                    ui.same_line(250.0);

                    if ui.button(im_str!("View"), [40.0, 20.0]) {
                        let playlist = handler.get_playlist(idx);
                        let playlist_fetch = playlist.clone();

                        app_state.playlist_data = playlist;
                        
                        if let Some(playlist_fetch) = playlist_fetch {
                            let cache_handler = handler.get_cache_handler();

                            std::thread::spawn(move || {
                                playlist_fetch.fetch_data(cache_handler);
                            });
                        }
                    }
                    
                    ui.separator();
                }

                if handler.get_playback_status() {
                    if let Some(track) = handler.get_current_song() {
                        ui.text(format!("{} - {}", track.artist(), track.title()));
                    }
                    else {
                        ui.text("Waiting for player/data...");
                    }
                }
                else {
                    ui.text("Player's empty");
                }
            }
        });
    }

    fn playlist_view_window(ui: &Ui, app_state: &mut AppState) {
        Window::new(im_str!("Playlist")).size([800.0, 500.0], Condition::FirstUseEver).build(&ui, || {
            ui.text("Songs");

            if let Some(plist) = app_state.playlist_data.as_ref() {
                if let Ok(lock) = plist.entries_data().lock() {
                    let data_len = lock.len();
                    let entries_len = plist.entries().len();

                    if data_len != entries_len {
                        ui.same_line(70.0);
                        ui.text_colored([1.0, 0.1, 0.1, 1.0], format!("Loading songs: {}/{}", data_len, entries_len));
                    }

                    ui.separator();

                    ui.columns(3, im_str!("Columns?"), true);

                    for entry in lock.iter() {
                        ui.text(entry.title());
                        ui.next_column();

                        ui.text(entry.artist());
                        ui.next_column();

                        let seconds = entry.duration() / 1000;
                        let minutes = seconds / 60;
                        let seconds = seconds % 60;

                        ui.text(format!("{}:{:02}", minutes, seconds));
                        ui.next_column();
                    }
                }
            }
        });
    }
}
