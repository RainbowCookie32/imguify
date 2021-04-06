mod windows;

use crate::spotify::player::PlayerCommand;
use crate::spotify::{SpotifyHandler, PlaylistData};

use std::sync::Arc;
use std::sync::mpsc::Sender;

use imgui::*;

use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};

use glium::glutin;
use glium::{Display, Surface};
use glium::glutin::event::ElementState;
use glium::glutin::window::WindowBuilder;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};

const MEDIA_SKIP: u32 = 163;
const MEDIA_PAUSE: u32 = 164;
const MEDIA_PREVIOUS: u32 = 165;

pub struct AppState {
    username: ImString,
    password: ImString,
    login_failed: bool,

    playlist_data: Option<Arc<PlaylistData>>,
    spotify_handler: Option<SpotifyHandler>,
    player_tx: Option<Sender<PlayerCommand>>
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            username: ImString::new(""),
            password: ImString::new(""),
            login_failed: false,

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

        event_loop.run(move |event, _, control_flow| match event {
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();

                platform.prepare_frame(imgui.io_mut(), &gl_window.window()).unwrap();
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let ui = imgui.frame();

                if app_state.spotify_handler.is_none() {
                    windows::login_window::build(&ui, &mut app_state);
                }
                else {
                    windows::main_window::build(&ui, &mut app_state);

                    if app_state.playlist_data.is_some() {
                        windows::playlist_window::build(&ui, &mut app_state);
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
                        MEDIA_SKIP => {
                            if let Some(tx) = app_state.player_tx.as_ref() {
                                tx.send(PlayerCommand::SkipTrack).unwrap();
                            }
                        }
                        MEDIA_PAUSE => {
                            if let Some(tx) = app_state.player_tx.as_ref() {
                                tx.send(PlayerCommand::PlayPause).unwrap();
                            }
                        }
                        MEDIA_PREVIOUS => {
                            if let Some(tx) = app_state.player_tx.as_ref() {
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
}