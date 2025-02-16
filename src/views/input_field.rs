use {
    super::*,
    crate::*,
    crossterm::{
        cursor,
        event::{
            KeyCode,
            KeyEvent,
            KeyModifiers,
        },
        queue,
        style::{
            Attribute,
            Color,
            SetBackgroundColor,
        },
    },
    std::io::Write,
};

/// A simple input field, managing its cursor position and
/// either handling the events you give it or being managed
/// through direct manipulation functions
/// (put_char, del_char_left, etc.).
///
/// To create a multiline input_field (otherwise called a
/// textarea) you should set an area with a height of more
/// than 1 and allow newline to be created on keyboard with
/// `new_line_on`.
pub struct InputField {
    content: InputFieldContent,
    area: Area,
    focused_style: CompoundStyle,
    unfocused_style: CompoundStyle,
    cursor_style: CompoundStyle,
    /// when true, the display will have stars instead of the normal chars
    pub password_mode: bool,
    /// if not focused, the content will be displayed as text
    focused: bool,
    scroll: Pos,
    new_line_keys: Vec<KeyEvent>,
}

impl Default for InputField {
    fn default() -> Self {
        Self::new(Area::uninitialized())
    }
}

macro_rules! wrap_content_fun {
    ($fun:ident) => {
        pub fn $fun(&mut self) -> bool {
            if self.content.$fun() {
                self.fix_scroll();
                true
            } else {
                false
            }
        }
    };
}

impl InputField {

    pub const ENTER: KeyEvent = KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
    };
    pub const ALT_ENTER: KeyEvent = KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::ALT,
    };

    pub fn new(area: Area) -> Self {
        let focused_style = CompoundStyle::default();
        let unfocused_style = CompoundStyle::default();
        let mut cursor_style = focused_style.clone();
        cursor_style.add_attr(Attribute::Reverse);
        Self {
            content: InputFieldContent::default(),
            area,
            focused_style,
            unfocused_style,
            cursor_style,
            password_mode: false,
            focused: true,
            scroll: Pos::default(),
            new_line_keys: Vec::default(),
        }
    }
    pub fn set_mono_line(&mut self) {
        self.new_line_keys.clear();
    }
    /// define a key which will be interpreted as a new line.
    ///
    /// You may define several ones. If you set none, the input
    /// field will stay monoline unless you manage key events
    /// yourself to insert new lines.
    ///
    /// Beware that keys like Ctrl-Enter and Shift-Enter
    /// are usually received by TUI applications as simple Enter.
    ///
    /// Example:
    /// ```
    /// use termimad::*;
    /// let mut textarea = InputField::new(Area::new(5, 5, 20, 10));
    /// textarea.new_line_on(InputField::ALT_ENTER);
    /// ```
    pub fn new_line_on(&mut self, key: KeyEvent) {
        self.new_line_keys.push(key);
    }
    /// Change the area x, y and width, but not the height.
    ///
    /// Makes most sense for monoline inputs
    pub fn change_area(&mut self, x: u16, y: u16, w: u16) {
        self.area.left = x;
        self.area.top = y;
        self.area.width = w;
        self.fix_scroll();
    }
    pub fn set_area(&mut self, area: Area) {
        if &self.area != &area {
            self.area = area;
            self.fix_scroll();
        }
    }
    pub const fn area(&self) -> &Area {
        &self.area
    }
    /// return the current scrolling state on both axis
    pub const fn scroll(&self) -> Pos {
        self.scroll
    }
    /// Tell the input to be or not focused
    pub fn set_focus(&mut self, b: bool) {
        self.focused = b;
        // there's no reason to change the scroll when unfocusing
        if self.focused {
            self.fix_scroll();
        }
    }
    pub const fn focused(&self) -> bool {
        self.focused
    }
    pub fn set_normal_style(&mut self, style: CompoundStyle) {
        self.focused_style = style;
        self.cursor_style = self.focused_style.clone();
        self.cursor_style.add_attr(Attribute::Reverse);
    }
    pub fn set_unfocused_style(&mut self, style: CompoundStyle) {
        self.unfocused_style = style;
    }
    pub const fn content(&self) -> &InputFieldContent {
        &self.content
    }
    pub fn get_content(&self) -> String {
        self.content.to_string()
    }
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    /// tell whether the content of the input is equal
    ///  to the argument
    pub fn is_content(&self, s: &str) -> bool {
        self.content.is_str(s)
    }
    /// change the content to the new one and
    ///  put the cursor at the end **if** the
    ///  content is different from the previous one.
    pub fn set_str<S: AsRef<str>>(&mut self, s: S) {
        self.content.set_str(s);
        self.fix_scroll();
    }
    pub fn insert_new_line(&mut self) -> bool {
        self.content.insert_new_line();
        self.fix_scroll();
        true
    }
    /// put a char at cursor position (and increment this
    /// position).
    pub fn put_char(&mut self, c: char) -> bool {
        self.content.insert_char(c);
        self.fix_scroll();
        true
    }
    pub fn clear(&mut self) {
        self.content.clear();
        self.fix_scroll();
    }
    /// remove the char at cursor position, if any
    pub fn del_char_below(&mut self) -> bool {
        self.content.del_char_below()
    }
    /// Insert the string on cursor point, as if it was typed
    pub fn insert_str<S: AsRef<str>>(&mut self, s: S) {
        self.content.insert_str(s);
        self.fix_scroll();
    }

    wrap_content_fun!(move_up);
    wrap_content_fun!(move_down);
    wrap_content_fun!(move_left);
    wrap_content_fun!(move_right);
    wrap_content_fun!(move_to_start);
    wrap_content_fun!(move_to_end);
    wrap_content_fun!(move_to_line_start);
    wrap_content_fun!(move_to_line_end);
    wrap_content_fun!(move_word_left);
    wrap_content_fun!(move_word_right);
    wrap_content_fun!(del_char_left);
    wrap_content_fun!(del_word_left);
    wrap_content_fun!(del_word_right);

    pub fn page_up(&mut self) -> bool {
        if self.content.move_lines_up(self.area.height as usize) {
            self.fix_scroll();
            true
        } else {
            false
        }
    }

    pub fn page_down(&mut self) -> bool {
        if self.content.move_lines_down(self.area.height as usize) {
            self.fix_scroll();
            true
        } else {
            false
        }
    }

    /// apply an event being a key
    ///
    ///
    /// This function handles a few events like deleting a
    /// char, or going to the start (home key) or end (end key)
    /// of the input. If you want to totally handle events, you
    /// may call function like `put_char` and `del_char_left`
    /// directly.
    pub fn apply_key_event(&mut self, key: KeyEvent) -> bool {
        if !self.focused {
            return false;
        }
        if self.new_line_keys.contains(&key) {
            self.insert_new_line();
            return true;
        }
        use crossterm::event::{
            KeyModifiers as Mod,
        };
        match (key.code, key.modifiers) {
            (code, Mod::NONE) | (code, Mod::SHIFT) => self.apply_keycode_event(code),
            _ => false,
        }
    }

    /// apply an event being a key without modifier.
    ///
    /// You don't usually call this function but the more
    /// general `apply_event`. This one is useful when you
    /// manage events mostly yourselves.
    pub fn apply_keycode_event(&mut self, code: KeyCode) -> bool {
        if !self.focused {
            return false;
        }
        match code {
            KeyCode::Home => self.move_to_line_start(),
            KeyCode::End => self.move_to_line_end(),
            KeyCode::Char(c) => self.put_char(c),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Left => self.move_left(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Right => self.move_right(),
            KeyCode::Backspace => self.del_char_left(),
            KeyCode::Delete => self.del_char_below(),
            _ => false,
        }
    }

    /// Apply a click event
    pub fn apply_click_event(&mut self, x: u16, y: u16) -> bool {
        if self.area.contains(x, y) {
            if self.focused {
                self.content.set_cursor_pos(Pos {
                    x: (x - self.area.left) as usize + self.scroll.x,
                    y: (y - self.area.top) as usize + self.scroll.y,
                });
            } else {
                self.focused = true;
            }
            true
        } else {
            false
        }
    }

    /// apply the passed event to change the state (content, cursor)
    ///
    /// Return true when the event was used.
    pub fn apply_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Click(x, y, ..) => {
                self.apply_click_event(*x, *y)
            }
            Event::Key(KeyEvent{code, modifiers})
                if (modifiers.is_empty()||*modifiers==KeyModifiers::SHIFT)
            => {
                self.apply_keycode_event(*code)
            }
            _ => false,
        }
    }

    fn fix_scroll(&mut self) {
        let mut width = self.area.width as usize;
        let height = self.area.height as usize;
        let lines = &self.content.lines();
        let has_y_scroll = lines.len() > height;
        if has_y_scroll {
            width -= 1;
        } else {
            self.scroll.y = 0;
        }
        let pos = self.content.cursor_pos();

        if has_y_scroll {
            if self.scroll.y + height > lines.len() {
                self.scroll.y = lines.len() - height;
            }
            if self.focused {
                // we must ensure the cursor is visible
                if self.scroll.y > pos.y {
                    self.scroll.y = pos.y;
                    if self.scroll.y > 0 && height > 4 {
                        self.scroll.y -= 1;
                    }
                } else if pos.y >= self.scroll.y + height {
                    self.scroll.y = pos.y - height + 1;
                    if pos.y + 1 < lines.len() {
                        self.scroll.y -= 1;
                    }
                }
            }
        }

        let line_len = self.content.current_line().chars.len();
        if line_len < width {
            self.scroll.x = 0;
        } else {
            if self.focused {
                // we don't show ellipsis if the width is below 4
                // so we need less margin
                if width < 4 {
                    if pos.x < 2 {
                        self.scroll.x = 0;
                    } else if pos.x < self.scroll.x + 1 {
                        self.scroll.x = pos.x - 1;
                    } else if pos.x > self.scroll.x + width {
                        self.scroll.x = pos.x + 1 - width;
                    }
                } else {
                    if pos.x < self.scroll.x + 2 {
                        if pos.x < 2 {
                            self.scroll.x = 0;
                        } else {
                            self.scroll.x = pos.x - 2;
                        }
                    } else if pos.x > self.scroll.x + width - 2 {
                        self.scroll.x = pos.x + 2 - width;
                    }
                }
            }
            if self.scroll.x + width > line_len + 1 {
                self.scroll.x = line_len + 1 - width;
            }
        }
    }

    /// Render the input field on screen.
    ///
    /// All rendering must be explicitely called, no rendering is
    /// done on functions changing the state.
    ///
    /// w is typically either stderr or stdout. This function doesn't
    /// flush by itself (useful to avoid flickering)
    pub fn display_on<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        let normal_style = if self.focused {
            &self.focused_style
        } else {
            &self.unfocused_style
        };

        let mut width = self.area.width as usize;
        let pos = self.content.cursor_pos();
        let scrollbar = self.area.scrollbar(
            self.scroll.y as u16,
            self.content.line_count() as u16,
        );
        if scrollbar.is_some() {
            width -= 1;
        }

        queue!(w, SetBackgroundColor(Color::Reset))?;
        let mut scrollbar_style = &crate::get_default_skin().scrollbar;
        let mut focused_scrollbar_style;
        if self.focused {
            if let Some(bg) = self.focused_style.get_bg() {
                focused_scrollbar_style = scrollbar_style.clone();
                focused_scrollbar_style.set_bg(bg);
                scrollbar_style = &focused_scrollbar_style;
            }
        }

        let mut numbered_lines = self.content.lines().iter()
            .map(|line| &line.chars)
            .enumerate()
            .skip(self.scroll.y);

        for j in 0..self.area.height {
            queue!(w, cursor::MoveTo(self.area.left, j + self.area.top))?;
            if let Some((y, chars)) = numbered_lines.next() {
                // we don't show ellipsis if the width is below 4
                let ellipsis_at_start = self.scroll.x > 0 && width > 4;
                let cursor_at_end = self.focused && y == pos.y && pos.x == chars.len();
                let ellipsis_at_end = !cursor_at_end
                    && chars.len() > self.scroll.x + width
                    && width > 4;
                for i in 0..width {
                    if i == 0 && ellipsis_at_start && chars.len() > 0 {
                        normal_style.queue(w, fit::ELLIPSIS)?;
                        continue;
                    }
                    if i == width-1 && ellipsis_at_end {
                        normal_style.queue(w, fit::ELLIPSIS)?;
                        continue;
                    }
                    let idx = i + self.scroll.x;
                    if idx >= chars.len() {
                        if cursor_at_end && idx == chars.len() {
                            self.cursor_style.queue(w, ' ')?;
                        } else {
                            normal_style.queue(w, ' ')?;
                        }
                    } else {
                        let c = if self.password_mode {
                            '*'
                        } else {
                            chars[idx]
                        };
                        if self.focused && pos.x == idx && pos.y == y {
                            self.cursor_style.queue(w, c)?;
                        } else {
                            normal_style.queue(w, c)?;
                        }
                    }
                }
            } else {
                SPACE_FILLING.queue_styled(w, &normal_style, width)?;
            }
            if let Some((sctop, scbottom)) = scrollbar {
                let y = j + self.area.top;
                if sctop <= y && y <= scbottom {
                    scrollbar_style.thumb.queue(w)?;
                } else {
                    scrollbar_style.track.queue(w)?;
                }
            }
        }
        Ok(())
    }

    /// render the input field on stdout
    pub fn display(&self) -> Result<(), Error> {
        let mut w = std::io::stdout();
        self.display_on(&mut w)?;
        w.flush()?;
        Ok(())
    }
}

