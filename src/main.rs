use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::iter;
use std::sync::Mutex;
use std::time::Instant;

use lazy_static::lazy_static;
use slotmap::{DefaultKey, SlotMap};
use wgpu::{AddressMode, BlendState, Color, Device, Extent3d, Features, FilterMode, IndexFormat, RenderPass, Sampler, SamplerDescriptor, TextureFormat};
use winit::window::Window;

use basic_render_state::BasicRenderState;
use terminal::Terminal;
use terminal::test;
use text::TextPass;

use crate::drawrects::{ColoredDrawRects, ColoredTriangleVertex};
use crate::gpu_device::{device, init_device};

mod gpu_device;
mod fps_counter;
mod drawrects;
mod terminal;
mod basic_render_state;
mod text;
mod input_state;
mod bezier;

pub fn load_file(path: &str) -> String {
    let mut buf = String::new();
    std::fs::File::open(path).unwrap().read_to_string(&mut buf).unwrap();
    buf
}

struct State {
    surface: wgpu::Surface,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    tp: TextPass,
    rp: RectPass,
}

struct RectPass {
    state: BasicRenderState,
    verts: ColoredDrawRects,
    rects: SlotMap<DefaultKey, RectObject>,
}

impl RectPass {
    fn new() -> Self {
        let verts = ColoredDrawRects::new();
        Self {
            state: BasicRenderState::new("rect", 4, Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,

            }, verts.layout.clone(), BlendState::ALPHA_BLENDING),
            verts,
            rects: Default::default(),
        }
    }

    fn add_rect(&mut self, ro: RectObject) -> DefaultKey {
        self.rects.insert(ro)
    }

    fn upload_data(&mut self, q: &mut wgpu::Queue) {
        for r in self.rects.values() {
            let rp = RectanglePoint::<i32> {
                x: r.x as i32,
                y: r.y as i32,
                x1: (r.x + r.w) as i32,
                y1: (r.y + r.h) as i32,
            };
            let rp = rp.div_by_float(WIDTH as f64, HEIGHT as f64);
            self.verts.extend([
                ColoredTriangleVertex { position: [rp.x, rp.y], color: r.color },
                ColoredTriangleVertex { position: [rp.x1, rp.y], color: r.color },
                ColoredTriangleVertex { position: [rp.x, rp.y1], color: r.color },
                ColoredTriangleVertex { position: [rp.x1, rp.y1], color: r.color },
            ]);
        }
        self.verts.confirm_extends(q);
    }

    fn finish(&mut self) {
        self.verts.finish();
    }

    fn render_self<'a>(&'a mut self, p: &mut RenderPass<'a>, q: &mut wgpu::Queue) {
        self.upload_data(q);
        p.set_pipeline(&self.state.render_pipeline);
        p.set_vertex_buffer(0, self.verts.get_vertex_buf());
        p.set_index_buffer(self.verts.get_index_buffer(), IndexFormat::Uint32);
        p.set_bind_group(0, &self.state.bind_group, &[]);
        p.draw_indexed(0..(self.verts.index_buffer.len()) as u32, 0, 0..1);
    }
}

#[derive(Debug)]
struct RectObject {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    color: [u8; 4],
}


#[derive(Debug)]
struct RectanglePoint<T = i32> {
    x: T,
    y: T,
    x1: T,
    y1: T,
}

impl RectanglePoint<i32> {
    fn div_by_float(&self, fx: f64, fy: f64) -> RectanglePoint<f32> {
        RectanglePoint { x: (self.x as f64 / fx) as f32, y: (self.y as f64 / fy) as f32, x1: (self.x1 as f64 / fx) as f32, y1: (self.y1 as f64 / fy) as f32 }
    }
}

impl RectanglePoint<f32> {
    fn div_by_float(&self, fx: f64, fy: f64) -> RectanglePoint<f32> {
        RectanglePoint { x: (self.x as f64 / fx) as f32, y: (self.y as f64 / fy) as f32, x1: (self.x1 as f64 / fx) as f32, y1: (self.y1 as f64 / fy) as f32 }
    }
}

impl<T> RectanglePoint<T> {
    fn as_array(self) -> [T; 4] {
        [self.x, self.y, self.x1, self.y1]
    }
}

impl<T: Display> Display for RectanglePoint<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Rect {} {} {} {}", self.x, self.x1, self.y, self.y1))
    }
}


fn create_sampler(device: &Device) -> Sampler {
    device.create_sampler(&SamplerDescriptor {
        label: None,
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        ..Default::default()
    })
}

lazy_static! {
    static ref RANDFILE: Mutex<File> = Mutex::new(File::open("file").unwrap());
}



impl State {
    fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = pollster::block_on(instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })).unwrap();

        let (device, queue) = pollster::block_on(adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )).unwrap();

        let device = init_device(device);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(device, &config);

        let tp = TextPass::new(&queue);
        let rp = RectPass::new();
        Self {
            surface,
            queue,
            size,
            tp,
            rp,
        }
    }


    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        self.rp.render_self(&mut render_pass, &mut self.queue);
        self.tp.render_self(&mut render_pass, &mut self.queue, (0.0, 0.0));

        std::mem::drop(render_pass);

        self.tp.finish();
        self.rp.finish();
        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn update(&mut self) {
        self.tp.update();
    }
}

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 400;

fn main() {
    env_logger::init();

    // let mut term = Terminal::new();
    test();
    // term.run();
}


