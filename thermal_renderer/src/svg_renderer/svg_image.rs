extern crate fontdue;
extern crate png;
extern crate textwrap;

use std::collections::HashMap;
use std::fmt::Write as _;
use textwrap::WordSeparator;
use thermal_parser::context::{Context, TextJustify, TextStrikethrough, TextUnderline};

use super::BASE_FONT_SIZE;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct FontDesc {
    pub family: String,
    pub style: String,
    pub size: (usize, usize), // width, height
}

impl Default for FontDesc {
    fn default() -> Self {
        Self {
            family: String::from("monospace"),
            style: String::from(""),
            size: (12, 24), // default ANK font is 12 × 24 dots(px), CJK is 24 × 24
        }
    }
}

pub type EncodingFontMap = HashMap<String, FontDesc>;

// #[derive(Default, Debug, Clone)]
// #[allow(dead_code)]
// struct TextElement {
//     text: String,
//     font: FontDesc,
//     attrs: HashMap<String, String>,
//     position: u16, // absolute x coordinate of the canvas
//     scale: u8,
//     encoding: String,
//     space: u8, // in dots, spaces between two characters, default 0
// }

/// A simple svg renderer designed for thermal printing effects emulation
/// This allows for an image with a fixed width that can grow in height
/// to accommodate sets of svg elements.
#[allow(dead_code)]
pub struct SvgImage {
    pub width: u16, // in dots, default 576 on 80mm width(printable width 72mm, 203dpi)
    pub height: u16,
    fonts: EncodingFontMap,
    pub svg_content: Vec<String>,
    pub base_cpl: u8, // number of characters per line, default width/12 in ANK, width/24 in CJK
    pub line_margin: u8,
    pub line_align: u8,  // 0 - left; 1 - center; 2 - right
    pub line_height: u8, // according font size
    pub feed_unit: u8,   // in dots, depends on character's height, line spacing & new line or not
    pub spacing: bool,
}

pub struct TextSpan {
    pub font: FontDesc,
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub stretch_width: u8,
    pub stretch_height: u8,
    pub inverted: bool,
    pub upside_down: bool,
    pub justify: TextJustify,
    pub encoding: String,
}

impl TextSpan {
    pub fn new(font: FontDesc, text: String, context: &Context) -> Self {
        let style = &context.text;

        let underline = match style.underline {
            TextUnderline::On => true,
            TextUnderline::Double => true,
            _ => false,
        };

        let strikethrough = match style.strikethrough {
            TextStrikethrough::On => true,
            TextStrikethrough::Double => true,
            _ => false,
        };

        Self {
            font,
            text,
            bold: style.bold,
            italic: style.italic,
            underline,
            strikethrough,
            stretch_width: style.width_mult,
            stretch_height: style.height_mult,
            inverted: style.invert,
            upside_down: style.upside_down,
            justify: context.text.justify.clone(),
            encoding: context.text.encoding.clone(),
        }
    }
}

pub struct TextLayout {
    pub spans: Vec<TextSpan>,
    pub line_height: usize,
    pub tab_len: usize,
}

impl SvgImage {
    pub fn new(fonts: EncodingFontMap, width: u16) -> Self {
        Self {
            width,
            height: 0,
            fonts,
            svg_content: Vec::new(),
            base_cpl: 48,
            line_margin: 0,
            line_align: 0,
            line_height: 24,
            feed_unit: 24,
            spacing: false,
        }
    }

    pub fn set_cpl(&mut self, cpl: u8) {
        self.base_cpl = cpl;
    }

    //Setting the width clears any bytes
    pub fn set_width(&mut self, width: u16) {
        self.width = width;
        self.svg_content = Vec::new();
    }

    pub fn get_fontdesc(&self, encoding: &str) -> FontDesc {
        self.fonts
            .get(encoding)
            .unwrap_or(&FontDesc::default())
            .clone()
    }

    pub(super) fn construct_svg_doc(&self, encoding: &str) -> String {
        let mut svgdoc = String::new();
        write!(&mut svgdoc, r#"<svg width="{0}", height="{1}"px viewBox="0 0 {2} {3}" preserveAspectRatio="xMinYMin meet"
xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1">
        "#, self.width, self.height, self.width, self.height).unwrap(); // svg header

        let font = self.get_fontdesc(encoding);
        let mut csstyle = String::new();
        if font.style.is_empty() == false {
            write!(
                &mut csstyle,
                r#"<style type="text/css"><![CDATA[{}]]></style>"#,
                font.style
            )
            .unwrap();
        }

        write!(&mut svgdoc, "\n{}\n", csstyle).unwrap(); // css style
        write!(&mut svgdoc, r#"<defs><filter id="textinvert" x="0" y="0" width="100%" height="100%"><feFlood flood-color="\#000"/><feComposite in="SourceGraphic" operator="xor"/></filter></defs>"#).unwrap(); // invert filter

        write!(&mut svgdoc, r#"<g font-family="{0}" fill="\#000" font-size="{1}" dominant-baseline="text-after-edge" text-anchor="middle">{2}</g></svg>\n"#, font.family, font.size.0, self.svg_content.join("")).unwrap();
        svgdoc
    }

    pub fn draw_text(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        layout: &mut TextLayout,
    ) -> (usize, usize) {
        let mut temp_x = 0;

        // line links to a TextSpan, and part of the TextSpan, and the x coordinate of the part in
        // this line.
        let newline = Vec::<(&TextSpan, String, usize)>::new();

        // lines initializes with an empty one.
        let mut lines = vec![newline.clone()];

        // walk through the layout, and break it into lines
        for span in &mut layout.spans {
            let (mut char_width_px, mut char_height_px) = span.font.size;
            char_width_px = char_width_px * span.stretch_width as usize;
            char_height_px = char_height_px * span.stretch_height as usize;
            let words = WordSeparator::UnicodeBreakProperties.find_words(span.text.as_str());

            for word in words {
                if word.word.contains('\t') {
                    let mut tab_len = layout.tab_len * char_width_px;
                    while tab_len < temp_x {
                        tab_len += tab_len;
                    }
                    if tab_len < width {
                        temp_x = tab_len;
                    }
                    continue;
                }

                if word.word.contains('\r') {
                    temp_x = 0;
                    continue;
                }

                if word.word.contains('\n') {
                    lines.push(newline.clone());
                    temp_x = 0;
                    continue;
                }

                let word_len = word.word.len() + word.whitespace.len();

                if word_len * char_width_px < width - temp_x {
                    // can be filled in this line.
                    lines.last_mut().unwrap().push((
                        span,
                        format!("{}{}", word.word, word.whitespace),
                        temp_x,
                    ));
                    temp_x += word_len * char_width_px;
                } else if word_len * char_width_px > width {
                    // this text span should break into multiple lines.
                    let broken = word.break_apart(width / char_width_px);

                    for broke in broken {
                        let broke_word_len = broke.word.len() + broke.whitespace.len();
                        if width - (broke_word_len * char_width_px) < char_width_px {
                            // use a whole line to fill this fragment.
                            lines.push(newline.clone());
                            temp_x = 0;
                            lines.last_mut().unwrap().push((
                                span,
                                format!("{}{}", broke.word, broke.whitespace),
                                temp_x,
                            ));
                            lines.push(newline.clone());
                        } else {
                            // current line can hold one or more characters.
                            lines.last_mut().unwrap().push((
                                span,
                                format!("{}{}", broke.word, broke.whitespace),
                                temp_x,
                            ));
                            temp_x += broke_word_len as usize * char_width_px;
                        }
                    }
                } else {
                    // append a new line and then add word
                    lines.push(newline.clone());
                    temp_x = 0;
                    lines.last_mut().unwrap().push((
                        span,
                        format!("{}{}", word.word, word.whitespace),
                        temp_x,
                    ));
                    temp_x += word_len * char_width_px;
                }
            }
        }

        let mut new_x = x;
        let mut new_y = y;

        for line in lines.into_iter() {
            let mut line_height_mult = 1;
            let mut precalculated_width = 0;
            let mut justify = TextJustify::Left;
            let mut iter = 0;

            for word in &line {
                if iter == 0 {
                    justify = word.0.justify.clone();
                }
                precalculated_width += word.1.len() * BASE_FONT_SIZE.0;
                iter += 1;
            }

            match justify {
                TextJustify::Center => new_x = (width - precalculated_width) / 2,
                TextJustify::Right => new_x = width - precalculated_width,
                _ => {}
            }

            for word in &line {
                if word.0.stretch_height > 1 {
                    line_height_mult = word.0.stretch_height as usize;
                }
                let (w, _) = self.render_word(new_x, new_y, word.1.as_str(), word.0);
                new_x += w;
            }
            new_x = x;
            new_y += layout.line_height as usize * line_height_mult;
        }

        (new_x, new_y)
    }

    pub fn render_word(
        &mut self,
        x: usize,
        y: usize,
        text: &str,
        span: &TextSpan,
    ) -> (usize, usize) {
        todo!() // TODO: use cjk nano
    }
}
