#![allow(
    dead_code,
    incomplete_features,
    clippy::needless_arbitrary_self_type,
    clippy::diverging_sub_expression
)]
#![feature(
    int_roundings,
    anonymous_lifetime_in_impl_trait,
    generic_const_exprs,
    addr_parse_ascii,
    get_many_mut,
    float_next_up_down
)]

use std::{sync::Arc, time::Instant};
use app_state::{AppState, WinitEvent};
use shared::version::APP_VERSION;
use special::worldline::PHYS_TIME_STEP;
use winit::{application::ApplicationHandler, event::{DeviceEvent, DeviceId, WindowEvent}, event_loop::{ActiveEventLoop, EventLoop}, window::{CursorGrabMode, Window, WindowId}};
use anyhow::Result;

pub mod app_state;
pub mod graphics;
pub mod gui;
pub mod shared;
pub mod special;
pub mod general;

struct App {
    window: Option<Arc<Window>>,
    app_state: Option<AppState>,
    mouse_locked: bool,
    last_frame: Instant,
    ticks_owed: f64,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = 
            Arc::new(event_loop.create_window(
                Window::default_attributes()
                    .with_title(format!("Worldline v{}", APP_VERSION))
            ).unwrap());
        window.set_ime_allowed(true);

        let app_state = AppState::new(Arc::clone(&window)).unwrap();
        self.mouse_locked = app_state.input_controller.is_mouse_locked();
        self.app_state = Some(app_state);
        
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let (window, app_state) = match (&self.window, &mut self.app_state) {
            (Some(window), Some(app_state)) => (window, app_state),
            _ => return,
        };

        if window_id != window.id() { return; }

        app_state.winit_event(WinitEvent::Window(&event));

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                // device_id,
                // event: input_event,
                // is_synthetic,
                ..
            } => {}
            WindowEvent::RedrawRequested => {
                let frame_time = self.last_frame.elapsed();
                self.last_frame = Instant::now();

                // tick handling
                self.ticks_owed += frame_time.as_secs_f64() / PHYS_TIME_STEP;
                for _ in 0..(self.ticks_owed as u32).min(20) {
                    app_state.phys_tick();
                }
                self.ticks_owed = self.ticks_owed.rem_euclid(1.0);
                
                // where the magic happens
                app_state.render(frame_time.as_secs_f64());

                // mouse logic
                let new_mouse_locked = app_state.input_controller.is_mouse_locked();
                if new_mouse_locked != self.mouse_locked {
                    if new_mouse_locked {
                        window.set_cursor_grab(CursorGrabMode::Locked).unwrap_or_else(|_| {
                            let _ = window.set_cursor_grab(CursorGrabMode::Confined);
                        });
                        window.set_cursor_visible(false);
                    } else {
                        window.set_cursor_grab(CursorGrabMode::None).unwrap();
                        window.set_cursor_visible(true);
                    }
                }
                self.mouse_locked = new_mouse_locked;
    
                app_state.input_controller.clear_inputs();

                window.request_redraw();
            }
            WindowEvent::Resized(new_size) => {
                app_state.graphics_controller.resize(new_size);
            }
            WindowEvent::Focused(is_focused) => {
                app_state.window_focus_changed(is_focused);
            }
            _ => {

            }
        }
    }

    fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: DeviceId,
            event: DeviceEvent,
        ) {
        let (_, game_state) = match (&self.window, &mut self.app_state) {
            (Some(window), Some(app_state)) => (window, app_state),
            _ => return,
        };

        game_state.winit_event(WinitEvent::Device(&event))
    }
}

fn main() -> Result<()> {
    env_logger::builder().format_timestamp(None).init();

    let mut app = App {
        window: None,
        app_state: None,
        mouse_locked: false,
        last_frame: Instant::now(),
        ticks_owed: 0.0,
    };

    EventLoop::new().unwrap().run_app(&mut app)?;

    Ok(())
}
