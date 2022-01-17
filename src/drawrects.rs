use std::ptr;
use bytemuck::{Pod, Zeroable};
use lazy_static::lazy_static;
use wgpu::{Buffer, BufferAddress, BufferDescriptor, BufferSlice, BufferUsages, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::gpu_device::device;

lazy_static! {
    pub static ref TEXT_VERTEX_ATTRIBUTES: [VertexAttribute; 2] =  wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
    pub static ref COLORED_RECT_VERTEX_ATTRIBUTES: [VertexAttribute; 2] =  wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32];
}

pub type Rect<T> = [T; 4];
pub type FontDrawRects = DrawRects<FontTriangleVertex>;
pub type ColoredDrawRects = DrawRects<ColoredTriangleVertex>;

pub struct DrawRects<VertexT> {
    pub vertex_buffer: Buffer,
    pub vertex_buffer_sz: u32,
    pub cpu_buffer: Vec<VertexT>,
    pub index_buffer: Vec<u32>,
    pub layout: VertexBufferLayout<'static>,
    pub index_buffer_dirty: bool,
}

impl FontDrawRects {
    pub fn new() -> Self {
        Self::new_with_layout(&*TEXT_VERTEX_ATTRIBUTES)
    }
}

impl ColoredDrawRects {
    pub fn new() -> Self {
        Self::new_with_layout(&*COLORED_RECT_VERTEX_ATTRIBUTES)
    }
}

impl<T: bytemuck::Pod> DrawRects<T> {
    pub fn finish(&mut self) {
        self.cpu_buffer.clear();
        self.index_buffer.clear();
    }
    const START_BUF_SIZE: u32 = 3000;
    pub fn get_vertex_buf(&self) -> BufferSlice {
        self.vertex_buffer.slice(..self.cpu_buffer_len() as u64)
    }

    pub fn extend(&mut self, r: Rect<T>) {
        fn extend_arr<T, const A: usize>(vec: &mut Vec<T>, arr: [T; A]) {
            if vec.capacity() < vec.len() + A {
                vec.reserve(vec.len() + A);
            }
            unsafe {
                let end = vec.as_mut_ptr().add(vec.len()) as *mut [T; A];
                ptr::write(end, arr);
                vec.set_len(vec.len() + A);
            }

        }
        let offset = self.cpu_buffer.len() as u32;
        extend_arr(&mut self.cpu_buffer, r);
        extend_arr(&mut self.index_buffer, [offset + 2, offset + 1, offset + 0]);
        extend_arr(&mut self.index_buffer, [offset + 2, offset + 3, offset + 1]);
    }

    fn cpu_buffer_len(&self) -> usize {
        self.cpu_buffer.len() * std::mem::size_of::<T>()
    }
    pub fn get_index_buffer(&self) -> BufferSlice {
        self.vertex_buffer.slice(self.cpu_buffer_len() as u64..)
    }
    pub fn confirm_extends(&mut self, queue: &mut wgpu::Queue) {
        // Clear all removed elements
        if self.cpu_buffer_len() + self.index_buffer.len() * 4 > self.vertex_buffer_sz as usize {
            self.vertex_buffer_sz = (self.cpu_buffer_len() + self.index_buffer.len() * 4) as u32 * 2;
            self.vertex_buffer = device().create_buffer(&BufferDescriptor {
                label: None,
                usage: BufferUsages::VERTEX | BufferUsages::INDEX | BufferUsages::COPY_DST,
                size: self.vertex_buffer_sz as u64,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0_u64, bytemuck::cast_slice(&self.cpu_buffer));
        queue.write_buffer(&self.vertex_buffer, self.cpu_buffer_len() as BufferAddress, bytemuck::cast_slice(&self.index_buffer));
        self.index_buffer_dirty = false;
    }
    pub fn new_with_layout(vertex_attrib_layout: &'static [VertexAttribute]) -> Self {
        let cpu_buffer = Vec::new();
        let layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<T>() as wgpu::BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: vertex_attrib_layout,
        };

        let vertex_buffer = device().create_buffer(&BufferDescriptor {
            label: None,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::INDEX,
            size: Self::START_BUF_SIZE as u64,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            vertex_buffer_sz: Self::START_BUF_SIZE,
            cpu_buffer,
            index_buffer: Vec::new(),
            layout,
            index_buffer_dirty: true,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct FontTriangleVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl FontTriangleVertex {
    pub fn new(pos: (f32, f32), tex: (f32, f32)) -> Self {
        Self {
            position: [pos.0, pos.1],
            tex_coords: [tex.0, tex.1],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct ColoredTriangleVertex {
    pub(crate) position: [f32; 2],
    pub(crate) color: [u8; 4],
}


unsafe impl Pod for FontTriangleVertex {}

unsafe impl Zeroable for FontTriangleVertex {}

unsafe impl Pod for ColoredTriangleVertex {}

unsafe impl Zeroable for ColoredTriangleVertex {}
