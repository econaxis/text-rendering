use std::cell::UnsafeCell;
use std::ops::Deref;
use lazy_static::lazy_static;
use wgpu::{ShaderModule, ShaderModuleDescriptor, ShaderSource};
use crate::load_file;

pub struct UnsafeGuaranteeLocal<T>(UnsafeCell<T>);

impl<T> UnsafeGuaranteeLocal<T> {
    fn new(t: T) -> Self {
        Self(UnsafeCell::new(t))
    }
    fn get_mut(&self) -> &mut T {
        unsafe { &mut *(self.0.get() as *mut T) }
    }
}

impl<T> Deref for UnsafeGuaranteeLocal<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.get() }
    }
}
lazy_static! {
    pub static ref DEVICE: UnsafeGuaranteeLocal<Option<wgpu::Device>> = UnsafeGuaranteeLocal::new(None);
    pub static ref SHADER: ShaderModule = DEVICE.as_ref().unwrap().create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader Module"),
            source: ShaderSource::Wgsl(load_file("/home/henry/twodr/shaders.wgsl").into()),
    });
}

pub fn init_device(d: wgpu::Device) -> &'static wgpu::Device {
    *DEVICE.get_mut() = Some(d);
    DEVICE.as_ref().unwrap()
}

pub fn device() -> &'static wgpu::Device {
    DEVICE.as_ref().unwrap()
}
pub fn shader() -> &'static ShaderModule {
    &SHADER
}

unsafe impl<T> Send for UnsafeGuaranteeLocal<T> {}

unsafe impl<T> Sync for UnsafeGuaranteeLocal<T> {}
