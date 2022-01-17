use std::collections::HashMap;
use std::io::Read;
use std::num::NonZeroU32;

use freetype::{GlyphMetrics, Library};
use freetype::face::LoadFlag;
use image::{Rgba, RgbaImage};
use slotmap::{DefaultKey, SlotMap};
use wgpu::{BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d, ImageCopyTexture, ImageDataLayout, IndexFormat, RenderPass};

use crate::{HEIGHT, RANDFILE, RectanglePoint, WIDTH};
use crate::basic_render_state::BasicRenderState;
use crate::drawrects::{FontDrawRects, FontTriangleVertex};

#[derive(Debug, Clone)]
pub struct TextInfo {
    pub min: (i32, i32),
    pub max: (i32, i32),
}

pub struct TextPass {
    state: BasicRenderState,
    verts: FontDrawRects,
    pub fontatl: FontAtlas,
    time: f32,
    text_objects: SlotMap<DefaultKey, TextObject>,
    text_info: HashMap<DefaultKey, TextInfo>,
    dirty: bool,
}

pub struct FontAtlas {
    img: RgbaImage,
    pub face: freetype::Face,
    advances: Vec<GlyphInfo>,
}

#[derive(Debug, Default, Clone)]
struct GlyphInfo {
    advance: i32,
    bearing: (i32, i32),
    size: (i32, i32),
    texture_coord: (i32, i32),
}

impl GlyphInfo {
    fn calculate_rect_pos(&self, origin: (i32, i32)) -> RectanglePoint {
        let left_edge = origin.0 + self.bearing.0;
        let top_edge = origin.1 + self.bearing.1;
        let bottom_edge = top_edge - self.size.1;
        let right_edge = left_edge + self.size.0;

        RectanglePoint {
            x: left_edge,
            y: top_edge,
            x1: right_edge,
            y1: bottom_edge,
        }
    }
    fn calculate_texture(&self) -> RectanglePoint<f32> {
        RectanglePoint {
            x: self.texture_coord.0 as f32,
            y1: self.texture_coord.1 as f32 + (self.size.1 / 64) as f32,
            y: self.texture_coord.1 as f32,
            x1: self.texture_coord.0 as f32 + (self.size.0 / 64) as f32,
        }
    }
    fn calculate_next_origin(&self, origin: (i32, i32)) -> (i32, i32) {
        (origin.0 + self.advance as i32, origin.1)
    }
    fn from_metrics(metrics: &GlyphMetrics, texture_coord: (i32, i32), texture_size: (i32, i32)) -> Self {
        Self {
            advance: metrics.horiAdvance as i32,
            bearing: (metrics.horiBearingX as i32, metrics.horiBearingY as i32),
            size: (texture_size.0 as i32 * 64, texture_size.1 as i32 * 64),
            texture_coord,
        }
    }
}

fn load_font_atlas() -> FontAtlas {
    let lib = Library::init().unwrap();
    let mut advances = vec![GlyphInfo::default(); 127];

    let face = lib.new_face("/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf", 0).unwrap();
    face.set_char_size(16 * 64, 0, 0, 0).unwrap();
    let mut max_height: u32 = 1;
    let mut width: u32 = 1;
    for i in 32..127 {
        face.load_glyph(i, LoadFlag::DEFAULT).unwrap();
        let glyph = face.glyph();
        glyph.render_glyph(freetype::RenderMode::Lcd).unwrap();
        let bitmap = glyph.bitmap();
        width += bitmap.width() as u32 / 3 + 2;
        max_height = max_height.max(bitmap.rows() as u32);
    }
    let mut image: RgbaImage = RgbaImage::new(width, max_height + 2);
    let mut next_x: u32 = 0;
    for i in 32..127 {
        face.load_char(i, LoadFlag::DEFAULT).unwrap();
        let glyph = face.glyph();
        glyph.render_glyph(freetype::RenderMode::Lcd).unwrap();
        let metrics = glyph.metrics();

        let bitmap = glyph.bitmap();
        let bitmap_buf = bitmap.buffer();
        let width = bitmap.width().abs() as u32 / 3;
        let height = bitmap.rows() as u32;

        if !(width == 0 || height == 0) {
            for y in 0..height {
                let start = y * bitmap.pitch() as u32;
                let end = start + bitmap.width() as u32;
                for x in (start..end).step_by(3) {
                    let xi = x as usize;
                    let xglobal = (x - start) / 3 + next_x;
                    let pixel = Rgba([bitmap_buf[xi], bitmap_buf[xi + 1], bitmap_buf[xi + 2], 255]);
                    image.put_pixel(xglobal, y, pixel);
                }
            }
        }
        advances[i] = GlyphInfo::from_metrics(&metrics, (next_x as i32, 0), (width as i32, height as i32));
        next_x += width + 2;
    }
    image.save("/tmp/font.png").unwrap();
    FontAtlas {
        img: image,
        face,
        advances,
    }
}

impl FontAtlas {
    fn size(&self) -> Extent3d {
        Extent3d {
            width: self.img.width(),
            height: self.img.height(),
            depth_or_array_layers: 1,
        }
    }
    pub fn font_height(&self) -> u32 {
        self.face.size_metrics().unwrap().height as u32
    }
}


#[allow(unused)]
fn random_str(len: usize) -> String {
    let mut vec: Vec<u8> = Vec::with_capacity(len);
    let slice = unsafe {
        std::slice::from_raw_parts_mut(vec.as_mut_ptr(), len)
    };
    RANDFILE.lock().unwrap().read_exact(slice).unwrap();
    unsafe { vec.set_len(len) };
    String::from_utf8(vec).unwrap()
}

pub struct TextObjectHandle(DefaultKey);

impl TextObjectHandle {
    pub fn resolve<'a>(&self, tp: &'a TextPass) -> &'a TextObject {
        tp.query(self.0)
    }

    pub fn resolve_mut<'a>(&self, tp: &'a mut TextPass) -> &'a mut TextObject {
        tp.query_mut(self.0)
    }

    pub fn get_stats<'a>(&self, tp: &'a TextPass) -> Option<&'a TextInfo> {
        tp.text_info.get(&self.0)
    }
    pub fn update_str(&self, tp: &mut TextPass, s: String) {
        self.resolve_mut(tp).update_str(s);
    }
    pub fn append_str(&self, tp: &mut TextPass, s: &str) {
        let to = self.resolve_mut(tp);
        to.render_str.push_str(s);
    }
    pub fn add_offset(&self, tp: &mut TextPass, offset: (i32, i32)) {
        let to = self.resolve_mut(tp);
        to.top_left = (to.top_left.0 + offset.0, to.top_left.1 + offset.1);
    }
}


impl TextPass {
    pub(crate) fn new(queue: &wgpu::Queue) -> Self {
        let fontatl = load_font_atlas();
        let atl_size = fontatl.size();

        let verts = FontDrawRects::new();
        let basic_state = BasicRenderState::new("font", 160, atl_size, verts.layout.clone(), BlendState::ALPHA_BLENDING);
        queue.write_texture(ImageCopyTexture {
            texture: &basic_state.texture,
            mip_level: 0,
            origin: Default::default(),
            aspect: Default::default(),
        }, fontatl.img.as_raw(), ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(NonZeroU32::try_from(fontatl.img.width() * 4).unwrap()),
            rows_per_image: Some(NonZeroU32::try_from(fontatl.img.height()).unwrap()),
        }, atl_size);

        Self {
            state: basic_state,
            verts,
            fontatl,
            time: 1.0,
            text_objects: Default::default(),
            text_info: Default::default(),
            dirty: true,
        }
    }
    fn draw_text(fontatl: &FontAtlas, verts: &mut FontDrawRects, TextObject { render_str, top_left, max_width, dirty: _ }: &TextObject) -> TextInfo {
        let top_left = (top_left.0 * 64, top_left.1 * 64);
        let max_width = max_width * 64;
        let atl_size = fontatl.size();
        let mut cursor_origin = (top_left.0, top_left.1 - fontatl.font_height() as i32);
        let iter = render_str.chars();

        for c in iter {
            if (cursor_origin.0 - top_left.0) as u32 > max_width || c == 0xd as char {
                cursor_origin.0 = top_left.0;
                cursor_origin.1 -= fontatl.font_height() as i32;
            }
            if let Some(gl_info) = &fontatl.advances.get(c as u8 as usize) {
                let rect_pos = gl_info.calculate_rect_pos(cursor_origin);
                let tex_pos = gl_info.calculate_texture();


                let [rectx, recty, rectx1, recty1] = rect_pos.div_by_float(WIDTH as f64 * 64.0, HEIGHT as f64 * 64.0).as_array();
                let [textx, texty, textx1, texty1] = tex_pos.div_by_float(atl_size.width as f64, atl_size.height as f64).as_array();

                verts.extend([
                    FontTriangleVertex::new((rectx, recty), (textx, texty)),
                    FontTriangleVertex::new((rectx1, recty), (textx1, texty)),
                    FontTriangleVertex::new((rectx, recty1), (textx, texty1)),
                    FontTriangleVertex::new((rectx1, recty1), (textx1, texty1)),
                ]);

                cursor_origin = gl_info.calculate_next_origin(cursor_origin);
            }
        }
        TextInfo {
            min: top_left,
            max: cursor_origin,
        }
    }

    pub(crate) fn add_text(&mut self, to: TextObject) -> TextObjectHandle {
        self.dirty = true;
        TextObjectHandle(self.text_objects.insert(to))
    }

    pub fn update(&mut self) {
        for (key, to) in &self.text_objects {
            let stats = Self::draw_text(&self.fontatl, &mut self.verts, to);
            self.text_info.insert(key, stats);
        }
        self.dirty = false;
    }

    pub(crate) fn render_self<'a>(&'a mut self, p: &mut RenderPass<'a>, queue: &mut wgpu::Queue, translate: (f32, f32)) {
        assert!(!self.dirty);

        self.verts.confirm_extends(queue);

        queue.write_buffer(&self.state.uniform_buffer, 0, bytemuck::cast_slice(&[translate.0, translate.1]));
        p.set_pipeline(&self.state.render_pipeline);
        p.set_vertex_buffer(0, self.verts.get_vertex_buf());
        p.set_index_buffer(self.verts.get_index_buffer(), IndexFormat::Uint32);
        p.set_bind_group(0, &self.state.bind_group, &[]);
        p.draw_indexed(0..(self.verts.index_buffer.len()) as u32, 0, 0..1);
    }

    pub fn query(&self, id: DefaultKey) -> &TextObject {
        self.text_objects.get(id).unwrap()
    }
    pub fn query_mut(&mut self, id: DefaultKey) -> &mut TextObject {
        self.dirty = true;
        self.text_objects.get_mut(id).unwrap()
    }

    pub(crate) fn finish(&mut self) {
        self.verts.finish();
    }
}


pub struct TextObject {
    pub render_str: String,
    pub top_left: (i32, i32),
    pub max_width: u32,
    pub dirty: bool,
}

impl TextObject {
    pub(crate) fn new(str: &str, bl: (i32, i32), width: u32) -> Self {
        Self {
            render_str: str.to_owned(),
            top_left: bl,
            max_width: width,
            dirty: true,
        }
    }
    pub(crate) fn update_str(&mut self, new: String) {
        self.render_str = new;
        self.dirty = true;
    }
}
