use crate::renderer::CommandRenderer;
use std::io;
use thermal_parser::command::DeviceCommand;
use thermal_parser::context::Context;

pub mod svg_image;
use svg_image::*;

pub struct SvgRenderer {
    pub canvas: SvgImage,
    pub text_layout: Option<TextLayout>,
    pub device: Box<dyn io::Write>,
}

const BASE_FONT_SIZE: (usize, usize) = (12, 24); // (width, height) in pixels
const DEFAULT_PRINTABLE_WIDTH: u16 = 576;
const DEFAULT_CPL: u8 = 48; // 12*24px character in ANK, 24*24px character in CJK
const DEFAULT_FONT_FAMILY: &str = "monospace";

impl SvgRenderer {
    pub fn new(d: Box<dyn io::Write>) -> Self {
        Self {
            canvas: SvgImage::new(DEFAULT_PRINTABLE_WIDTH),
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

    fn draw_rect(&mut self, _context: &mut Context, w: usize, h: usize) {
        println!("rect {} {}", w, h);
    }
    fn end_graphics(&mut self, _context: &mut Context) {}

    fn draw_image(&mut self, context: &mut Context, bytes: Vec<u8>, width: usize, height: usize) {
        self.maybe_render_text(context);
        println!("image {} {} {}", width, height, bytes.len());
    }

    fn draw_text(&mut self, context: &mut Context, text: String) {
        // copycat from thermal_renderer/src/image_renderer/mod.rs
        if self.text_layout.is_none() && text.eq("\n") {
            context.graphics.y += context.text.line_spacing as usize;
            return;
        }

        // layout contains multiple spans, TextSpan holds the text and it's style
        let span = TextSpan::new(text.to_string(), context);

        if self.text_layout.is_none() {
            // first span
            self.text_layout = Some(TextLayout {
                spans: vec![span],
                line_height: BASE_FONT_SIZE.1,
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
        let svgdoc = self.canvas.construct_svg_doc(&context);
        self.device.write(svgdoc.as_bytes()).unwrap();
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
    // fn start_group(&mut self, context: &mut Context) {
    //     self.canvas.svg_content.push(format!(
    //         "<g transform=\"translate({}, {})\">",
    //         context.graphics.x, context.graphics.y
    //     ));
    // }
    // fn end_group(&mut self) {
    //     self.canvas.svg_content.push("</g>".to_string());
    // }
}
