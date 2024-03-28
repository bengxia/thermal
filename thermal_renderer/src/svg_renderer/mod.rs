use crate::renderer::CommandRenderer;
use std::io;
use thermal_parser::command::DeviceCommand;
use thermal_parser::context::Context;

use once_cell::sync::Lazy;
use std::sync::Mutex;

pub mod svg_image;
use svg_image::*;

pub struct SvgRenderer {
    pub canvas: SvgImage,
    pub text_layout: Option<TextLayout>,
    pub device: Box<dyn io::Write>,
}

const BASE_FONT_SIZE: (usize, usize) = (12, 24); // (width, height) in pixels

static FONT_LIST: Lazy<Mutex<EncodingFontMap>> = Lazy::new(|| {
    let mut m = EncodingFontMap::new();

    // simple chinese
    let font_sc = FontDesc {
        size: (22, 24),
        ..FontDesc::default()
    };
    // traditional chinese
    let font_tc = FontDesc {
        size: (22, 24),
        ..FontDesc::default()
    };
    // korean
    let font_k = FontDesc {
        size: (22, 24),
        ..Default::default()
    };
    // japanese
    let font_j = FontDesc {
        family: "'Kosugi Maru', 'MS Gothic', 'San Francisco', 'Osaka-Mono', monospace".to_string(),
        style: "@import url('https://fonts.loli.net/css2?family=Kosugi+Maru&display=swap');"
            .to_string(),
        size: (24, 24),
    };
    // thai
    let font_t = FontDesc {
        family: "'Sarabun', monospace".to_string(),
        style: "@import url('https://fonts.loli.net/css2?family=Sarabun&display=swap');"
            .to_string(),
        size: (20, 24),
    };
    // default
    let font_default = FontDesc {
        family: "'Courier Prime', 'Courier New', 'Courier', monospace".to_string(),
        style: "@import url('https://fonts.loli.net/css2?family=Courier+Prime&display=swap);"
            .to_string(),
        size: (22, 24),
    };
    m.insert("utf-8".to_string(), font_default.clone()); // default encoding/font

    m.insert("cp936".to_string(), font_sc.clone());
    m.insert("gbk".to_string(), font_sc.clone());
    m.insert("gb18030".to_string(), font_sc.clone());

    m.insert("cp932".to_string(), font_j.clone());
    m.insert("shiftjis".to_string(), font_j.clone());

    m.insert("cp950".to_string(), font_tc.clone());
    m.insert("big5".to_string(), font_tc.clone());

    m.insert("cp949".to_string(), font_k.clone());
    m.insert("ksc5401".to_string(), font_k.clone());

    m.insert("tis620".to_string(), font_t.clone());
    Mutex::new(m)
});
impl SvgRenderer {
    pub fn new(d: Box<dyn io::Write>) -> Self {
        Self {
            canvas: SvgImage::new(FONT_LIST.lock().unwrap().clone(), 576),
            text_layout: None,
            device: d,
        }
    }
}

impl CommandRenderer for SvgRenderer {
    fn begin_render(&mut self, context: &mut Context) {
        // svg renderer
        // 1. initialize svg canvas size, viewbox, etc
        self.canvas
            .set_width(context.available_width_pixels() as u16);
    }

    fn begin_graphics(&mut self, context: &mut Context) {
        self.maybe_render_text(context);
    }

    fn draw_rect(&mut self, context: &mut Context, w: usize, h: usize) {
        todo!()
    }
    fn end_graphics(&mut self, _context: &mut Context) {}

    fn draw_image(&mut self, context: &mut Context, bytes: Vec<u8>, width: usize, height: usize) {
        self.maybe_render_text(context);
    }

    fn draw_text(&mut self, context: &mut Context, text: String) {
        // copycat from thermal_renderer/src/image_renderer/mod.rs
        if self.text_layout.is_none() && text.eq("\n") {
            context.graphics.y += context.text.line_spacing as usize;
            return;
        }

        // layout contains multiple spans, TextSpan holds the text and it's style
        let encoding = context.text.encoding.as_str();
        let font = self.canvas.get_fontdesc(context.text.encoding.as_str());
        let span = TextSpan::new(font, text.to_string(), context);

        if self.text_layout.is_none() {
            // first span
            self.text_layout = Some(TextLayout {
                spans: vec![span],
                line_height: context.line_height_pixels() as usize,
                tab_len: context.text.tab_len as usize,
            });
        } else {
            // add this span to the layout
            if let Some(layout) = &mut self.text_layout {
                layout.spans.push(span);
            }
        }
    }

    fn draw_device_command(&mut self, context: &mut Context, _command: &DeviceCommand) {
        self.maybe_render_text(context);
    }

    fn end_render(&mut self, context: &mut Context) {
        self.maybe_render_text(context);
        context.graphics.x = 0;
        context.graphics.y = 0;
    }
}

impl SvgRenderer {
    pub fn maybe_render_text(&mut self, context: &mut Context) {
        if let Some(layout) = &mut self.text_layout {
            // if there's text layout in buffer, render it
            let (_, y) = self.canvas.draw_text(
                context.graphics.x as usize,
                context.graphics.y as usize,
                self.canvas.width as usize,
                layout,
            );
            context.graphics.y = y; // update y coordinate
            self.text_layout = None; // clear the text layout
        }
    }
    fn start_group(&mut self, context: &mut Context) {
        self.canvas.svg_content.push(format!(
            "<g transform=\"translate({}, {})\">",
            context.graphics.x, context.graphics.y
        ));
    }
    fn end_group(&mut self) {
        self.canvas.svg_content.push("</g>".to_string());
    }
}
