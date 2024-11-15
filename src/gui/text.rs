use crate::{
    graphics::texture::{OrientedSection, TEXTURE_IMAGES},
    shared::bounding_box::{bbox, BBox2},
};

use super::{
    color::GuiColor,
    element::{GuiContext, GuiElement, GuiPrimitive},
    transform::GuiTransform,
};
use cgmath::{vec2, ElementWise, Vector2};
use codepage_437::CP437_WINGDINGS;
use image::{DynamicImage, GenericImageView};
use lazy_static::lazy_static;

pub const FONT_CHARS_PER_ROW: u32 = 16;
pub const FONT_PIXELS_PER_CHAR: u32 = 8;
pub const FONT_CHAR_PIXEL_PORTION: f32 = 1.0 / (FONT_PIXELS_PER_CHAR as f32);

#[derive(Debug, Clone, Copy)]
pub struct CharData {
    pub width: f32,
    pub offset: f32,
    pub uv: BBox2,
}

pub fn generate_char_data(atlas: &DynamicImage) -> [CharData; 256] {
    std::array::from_fn(|index| {
        let index = index as u32;
        let top_left =
            vec2(index % FONT_CHARS_PER_ROW, index / FONT_CHARS_PER_ROW) * FONT_PIXELS_PER_CHAR;

        let image_size = vec2(atlas.width() as f32, atlas.height() as f32);

        let mut pixel_offset: Option<u32> = None;
        let mut pixel_width: Option<u32> = None;

        for x_offset in 0..FONT_PIXELS_PER_CHAR {
            for y_offset in 0..FONT_PIXELS_PER_CHAR {
                let color = atlas
                    .get_pixel(top_left.x + x_offset, top_left.y + y_offset)
                    .0;
                if color[3] > 0 {
                    if pixel_offset.is_none() {
                        pixel_offset = Some(x_offset);
                    }
                    pixel_width = Some(x_offset + 1 - pixel_offset.unwrap());
                    break;
                }
            }
        }

        const TINY_MARGIN: Vector2<f32> = vec2(0.00001, 0.00001);

        let uv_top_left =
            top_left.cast::<f32>().unwrap().div_element_wise(image_size) + TINY_MARGIN;
        let uv_bottom_right = uv_top_left
            + vec2(FONT_PIXELS_PER_CHAR as f32, FONT_PIXELS_PER_CHAR as f32)
                .div_element_wise(image_size)
            - TINY_MARGIN * 2.0;

        let uv = bbox!(uv_top_left, uv_bottom_right);

        // the text cursor for TextBoxes is a character with zero width
        // actually, it has a width of -1 pixels to cancel out the margin
        // might be a little too hacky but whatever
        if index == 0 {
            return CharData {
                width: -FONT_CHAR_PIXEL_PORTION,
                offset: FONT_CHAR_PIXEL_PORTION,
                uv,
            };
        }

        CharData {
            width: pixel_width.unwrap_or(0) as f32 * FONT_CHAR_PIXEL_PORTION,
            offset: pixel_offset.unwrap_or(0) as f32 * FONT_CHAR_PIXEL_PORTION,
            uv,
        }
    })
}

lazy_static! {
    pub static ref FONT_CHAR_DATA: [CharData; 256] =
        generate_char_data(TEXTURE_IMAGES.get("font").unwrap());
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyling {
    pub text_color: GuiColor,
    pub drop_shadow_color: GuiColor,
    pub bold: bool,
}

impl Default for TextStyling {
    fn default() -> Self {
        Self {
            text_color: GuiColor::WHITE,
            drop_shadow_color: GuiColor::INVISIBLE,
            bold: false,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct StyledText {
    pub raw_text: String,
    pub sections: Vec<((usize, usize), TextStyling)>,
}

impl std::fmt::Display for StyledText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &((start, end), styling) in self.sections.iter() {
            use color_eyre::owo_colors::{DynColors, OwoColorize};
            let color = styling.text_color * 255.0;
            let shadow_color = styling.drop_shadow_color * 255.0;
            let raw_text_slice = &self.raw_text[start..end];
            let text =
                raw_text_slice.color(DynColors::Rgb(color.r as u8, color.g as u8, color.b as u8));
            let shadow_dyn_color = DynColors::Rgb(
                shadow_color.r as u8,
                shadow_color.g as u8,
                shadow_color.b as u8,
            );
            if styling.bold {
                if shadow_color.is_visible() {
                    write!(f, "{}", text.bold().on_color(shadow_dyn_color))?;
                } else {
                    write!(f, "{}", text.bold())?;
                }
            } else if shadow_color.is_visible() {
                write!(f, "{}", text.on_color(shadow_dyn_color))?;
            } else {
                write!(f, "{}", text)?;
            }
        }
        Ok(())
    }
}

impl StyledText {
    pub fn single_section(text: &str, styling: TextStyling) -> Self {
        Self {
            raw_text: text.to_owned(),
            sections: vec![((0, text.len()), styling)],
        }
    }

    pub fn from_format_string(text: &str) -> Self {
        const FORMAT_CHAR: char = '§';
        const NEGATE_CHAR: char = '!';

        let mut sections = Vec::<((usize, usize), TextStyling)>::new();
        let mut current_section: Option<(usize, usize)> = None;
        let mut current_styling = TextStyling::default();

        let mut format_expected = false;
        let mut negated = false;
        'char_loop: for (byte_index, character) in text.char_indices() {
            let next_byte_index = byte_index + character.len_utf8();
            let at_end = next_byte_index >= text.len();
            if format_expected {
                let mut is_valid = true;
                let old_styling = current_styling;
                match (character, negated) {
                    (NEGATE_CHAR, false) if !at_end => {
                        negated = true;
                        continue 'char_loop;
                    }
                    ('0'..='9' | 'a'..='f', false) => {
                        current_styling.text_color = match character {
                            '0' => GuiColor::BLACK,
                            '1' => GuiColor::DARK_BLUE,
                            '2' => GuiColor::DARK_GREEN,
                            '3' => GuiColor::DARK_AQUA,
                            '4' => GuiColor::DARK_RED,
                            '5' => GuiColor::DARK_PURPLE,
                            '6' => GuiColor::GOLD,
                            '7' => GuiColor::GRAY,
                            '8' => GuiColor::DARK_GRAY,
                            '9' => GuiColor::BLUE,
                            'a' => GuiColor::GREEN,
                            'b' => GuiColor::AQUA,
                            'c' => GuiColor::RED,
                            'd' => GuiColor::LIGHT_PURPLE,
                            'e' => GuiColor::YELLOW,
                            'f' => GuiColor::WHITE,
                            _ => unreachable!(),
                        };

                        if current_styling.drop_shadow_color.is_visible() {
                            current_styling.drop_shadow_color = current_styling.text_color.shadow();
                        }
                    }
                    // reset
                    ('r', false) => {
                        current_styling = TextStyling::default();
                    }
                    // drop shadow
                    ('k', negated) => {
                        current_styling.drop_shadow_color = if !negated {
                            current_styling.text_color.shadow()
                        } else {
                            GuiColor::INVISIBLE
                        }
                    }
                    // bold
                    ('l', negated) => {
                        current_styling.bold = !negated;
                    }
                    _ => {
                        is_valid = false;
                    }
                }

                negated = false;
                format_expected = false;

                if is_valid {
                    if let Some(range) = current_section.take() {
                        sections.push((range, old_styling));
                    }

                    continue 'char_loop;
                }
            } else if character == FORMAT_CHAR && !at_end {
                format_expected = true;
                continue 'char_loop;
            }

            current_section = Some(match current_section {
                Some((start, _)) => (start, next_byte_index),
                None => (byte_index, next_byte_index),
            });

            if at_end {
                sections.push((current_section.unwrap(), current_styling));
            }
        }

        Self {
            raw_text: text.to_owned(),
            sections,
        }
    }

    pub fn extend(&mut self, other: &StyledText) {
        let index_offset = self.raw_text.len();
        self.raw_text.push_str(&other.raw_text);
        self.sections.reserve(other.sections.len());
        for &((start, end), styling) in other.sections.iter() {
            self.sections
                .push(((start + index_offset, end + index_offset), styling));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderChar {
    pub ibm_code: u8,
    pub offset: f32,
    pub styling: TextStyling,
}

#[derive(Debug, Clone)]
pub struct RenderLine {
    pub chars: Vec<RenderChar>,
    pub total_width: f32,
}

impl Default for RenderLine {
    fn default() -> Self {
        Self {
            chars: Vec::with_capacity(32),
            total_width: Default::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TextRenderData {
    pub lines: Vec<RenderLine>,
}

impl TextRenderData {
    pub fn generate(text: &StyledText, max_line_width: f32) -> Self {
        let char_spacing = FONT_CHAR_PIXEL_PORTION;
        let space_spacing = 0.5;

        let max_line_width = max_line_width.max(1.0 + char_spacing + FONT_CHAR_PIXEL_PORTION);

        let mut lines = Vec::<RenderLine>::new();

        let mut current_line = RenderLine::default();
        let mut last_whitespace_offset = 0.0;
        let mut current_word = Vec::<RenderChar>::new();
        let mut current_word_width = 0.0;

        let sections = text
            .sections
            .iter()
            .filter(|section| section.0 .0 != section.0 .1);
        let section_count = sections.clone().count();

        for (section_index, ((slice_start, slice_end), styling)) in sections.copied().enumerate() {
            let mut char_iter = text.raw_text[slice_start..slice_end].chars().peekable();
            while let Some(character) = char_iter.next() {
                let is_end = (section_index == section_count - 1) && (char_iter.peek().is_none());

                let ibm_code = CP437_WINGDINGS.encode(character).unwrap_or(b'?');
                let char_data = FONT_CHAR_DATA[ibm_code as usize];

                let is_newline = character == '\n';
                let is_space = character == ' ';
                let is_whitespace = is_newline || is_space;

                macro_rules! finish_line {
                    () => {
                        lines.push(current_line);
                        current_line = RenderLine::default();
                        last_whitespace_offset = 0.0;
                    };
                }

                macro_rules! finish_word {
                    () => {
                        let line_width_after =
                            current_line.total_width + current_word_width + last_whitespace_offset;

                        if line_width_after > max_line_width {
                            finish_line!();
                        }

                        for render_char in current_word.iter_mut() {
                            render_char.offset += current_line.total_width + last_whitespace_offset;
                        }
                        current_line.chars.append(&mut current_word); // this empties current_word
                        current_line.total_width += current_word_width + last_whitespace_offset;

                        current_word_width = 0.0;
                    };
                }

                if !is_whitespace {
                    let added_width = char_data.width
                        + char_spacing
                        + if styling.bold {
                            FONT_CHAR_PIXEL_PORTION
                        } else {
                            0.0
                        };

                    if current_word_width + added_width > max_line_width {
                        finish_word!();
                        last_whitespace_offset = 0.0;
                    }

                    current_word.push(RenderChar {
                        ibm_code,
                        offset: current_word_width - char_data.offset,
                        styling,
                    });
                    current_word_width += added_width;
                };

                if is_whitespace || is_end {
                    finish_word!();

                    if is_newline {
                        finish_line!();
                    } else if is_space {
                        last_whitespace_offset = space_spacing;
                    }

                    if is_end {
                        finish_line!();
                    }
                }
            }
        }

        Self { lines }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum TextBackgroundType {
    #[default]
    /// The background matches [`transform`](TextLabel::transform) with no resizing.
    Full,
    /// The background takes the shape of the smallest single rectangle that contains the entire text.
    BoundingBox,
    /// Each line gets a seperate smallest rectangle that contains it.
    BoundingBoxPerLine,
    /// [`TextBackgroundType::Full`] with a texture.
    TexturedFull(OrientedSection),
    /// [`TextBackgroundType::BoundingBox`] with a texture.
    TexturedBoundingBox(OrientedSection),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextLabel {
    /// The positioning and sizing of the label.
    pub transform: GuiTransform,
    /// The text to display.
    pub text: StyledText,
    /// The height, in pixels, of a single character.
    pub char_pixel_height: f32,
    /// The alignment of the text. e.g. `(0.0, 0.5)` aligns to the middle left.
    ///
    /// You can use the [`TextLabel::ALIGN_*`] constants for more readability.
    pub text_alignment: Vector2<f32>,
    /// The color of the background.
    pub background_color: GuiColor,
    /// The behavior of the background.
    pub background_type: TextBackgroundType,
}

impl Default for TextLabel {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            text: Default::default(),
            char_pixel_height: 14.0,
            text_alignment: Self::ALIGN_TOP_LEFT,
            background_color: GuiColor::INVISIBLE,
            background_type: Default::default(),
        }
    }
}

impl TextLabel {
    pub const ALIGN_TOP_LEFT: Vector2<f32> = vec2(0.0, 0.0);
    pub const ALIGN_TOP_CENTER: Vector2<f32> = vec2(0.5, 0.0);
    pub const ALIGN_TOP_RIGHT: Vector2<f32> = vec2(1.0, 0.0);

    pub const ALIGN_MIDDLE_LEFT: Vector2<f32> = vec2(0.0, 0.5);
    pub const ALIGN_MIDDLE_CENTER: Vector2<f32> = vec2(0.5, 0.5);
    pub const ALIGN_MIDDLE_RIGHT: Vector2<f32> = vec2(1.0, 0.5);

    pub const ALIGN_BOTTOM_LEFT: Vector2<f32> = vec2(0.0, 1.0);
    pub const ALIGN_BOTTOM_CENTER: Vector2<f32> = vec2(0.5, 1.0);
    pub const ALIGN_BOTTOM_RIGHT: Vector2<f32> = vec2(1.0, 1.0);

    const LINE_HEIGHT: f32 = 1.0 + FONT_CHAR_PIXEL_PORTION * 2.0;

    pub fn get_max_char_pixel_height(container_height: f32, lines: u32) -> f32 {
        container_height / (lines.max(1) as f32 * Self::LINE_HEIGHT + FONT_CHAR_PIXEL_PORTION)
    }
}

impl GuiElement for TextLabel {
    fn transform(&self) -> GuiTransform {
        self.transform
    }

    fn render(&self, context: &mut GuiContext) -> Vec<GuiPrimitive> {
        let GuiContext {
            texture_provider,
            frame,
            ..
        } = context;
        let frame = *frame;

        let char_pixel_height = self.char_pixel_height.max(1.0);

        let mut primitives = Vec::<GuiPrimitive>::with_capacity(64);

        let (absolute_position, absolute_size) = self.transform.absolute(frame);
        let absolute_top_left = absolute_position
            + vec2(char_pixel_height, char_pixel_height) * FONT_CHAR_PIXEL_PORTION;
        let bounds = (absolute_size / char_pixel_height)
            - vec2(FONT_CHAR_PIXEL_PORTION, FONT_CHAR_PIXEL_PORTION);
        let max_lines = (bounds.y / Self::LINE_HEIGHT + 0.01) as usize;
        let render_data = TextRenderData::generate(&self.text, bounds.x);

        let line_count = render_data.lines.len().min(max_lines);
        let total_height = Self::LINE_HEIGHT * line_count as f32;
        let lines_start_y = (bounds.y - total_height) * self.text_alignment.y;

        let font_texture_section = texture_provider.get_section("font");
        let white_texture_section = context.white();

        // background
        let mut bounding_box_per_line = false;
        if self.background_color.is_visible() {
            match self.background_type {
                TextBackgroundType::Full | TextBackgroundType::TexturedFull(..) => {
                    let section =
                        if let TextBackgroundType::TexturedFull(section) = self.background_type {
                            section
                        } else {
                            white_texture_section
                        };
                    primitives.push(GuiPrimitive {
                        absolute_position,
                        absolute_size,
                        section,
                        color: self.background_color,
                    });
                }
                TextBackgroundType::BoundingBox | TextBackgroundType::TexturedBoundingBox(..) => {
                    let section = if let TextBackgroundType::TexturedBoundingBox(section) =
                        self.background_type
                    {
                        section
                    } else {
                        white_texture_section
                    };

                    let widest = render_data
                        .lines
                        .iter()
                        .take(line_count)
                        .map(|line| line.total_width)
                        .reduce(|biggest, current| biggest.max(current))
                        .unwrap_or(0.0);
                    if widest > 0.0 {
                        let widest_absolute =
                            (widest + FONT_CHAR_PIXEL_PORTION) * char_pixel_height;
                        primitives.push(GuiPrimitive {
                            absolute_position: vec2(
                                (bounds.x - widest) * self.text_alignment.x,
                                lines_start_y,
                            ) * char_pixel_height,
                            absolute_size: vec2(
                                widest_absolute,
                                (total_height - FONT_CHAR_PIXEL_PORTION) * char_pixel_height,
                            ),
                            section,
                            color: self.background_color,
                        });
                    }
                }
                TextBackgroundType::BoundingBoxPerLine => {
                    bounding_box_per_line = true;
                }
            }
        }

        for (line_index, line) in render_data.lines.iter().take(line_count).enumerate() {
            let start_x = (bounds.x - line.total_width) * self.text_alignment.x;
            let start_y = lines_start_y + Self::LINE_HEIGHT * line_index as f32;

            if bounding_box_per_line && line.total_width > 0.0 {
                primitives.push(GuiPrimitive {
                    absolute_position: absolute_top_left
                        + vec2(
                            start_x - FONT_CHAR_PIXEL_PORTION,
                            start_y - FONT_CHAR_PIXEL_PORTION,
                        ) * char_pixel_height,
                    absolute_size: vec2(
                        line.total_width + FONT_CHAR_PIXEL_PORTION,
                        Self::LINE_HEIGHT,
                    ) * char_pixel_height,
                    section: white_texture_section,
                    color: self.background_color,
                })
            }

            for render_char in line.chars.iter() {
                let char_data = FONT_CHAR_DATA[render_char.ibm_code as usize];

                let has_shadow = render_char.styling.drop_shadow_color.is_visible();
                let extra_offset = if has_shadow {
                    vec2(char_pixel_height, char_pixel_height) * -FONT_CHAR_PIXEL_PORTION / 2.0
                } else {
                    vec2(0.0, 0.0)
                };

                let base_primitive = GuiPrimitive {
                    absolute_position: absolute_top_left
                        + vec2(start_x + render_char.offset, start_y) * char_pixel_height
                        + extra_offset,
                    absolute_size: vec2(char_pixel_height, char_pixel_height),
                    section: font_texture_section.local_uv(char_data.uv),
                    color: render_char.styling.text_color,
                };

                if has_shadow {
                    let shadow_position = base_primitive.absolute_position
                        + vec2(char_pixel_height, char_pixel_height) * FONT_CHAR_PIXEL_PORTION;
                    primitives.push(GuiPrimitive {
                        absolute_position: shadow_position,
                        color: render_char.styling.drop_shadow_color,

                        ..base_primitive
                    });
                    if render_char.styling.bold {
                        primitives.push(GuiPrimitive {
                            absolute_position: shadow_position
                                + vec2(char_pixel_height * FONT_CHAR_PIXEL_PORTION, 0.0),
                            color: render_char.styling.drop_shadow_color,

                            ..base_primitive
                        });
                    }
                }

                if render_char.styling.text_color.is_visible() {
                    primitives.push(base_primitive);
                    if render_char.styling.bold {
                        primitives.push(GuiPrimitive {
                            absolute_position: base_primitive.absolute_position
                                + vec2(char_pixel_height * FONT_CHAR_PIXEL_PORTION, 0.0),

                            ..base_primitive
                        });
                    }
                }
            }
        }

        primitives
    }
}
