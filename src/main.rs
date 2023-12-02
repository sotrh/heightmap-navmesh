mod game;
mod resources;
mod pipelines;

use game::Game;
use pollster::FutureExt;
use resources::{load_json, save_json};
use winit::{
    event::{ElementState, Event, WindowEvent, KeyEvent, DeviceEvent},
    event_loop::EventLoop,
    window::WindowBuilder, keyboard::PhysicalKey,
};

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;

    let window = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)?;

    let config = load_json("config.json").block_on().unwrap_or_default();

    let mut game = Game::new(config, window).block_on()?;

    event_loop.run(move |event, target| match event {
        Event::NewEvents(_) => game.show(),
        Event::AboutToWait => {
            if !game.is_running() {
                target.exit();
            }
        }
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => target.exit(),
            WindowEvent::Resized(size) => game.resize(size.width, size.height),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => game.handle_keyboard(key, state == ElementState::Pressed),
            WindowEvent::RedrawRequested => {
                game.render();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                game.handle_mouse_button(button, state == ElementState::Pressed);
            }
            _ => (),
        },
        Event::DeviceEvent { device_id, event } => match event {
            DeviceEvent::Added => println!("Added: {device_id:?}"),
            DeviceEvent::Removed => println!("Removed: {device_id:?}"),
            DeviceEvent::Motion { axis, value } => game.handle_axis(axis, value as f32),
            _ => (),
        }
        Event::LoopExiting => {
            save_json("config.json", game.export_config())
                .block_on()
                .unwrap();
        }
        _ => (),
    })?;

    Ok(())
}
