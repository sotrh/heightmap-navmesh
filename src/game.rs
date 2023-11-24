use anyhow::Context;
use winit::{
    dpi::PhysicalSize,
    keyboard::KeyCode,
    window::{Fullscreen, Window},
};

use crate::{
    pipelines::fur::Fur,
    resources::{
        camera::{Camera, CameraBinder, CameraBinding},
        model::Model,
        texture::Texture,
    },
};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GameConfig {
    fullscreen: bool,
    monitor: Option<String>,
    mouse_sensitivity: f32,
    width: u32,
    height: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            fullscreen: false,
            monitor: None,
            mouse_sensitivity: 0.1,
            width: 1920,
            height: 1080,
        }
    }
}

pub struct Game {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surf_config: wgpu::SurfaceConfiguration,
    running: bool,
    model: Model,
    depth_texture: Texture,
    fur: Fur,
    window: Window,
    camera: Camera,
    camera_binding: CameraBinding,
    last_time: Option<instant::Instant>,
    mouse_sensitivity: f32,
    lmb_pressed: bool,
}

impl Game {
    pub async fn new(config: GameConfig, window: Window) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(Default::default());

        // Safety: surface and window are owned by game
        let surface = unsafe { instance.create_surface(&window)? };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .context("No valid adapter")?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await?;

        if config.fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(find_or_first(
                window.available_monitors(),
                |m| m.name() == config.monitor,
            ))))
        } else {
            let _ = window.request_inner_size(PhysicalSize {
                width: config.width,
                height: config.height,
            });
        }

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];

        println!("caps: {:?}", caps);

        let surf_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surf_config);

        println!("format: {:?}", format);

        let depth_texture = Texture::depth_texture(&device, surf_config.width, surf_config.height);

        let camera_binder = CameraBinder::new(&device);
        let camera = Camera::look_at(
            glam::vec3(0.0, 0.0, -4.0),
            glam::vec3(0.0, 0.0, 0.0),
            surf_config.width as _,
            surf_config.height as _,
            1.0,
            0.1,
            100.0,
        );
        let camera_binding = camera_binder.bind(&device, &camera);

        let fur = Fur::new(
            &device,
            32,
            surf_config.format,
            depth_texture.format(),
            &camera_binder,
        );

        let model = Model::load(&device, &queue, "res/shape-keys.glb").await?;

        Ok(Self {
            device,
            queue,
            surface,
            surf_config,
            running: true,
            mouse_sensitivity: config.mouse_sensitivity,
            depth_texture,
            fur,
            model,
            camera,
            camera_binding,
            last_time: None,
            lmb_pressed: false,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surf_config.width = width;
        self.surf_config.height = height;
        self.surface.configure(&self.device, &self.surf_config);
        self.camera.resize(self.surf_config.width, self.surf_config.height);
        self.depth_texture = Texture::depth_texture(&self.device, width, height);
    }

    pub fn render(&mut self) {
        if !self.is_running() {
            return;
        }

        self.window.request_redraw();

        let target = match self.surface.get_current_texture() {
            Ok(target) => target,
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.surf_config);
                return
            }
            Err(e) => {
                eprintln!("{}", e);
                self.running = false;
                return;
            }
        };

        let current_time = instant::Instant::now();
        let dt = if let Some(last_time) = self.last_time.as_mut() {
            current_time - *last_time
        } else {
            instant::Duration::ZERO
        };
        self.last_time = Some(current_time);

        self.camera_binding.update(&self.queue, &self.camera);

        let view = target.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        store: wgpu::StoreOp::Store,
                        load: wgpu::LoadOp::Clear(if self.window.fullscreen().is_none() { wgpu::Color::BLACK } else { wgpu::Color::BLUE }),
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.depth_texture.view(),
                    depth_ops: Some(wgpu::Operations {
                        store: wgpu::StoreOp::Store,
                        load: wgpu::LoadOp::Clear(1.0),
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.fur.draw(&mut pass, &self.model, &self.camera_binding);
        }

        self.queue.submit([encoder.finish()]);
        target.present();
    }

    pub fn show(&self) {
        self.window.set_visible(true);
    }

    pub fn toggle_fullscreen(&mut self) {
        if self.is_fullscreen() {
            self.window.set_fullscreen(None);
        } else {
            self.window
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
            // self.window.set_fullscreen(Some(Fullscreen::Exclusive(self.window.current_monitor().unwrap().video_modes().next().unwrap())));
        }
    }

    fn is_fullscreen(&mut self) -> bool {
        self.window.fullscreen().is_some()
    }

    pub fn export_config(&self) -> GameConfig {
        let size = self.window.inner_size();
        GameConfig {
            fullscreen: self.window.fullscreen().is_some(),
            monitor: self.window.current_monitor().map(|m| m.name()).flatten(),
            mouse_sensitivity: self.mouse_sensitivity,
            width: size.width,
            height: size.height,
        }
    }

    pub fn handle_axis(&mut self, axis: u32, value: f32) {
        if self.lmb_pressed {
            match axis {
                0 => self.camera.rotate_right(-value * self.mouse_sensitivity),
                1 => self.camera.rotate_up(value * self.mouse_sensitivity),
                _ => (),
            }
        }
    }

    pub fn handle_mouse_button(&mut self, button: winit::event::MouseButton, pressed: bool) {
        match button {
            winit::event::MouseButton::Left => {
                self.lmb_pressed = pressed;
                if self.lmb_pressed {
                    self.window.set_cursor_visible(false);
                } else {
                    self.window.set_cursor_visible(true);
                }
            }
            winit::event::MouseButton::Right => (),
            winit::event::MouseButton::Middle => (),
            winit::event::MouseButton::Back => (),
            winit::event::MouseButton::Forward => (),
            winit::event::MouseButton::Other(_) => (),
        }
        println!("lmb: {}", self.lmb_pressed);
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        match (key, pressed) {
            (KeyCode::Escape, true) => self.running = false,
            (KeyCode::F11, true) => self.toggle_fullscreen(),
            (KeyCode::KeyW, true) => self.camera.walk_forward(0.5),
            (KeyCode::KeyS, true) => self.camera.walk_forward(-0.5),
            (KeyCode::KeyD, true) => self.camera.walk_right(-0.5),
            (KeyCode::KeyA, true) => self.camera.walk_right(0.5),
            _ => (),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

}

fn find_or_first<T>(iter: impl Iterator<Item = T>, predicate: impl Fn(&T) -> bool) -> Option<T> {
    let mut found = None;

    for item in iter {
        if found.is_none() || predicate(&item) {
            found = Some(item);
        }
    }

    found
}
