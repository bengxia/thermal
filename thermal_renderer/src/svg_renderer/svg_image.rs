extern crate textwrap;

use std::fmt::Write as _;
use textwrap::WordSeparator;
use thermal_parser::context;
use thermal_parser::context::{Context, TextJustify, TextStrikethrough, TextUnderline};

use super::BASE_FONT_SIZE;
use super::DEFAULT_CPL;
use super::DEFAULT_FONT_FAMILY;
use unicode_display_width::is_double_width;
use unicode_display_width::width as diswidth;

/// A simple svg renderer designed for thermal printing effects emulation
/// This allows for an image with a fixed width that can grow in height
/// to accommodate sets of svg elements.
#[allow(dead_code)]
pub struct SvgImage {
    pub width: u16, // in dots, default 576 on 80mm width(printable width 72mm, 203dpi)
    pub height: u16,
    pub svg_content: Vec<String>,
    pub base_cpl: u8, // number of characters per line, default width/12 in ANK, width/24 in CJK
    pub line_margin: u8,
    pub line_align: u8,  // 0 - left; 1 - center; 2 - right
    pub line_height: u8, // according font size
    pub feed_unit: u8,   // in dots, depends on character's height, line spacing & new line or not
    pub spacing: bool,
}

pub struct TextSpan {
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
}

impl TextSpan {
    pub fn new(text: String, context: &Context) -> Self {
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
        }
    }
}

pub struct TextLayout {
    pub spans: Vec<TextSpan>,
    pub line_height: usize,
    pub tab_len: usize,
}

impl SvgImage {
    pub fn new(width: u16) -> Self {
        Self {
            width,
            height: 0,
            svg_content: Vec::new(),
            base_cpl: DEFAULT_CPL,
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

    pub(super) fn construct_svg_doc(&self, context: &context::Context) -> String {
        let mut svgdoc = String::new();
        write!(&mut svgdoc, r#"<svg width="{0}px" height="{1}px" viewBox="0 0 {2} {3}" preserveAspectRatio="xMinYMin meet" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1">
        "#, self.width, context.graphics.y, self.width, context.graphics.y).unwrap(); // svg header

        write!(&mut svgdoc, r##"<defs><filter id="textinvert" x="0" y="0" width="100%" height="100%"><feFlood flood-color="#000"/><feComposite in="SourceGraphic" operator="xor"/></filter></defs>"##).unwrap(); // invert filter

        write!(&mut svgdoc, r##"<g font-family="{0}" fill="#000" font-size="{1}" dominant-baseline="text-after-edge" text-anchor="middle">{2}</g></svg>"##, DEFAULT_FONT_FAMILY, BASE_FONT_SIZE.1 - 2, self.svg_content.join("")).unwrap();
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
            let (mut char_width_px, _) = BASE_FONT_SIZE; // (12,24) as FontA baseline
            char_width_px = char_width_px * span.stretch_width as usize;
            let words = WordSeparator::UnicodeBreakProperties.find_words(span.text.as_str());
            // Word: {
            //     word: String,
            //     whitespace: String,
            //     penalty: String,
            //     width: usize,
            // }
            for word in words {
                // \t,\r,\n must be an individual textspan, there's no other characters.
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
                let dwidth = diswidth(word.word);
                let word_len = dwidth as usize + word.whitespace.len();

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
                    // textwrap::wrap is a better choice while the word contains CJK characters?
                    // NO! the word just one word(2 display width) for CJK.
                    let broken = word.break_apart(width / char_width_px);

                    for broke in broken {
                        let broke_word_len = diswidth(broke.word) as usize + broke.whitespace.len();
                        println!(
                            "word: {:#?}, broke: {:#?}, broke_word_len: {}, width: {}, char_width_px: {}",
                            word, broke, broke_word_len, width, char_width_px
                        );
                        if char_width_px + (broke_word_len * char_width_px) > width {
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
                            temp_x += broke_word_len * char_width_px;
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
            let mut stretch_width = 1;
            let mut stretch_height = 2;
            let mut itered = false;

            for word in &line {
                let (mut char_width_px, _) = BASE_FONT_SIZE; // (12,24) as FontA baseline
                if itered == false {
                    justify = word.0.justify.clone();
                    stretch_width = word.0.stretch_width;
                    stretch_height = word.0.stretch_height;
                    char_width_px = char_width_px * stretch_width as usize;
                }
                precalculated_width += diswidth(word.1.as_str()) as usize * char_width_px;
                itered = true;
            }

            match justify {
                TextJustify::Center => {
                    new_x = (width - precalculated_width) / (2 * stretch_width as usize)
                }
                TextJustify::Right => new_x = width - precalculated_width,
                _ => {}
            }
            self.svg_content.push(format!(
                r#"<g transform="translate({},{})"><text transform="scale({},{})">"#,
                0, new_y, stretch_width, stretch_height
            ));
            for word in &line {
                if word.0.stretch_height > 1 {
                    line_height_mult = word.0.stretch_height as usize;
                }
                let (w, _) = self.render_word(new_x, new_y, word.1.as_str(), word.0);
                new_x += w;
            }
            self.svg_content.push("</text></g>".to_string());
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
        _span: &TextSpan,
    ) -> (usize, usize) {
        println!("{x:>3}, {y:>3}: {text}");

        let (char_width_px, _) = BASE_FONT_SIZE; // (12,24) as FontA baseline
                                                 // char_width_px = char_width_px * span.stretch_width as usize;
        let mut curr_x = x;
        let mut w = 0;
        for c in text.chars() {
            let iscjk = is_double_width(c);
            self.svg_content.push(format!(
                r#"<tspan x="{}">{}</tspan>"#,
                curr_x,
                SvgImage::escape_char(c)
            ));
            let chwidth = char_width_px * if iscjk { 2 } else { 1 };
            curr_x = curr_x + chwidth;
            w = w + chwidth;
        }

        (w, 0)
    }

    fn escape_char(c: char) -> String {
        match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            ' ' => "&#xa0;".to_string(),
            _ => c.to_string(),
        }
    }
}
