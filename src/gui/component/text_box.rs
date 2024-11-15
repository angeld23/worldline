use super::GuiComponentId;
use crate::{
    gui::{
        color::GuiColor,
        text::{TextLabel, TextStyling},
    },
    shared::{char_indexing::CharIndexing, input::InputController},
};
use log::debug;
use std::time::{Duration, Instant};
use winit::keyboard::NamedKey;

#[derive(Debug, Clone, PartialEq)]
pub struct TextBoxDescriptor {
    /// The [`TextStyling`] for non-selected text.
    pub text_styling: TextStyling,
    /// The [`TextStyling`] for selected text.
    pub selected_text_styling: TextStyling,
    /// The maximum amount of characters that can be inputted.
    pub max_chars: u32,
    /// The default text to initialize the [`TextBox`] with.
    pub default_text: String,
    /// The default text cursor position.
    pub default_cursor_position: u32,
    /// Whether pressing the Enter key will insert a newline.
    pub allow_newlines: bool,
}

impl Default for TextBoxDescriptor {
    fn default() -> Self {
        Self {
            text_styling: Default::default(),
            selected_text_styling: TextStyling {
                text_color: GuiColor::BLUE,
                drop_shadow_color: GuiColor::DARK_BLUE,
                bold: false,
            },
            max_chars: 1024,
            default_text: String::new(),
            default_cursor_position: u32::MAX,
            allow_newlines: true,
        }
    }
}

/// Handles behavior for inputting text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    /// The current text input.
    pub current_input: String,
    /// Offset (in chars) of the text cursor.
    pub cursor_position: u32,
    /// Offset (in chars) of the selection anchor. If this is different from [`TextBox::cursor_position`],
    /// text will be selected within that range.
    pub selection_anchor: u32,

    /// The [`TextBoxDescriptor`] that was passed into [`TextBox::new()`].
    pub descriptor: TextBoxDescriptor,

    blink_start_time: Instant,
    id: GuiComponentId,
    is_focused: bool,
}

impl Default for TextBox {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl TextBox {
    const TEXT_CURSOR_BLINK_PERIOD: Duration = Duration::from_millis(1000);

    pub fn new(descriptor: TextBoxDescriptor) -> Self {
        Self {
            current_input: descriptor.default_text.to_owned(),
            cursor_position: descriptor.default_cursor_position,
            selection_anchor: descriptor.default_cursor_position,
            descriptor,

            blink_start_time: Instant::now(),
            id: Default::default(),
            is_focused: false,
        }
    }

    pub fn id(&self) -> GuiComponentId {
        self.id
    }

    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    fn selection(&self) -> (bool, u32, u32) {
        (
            self.selection_anchor != self.cursor_position,
            self.selection_anchor.min(self.cursor_position),
            self.selection_anchor.max(self.cursor_position),
        )
    }

    pub fn clear(&mut self) {
        self.current_input.clear();
        self.cursor_position = 0;
        self.selection_anchor = 0;
    }

    pub fn update(&mut self, input_controller: &InputController) {
        let is_focused = input_controller.component_is_focused(self.id);
        self.is_focused = is_focused;

        let old_cursor_position = self.cursor_position;

        let mut new_text = input_controller.just_typed().to_owned();

        if !is_focused {
            self.cursor_position = u32::MAX;
            self.selection_anchor = self.cursor_position;
        } else {
            let char_count = self.current_input.chars().count() as u32;

            let shift_held = input_controller.held(NamedKey::Shift);
            let ctrl_held = input_controller.held(NamedKey::Control);

            if ctrl_held {
                // ctrl+a
                if input_controller.pressed("a") {
                    new_text.clear();
                    self.selection_anchor = 0;
                    self.cursor_position = char_count;
                }

                let (has_selection, selection_min, selection_max) = self.selection();

                // copy
                if input_controller.pressed_or_repeated("c") {
                    if has_selection {
                        let _ = clipboard_anywhere::set_clipboard(
                            &self.current_input[self
                                .current_input
                                .char_to_byte_range_clamped(selection_min..selection_max)],
                        );
                    }

                    new_text.clear();
                }

                // cut
                if input_controller.pressed_or_repeated("x") {
                    new_text.clear();
                    if has_selection
                        && clipboard_anywhere::set_clipboard(
                            &self.current_input[self
                                .current_input
                                .char_to_byte_range_clamped(selection_min..selection_max)],
                        )
                        .is_ok()
                    {
                        new_text.push('\u{8}');
                    }
                }

                // paste
                if input_controller.pressed_or_repeated("v") {
                    new_text.clear();
                    if let Ok(text) = clipboard_anywhere::get_clipboard() {
                        new_text.push_str(&text);
                    }
                }
            }

            'char_loop: for mut character in new_text.chars() {
                let (has_selection, selection_min, selection_max) = self.selection();

                let selection_range = self
                    .current_input
                    .char_to_byte_range_clamped(selection_min..selection_max);

                macro_rules! clear_selection {
                    () => {
                        self.cursor_position = selection_min;
                        self.selection_anchor = selection_min;
                        self.current_input.replace_range(selection_range, "");
                    };
                }

                // handle control characters
                if character.is_control() {
                    match character {
                        // backspace
                        '\u{8}' => {
                            if has_selection {
                                clear_selection!();
                            } else if self.cursor_position > 0 {
                                if let Some(byte_index) = self
                                    .current_input
                                    .char_to_byte_index(self.cursor_position - 1)
                                {
                                    self.current_input.remove(byte_index);
                                    self.cursor_position -= 1;
                                    self.selection_anchor -= 1;
                                }
                            }
                            continue 'char_loop;
                        }
                        // enter
                        '\r' => {
                            if !self.descriptor.allow_newlines {
                                continue 'char_loop;
                            }
                            character = '\n';
                        }
                        '\n' => {}
                        _ => {
                            debug!("unprocessed control char: {:?}", character);
                            continue 'char_loop;
                        }
                    }
                }

                // the enter key is seperately guarded against but this handles pasting
                if !self.descriptor.allow_newlines && character == '\n' {
                    character = ' ';
                }

                if has_selection {
                    clear_selection!();
                }

                if let Some(byte_index) =
                    self.current_input.char_to_byte_index(self.cursor_position)
                {
                    self.current_input.insert(byte_index, character);
                } else {
                    self.current_input.push(character);
                }

                self.cursor_position += 1;
                self.selection_anchor = self.cursor_position;
            }

            let (has_selection, selection_min, selection_max) = self.selection();

            let char_count = self.current_input.chars().count() as u32;

            if input_controller.pressed_or_repeated(NamedKey::End) {
                self.cursor_position = self
                    .current_input
                    .chars()
                    .enumerate()
                    .skip(self.cursor_position as usize)
                    .find_map(|(i, character)| (character == '\n').then_some(i as u32))
                    .unwrap_or(char_count);

                if !shift_held {
                    self.selection_anchor = self.cursor_position;
                }
            }

            if input_controller.pressed_or_repeated(NamedKey::Home) {
                let mut newline_char_index = 0;
                for (i, character) in self
                    .current_input
                    .chars()
                    .enumerate()
                    .take(self.cursor_position as usize)
                {
                    if character == '\n' {
                        newline_char_index = i as u32 + 1;
                    }
                }
                self.cursor_position = newline_char_index;

                if !shift_held {
                    self.selection_anchor = self.cursor_position;
                }
            }

            if input_controller.pressed_or_repeated(NamedKey::ArrowLeft) {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }

                if !shift_held {
                    if has_selection {
                        self.cursor_position = selection_min;
                    }
                    self.selection_anchor = self.cursor_position;
                }
            }

            if input_controller.pressed_or_repeated(NamedKey::ArrowRight) {
                self.cursor_position += 1;
                if !shift_held {
                    if has_selection {
                        self.cursor_position = selection_max;
                    }
                    self.selection_anchor = self.cursor_position;
                }
            }
        }

        // keep the input text under max_chars
        if let Some(byte_size) = self
            .current_input
            .char_to_byte_index_open_end(self.descriptor.max_chars)
        {
            self.current_input.truncate(byte_size);
        }

        let char_count = self.current_input.chars().count() as u32;

        // keep the text cursor and selection anchor in bounds
        self.cursor_position = self.cursor_position.clamp(0, char_count);
        self.selection_anchor = self.selection_anchor.clamp(0, char_count);

        // stop the text cursor from blinking when moving it, cause otherwise it's hard to tell where it is
        if old_cursor_position != self.cursor_position {
            self.blink_start_time = Instant::now();
        }
    }

    pub fn wrap(&self, mut label: TextLabel) -> TextLabel {
        let (_, selection_min, selection_max) = self.selection();

        let TextBoxDescriptor {
            text_styling,
            selected_text_styling,
            ..
        } = self.descriptor;

        let selection_byte_range = self
            .current_input
            .char_to_byte_range_clamped(selection_min..selection_max);

        let cursor_byte_index = self
            .current_input
            .char_to_byte_index_open_end(self.cursor_position)
            .unwrap_or(0);

        self.current_input.clone_into(&mut label.text.raw_text);
        label.text.raw_text.push('\u{0}');

        let cursor_char_range = (label.text.raw_text.len() - 1, label.text.raw_text.len());
        let cursor_is_visible = self.is_focused
            && (self.blink_start_time.elapsed().as_secs_f32()
                / Self::TEXT_CURSOR_BLINK_PERIOD.as_secs_f32())
                % 1.0
                < 0.5;
        let cursor_alpha = if cursor_is_visible { 0.75 } else { 0.0 };

        let mut sections = Vec::with_capacity(4);

        if selection_byte_range.is_empty() {
            sections.push(((0, cursor_byte_index), text_styling));
            sections.push((
                cursor_char_range,
                TextStyling {
                    text_color: text_styling
                        .text_color
                        .with_alpha(text_styling.text_color.a * cursor_alpha),
                    drop_shadow_color: text_styling
                        .drop_shadow_color
                        .with_alpha(text_styling.drop_shadow_color.a * cursor_alpha),
                    ..text_styling
                },
            ));
            sections.push(((cursor_byte_index, self.current_input.len()), text_styling));
        } else {
            sections.push(((0, selection_byte_range.start), text_styling));

            let cursor = (
                cursor_char_range,
                TextStyling {
                    text_color: selected_text_styling
                        .text_color
                        .with_alpha(selected_text_styling.text_color.a * cursor_alpha),
                    drop_shadow_color: selected_text_styling
                        .drop_shadow_color
                        .with_alpha(selected_text_styling.drop_shadow_color.a * cursor_alpha),
                    ..selected_text_styling
                },
            );
            let selected_text = (
                (selection_byte_range.start, selection_byte_range.end),
                selected_text_styling,
            );

            if self.cursor_position == selection_min {
                sections.push(cursor);
                sections.push(selected_text);
            } else {
                sections.push(selected_text);
                sections.push(cursor);
            }

            sections.push((
                (selection_byte_range.end, self.current_input.len()),
                text_styling,
            ));
        }

        label.text.sections = sections;

        label
    }
}
