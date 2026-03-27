use std::ffi::c_void;
use std::fmt;
use std::ptr;

use ghostty_vt_sys::*;

#[derive(Debug)]
pub enum Error {
    CreateFailed,
    OutOfMemory,
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CreateFailed => write!(f, "terminal create failed"),
            Error::OutOfMemory => write!(f, "out of memory"),
            Error::InvalidValue => write!(f, "invalid value"),
        }
    }
}

impl std::error::Error for Error {}

pub fn mode_new(value: u16, ansi: bool) -> GhosttyMode {
    (value & 0x7FFF) | ((ansi as u16) << 15)
}

fn check(result: GhosttyResult) -> Result<(), Error> {
    match result {
        GHOSTTY_SUCCESS => Ok(()),
        GHOSTTY_OUT_OF_MEMORY => Err(Error::OutOfMemory),
        _ => Err(Error::InvalidValue),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<GhosttyColorRgb> for Rgb {
    fn from(c: GhosttyColorRgb) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum CursorVisualStyle {
    Bar = 0,
    Block = 1,
    Underline = 2,
    BlockHollow = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum DirtyState {
    Clean = 0,
    Partial = 1,
    Full = 2,
}

#[derive(Clone, Copy, Debug)]
pub struct CursorInfo {
    pub x: u16,
    pub y: u16,
    pub style: CursorVisualStyle,
    pub visible: bool,
    pub blinking: bool,
    pub in_viewport: bool,
}

#[derive(Clone, Debug)]
pub struct CellStyle {
    pub bold: bool,
    pub italic: bool,
    pub faint: bool,
    pub blink: bool,
    pub inverse: bool,
    pub invisible: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub underline: i32,
    pub fg_color: Option<Rgb>,
    pub bg_color: Option<Rgb>,
    pub underline_color: Option<Rgb>,
}

fn style_color_to_rgb(sc: &GhosttyStyleColor) -> Option<Rgb> {
    match sc.tag {
        GHOSTTY_STYLE_COLOR_RGB => Some(unsafe { sc.value.rgb }.into()),
        GHOSTTY_STYLE_COLOR_PALETTE => None,
        _ => None,
    }
}

pub struct Terminal {
    ptr: GhosttyTerminal,
}

unsafe impl Send for Terminal {}

impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe { ghostty_terminal_free(self.ptr) };
    }
}

impl Terminal {
    pub fn new(cols: u16, rows: u16, max_scrollback: usize) -> Result<Self, Error> {
        let opts = GhosttyTerminalOptions {
            cols,
            rows,
            max_scrollback,
        };
        let mut ptr: GhosttyTerminal = ptr::null_mut();
        check(unsafe { ghostty_terminal_new(ptr::null(), &mut ptr, opts) })?;
        if ptr.is_null() {
            return Err(Error::CreateFailed);
        }
        Ok(Self { ptr })
    }

    pub fn raw(&self) -> GhosttyTerminal {
        self.ptr
    }

    pub fn vt_write(&self, data: &[u8]) {
        unsafe { ghostty_terminal_vt_write(self.ptr, data.as_ptr(), data.len()) };
    }

    pub fn resize(
        &self,
        cols: u16,
        rows: u16,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> Result<(), Error> {
        check(unsafe {
            ghostty_terminal_resize(self.ptr, cols, rows, cell_width_px, cell_height_px)
        })
    }

    pub fn reset(&self) {
        unsafe { ghostty_terminal_reset(self.ptr) };
    }

    pub fn set_userdata(&self, userdata: *mut c_void) {
        unsafe {
            ghostty_terminal_set(
                self.ptr,
                GHOSTTY_TERMINAL_OPT_USERDATA,
                userdata as *const c_void,
            );
        }
    }

    pub fn set_write_pty_callback(&self, cb: GhosttyTerminalWritePtyFn) {
        unsafe {
            ghostty_terminal_set(
                self.ptr,
                GHOSTTY_TERMINAL_OPT_WRITE_PTY,
                cb.map(|f| f as *const c_void).unwrap_or(ptr::null()),
            );
        }
    }

    pub fn set_title_changed_callback(&self, cb: GhosttyTerminalTitleChangedFn) {
        unsafe {
            ghostty_terminal_set(
                self.ptr,
                GHOSTTY_TERMINAL_OPT_TITLE_CHANGED,
                cb.map(|f| f as *const c_void).unwrap_or(ptr::null()),
            );
        }
    }

    pub fn set_bell_callback(&self, cb: GhosttyTerminalBellFn) {
        unsafe {
            ghostty_terminal_set(
                self.ptr,
                GHOSTTY_TERMINAL_OPT_BELL,
                cb.map(|f| f as *const c_void).unwrap_or(ptr::null()),
            );
        }
    }

    pub fn set_device_attributes_callback(&self, cb: GhosttyTerminalDeviceAttributesFn) {
        unsafe {
            ghostty_terminal_set(
                self.ptr,
                GHOSTTY_TERMINAL_OPT_DEVICE_ATTRIBUTES,
                cb.map(|f| f as *const c_void).unwrap_or(ptr::null()),
            );
        }
    }

    pub fn set_size_callback(&self, cb: GhosttyTerminalSizeFn) {
        unsafe {
            ghostty_terminal_set(
                self.ptr,
                GHOSTTY_TERMINAL_OPT_SIZE,
                cb.map(|f| f as *const c_void).unwrap_or(ptr::null()),
            );
        }
    }

    pub fn scroll_viewport_delta(&self, delta: isize) {
        let mut sv = GhosttyTerminalScrollViewport {
            tag: GHOSTTY_SCROLL_VIEWPORT_DELTA,
            value: [0; 2],
        };
        sv.value[0] = delta as u64;
        unsafe { ghostty_terminal_scroll_viewport(self.ptr, sv) };
    }

    pub fn scroll_viewport_top(&self) {
        let sv = GhosttyTerminalScrollViewport {
            tag: GHOSTTY_SCROLL_VIEWPORT_TOP,
            value: [0; 2],
        };
        unsafe { ghostty_terminal_scroll_viewport(self.ptr, sv) };
    }

    pub fn scroll_viewport_bottom(&self) {
        let sv = GhosttyTerminalScrollViewport {
            tag: GHOSTTY_SCROLL_VIEWPORT_BOTTOM,
            value: [0; 2],
        };
        unsafe { ghostty_terminal_scroll_viewport(self.ptr, sv) };
    }

    pub fn get_mode(&self, mode: GhosttyMode) -> bool {
        let mut val = false;
        let _ = unsafe { ghostty_terminal_mode_get(self.ptr, mode, &mut val) };
        val
    }

    pub fn cols(&self) -> u16 {
        let mut val: u16 = 0;
        let _ = unsafe {
            ghostty_terminal_get(
                self.ptr,
                GHOSTTY_TERMINAL_DATA_COLS,
                &mut val as *mut u16 as *mut c_void,
            )
        };
        val
    }

    pub fn rows(&self) -> u16 {
        let mut val: u16 = 0;
        let _ = unsafe {
            ghostty_terminal_get(
                self.ptr,
                GHOSTTY_TERMINAL_DATA_ROWS,
                &mut val as *mut u16 as *mut c_void,
            )
        };
        val
    }

    pub fn title(&self) -> Option<String> {
        let mut s = GhosttyString {
            ptr: ptr::null(),
            len: 0,
        };
        let result = unsafe {
            ghostty_terminal_get(
                self.ptr,
                GHOSTTY_TERMINAL_DATA_TITLE,
                &mut s as *mut GhosttyString as *mut c_void,
            )
        };
        if result != GHOSTTY_SUCCESS || s.ptr.is_null() || s.len == 0 {
            return None;
        }
        let slice = unsafe { std::slice::from_raw_parts(s.ptr, s.len) };
        Some(String::from_utf8_lossy(slice).into_owned())
    }

    pub fn mouse_tracking(&self) -> bool {
        let mut val = false;
        let _ = unsafe {
            ghostty_terminal_get(
                self.ptr,
                GHOSTTY_TERMINAL_DATA_MOUSE_TRACKING,
                &mut val as *mut bool as *mut c_void,
            )
        };
        val
    }

    pub fn scrollbar(&self) -> GhosttyTerminalScrollbar {
        let mut val = GhosttyTerminalScrollbar {
            total: 0,
            offset: 0,
            len: 0,
        };
        let _ = unsafe {
            ghostty_terminal_get(
                self.ptr,
                GHOSTTY_TERMINAL_DATA_SCROLLBAR,
                &mut val as *mut GhosttyTerminalScrollbar as *mut c_void,
            )
        };
        val
    }
}

pub struct RenderState {
    state: GhosttyRenderState,
    row_iter: GhosttyRenderStateRowIterator,
    cells: GhosttyRenderStateRowCells,
}

unsafe impl Send for RenderState {}

impl Drop for RenderState {
    fn drop(&mut self) {
        unsafe {
            ghostty_render_state_row_cells_free(self.cells);
            ghostty_render_state_row_iterator_free(self.row_iter);
            ghostty_render_state_free(self.state);
        }
    }
}

impl RenderState {
    pub fn new() -> Result<Self, Error> {
        let mut state: GhosttyRenderState = ptr::null_mut();
        let mut row_iter: GhosttyRenderStateRowIterator = ptr::null_mut();
        let mut cells: GhosttyRenderStateRowCells = ptr::null_mut();
        check(unsafe { ghostty_render_state_new(ptr::null(), &mut state) })?;
        check(unsafe { ghostty_render_state_row_iterator_new(ptr::null(), &mut row_iter) })?;
        check(unsafe { ghostty_render_state_row_cells_new(ptr::null(), &mut cells) })?;
        Ok(Self {
            state,
            row_iter,
            cells,
        })
    }

    pub fn update(&self, terminal: &Terminal) -> Result<(), Error> {
        check(unsafe { ghostty_render_state_update(self.state, terminal.ptr) })
    }

    pub fn dirty(&self) -> DirtyState {
        let mut val: u32 = 0;
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_DIRTY,
                &mut val as *mut u32 as *mut c_void,
            )
        };
        match val {
            GHOSTTY_RENDER_STATE_DIRTY_PARTIAL => DirtyState::Partial,
            GHOSTTY_RENDER_STATE_DIRTY_FULL => DirtyState::Full,
            _ => DirtyState::Clean,
        }
    }

    pub fn clear_dirty(&self) {
        let val = GHOSTTY_RENDER_STATE_DIRTY_FALSE;
        let _ = unsafe {
            ghostty_render_state_set(
                self.state,
                GHOSTTY_RENDER_STATE_OPTION_DIRTY,
                &val as *const u32 as *const c_void,
            )
        };
    }

    pub fn cols(&self) -> u16 {
        let mut val: u16 = 0;
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_COLS,
                &mut val as *mut u16 as *mut c_void,
            )
        };
        val
    }

    pub fn rows(&self) -> u16 {
        let mut val: u16 = 0;
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_ROWS,
                &mut val as *mut u16 as *mut c_void,
            )
        };
        val
    }

    pub fn background(&self) -> Rgb {
        let mut val = GhosttyColorRgb { r: 0, g: 0, b: 0 };
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_COLOR_BACKGROUND,
                &mut val as *mut GhosttyColorRgb as *mut c_void,
            )
        };
        val.into()
    }

    pub fn foreground(&self) -> Rgb {
        let mut val = GhosttyColorRgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_COLOR_FOREGROUND,
                &mut val as *mut GhosttyColorRgb as *mut c_void,
            )
        };
        val.into()
    }

    pub fn cursor_info(&self) -> CursorInfo {
        let mut has_value = false;
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE,
                &mut has_value as *mut bool as *mut c_void,
            )
        };
        let mut x: u16 = 0;
        let mut y: u16 = 0;
        let mut style: u32 = 0;
        let mut visible = true;
        let mut blinking = false;
        if has_value {
            let _ = unsafe {
                ghostty_render_state_get(
                    self.state,
                    GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_X,
                    &mut x as *mut u16 as *mut c_void,
                )
            };
            let _ = unsafe {
                ghostty_render_state_get(
                    self.state,
                    GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_Y,
                    &mut y as *mut u16 as *mut c_void,
                )
            };
        }
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_CURSOR_VISUAL_STYLE,
                &mut style as *mut u32 as *mut c_void,
            )
        };
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_CURSOR_VISIBLE,
                &mut visible as *mut bool as *mut c_void,
            )
        };
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_CURSOR_BLINKING,
                &mut blinking as *mut bool as *mut c_void,
            )
        };
        CursorInfo {
            x,
            y,
            style: match style {
                GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK => CursorVisualStyle::Block,
                GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE => CursorVisualStyle::Underline,
                GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW => {
                    CursorVisualStyle::BlockHollow
                }
                _ => CursorVisualStyle::Bar,
            },
            visible,
            blinking,
            in_viewport: has_value,
        }
    }

    pub fn begin_row_iteration(&mut self) {
        let _ = unsafe {
            ghostty_render_state_get(
                self.state,
                GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR,
                &mut self.row_iter as *mut GhosttyRenderStateRowIterator as *mut c_void,
            )
        };
    }

    pub fn next_row(&self) -> bool {
        unsafe { ghostty_render_state_row_iterator_next(self.row_iter) }
    }

    pub fn row_dirty(&self) -> bool {
        let mut val = false;
        let _ = unsafe {
            ghostty_render_state_row_get(
                self.row_iter,
                GHOSTTY_RENDER_STATE_ROW_DATA_DIRTY,
                &mut val as *mut bool as *mut c_void,
            )
        };
        val
    }

    pub fn clear_row_dirty(&self) {
        let val = false;
        let _ = unsafe {
            ghostty_render_state_row_set(
                self.row_iter,
                GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY,
                &val as *const bool as *const c_void,
            )
        };
    }

    pub fn begin_cell_iteration(&mut self) {
        let _ = unsafe {
            ghostty_render_state_row_get(
                self.row_iter,
                GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                &mut self.cells as *mut GhosttyRenderStateRowCells as *mut c_void,
            )
        };
    }

    pub fn next_cell(&self) -> bool {
        unsafe { ghostty_render_state_row_cells_next(self.cells) }
    }

    pub fn cell_graphemes(&self, buf: &mut Vec<u32>) -> usize {
        let mut len: u32 = 0;
        let _ = unsafe {
            ghostty_render_state_row_cells_get(
                self.cells,
                GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
                &mut len as *mut u32 as *mut c_void,
            )
        };
        if len == 0 {
            return 0;
        }
        buf.resize(len as usize, 0);
        let _ = unsafe {
            ghostty_render_state_row_cells_get(
                self.cells,
                GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF,
                buf.as_mut_ptr() as *mut c_void,
            )
        };
        len as usize
    }

    pub fn cell_fg(&self) -> Option<Rgb> {
        let mut val = GhosttyColorRgb { r: 0, g: 0, b: 0 };
        let result = unsafe {
            ghostty_render_state_row_cells_get(
                self.cells,
                GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR,
                &mut val as *mut GhosttyColorRgb as *mut c_void,
            )
        };
        if result == GHOSTTY_SUCCESS {
            Some(val.into())
        } else {
            None
        }
    }

    pub fn cell_bg(&self) -> Option<Rgb> {
        let mut val = GhosttyColorRgb { r: 0, g: 0, b: 0 };
        let result = unsafe {
            ghostty_render_state_row_cells_get(
                self.cells,
                GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR,
                &mut val as *mut GhosttyColorRgb as *mut c_void,
            )
        };
        if result == GHOSTTY_SUCCESS {
            Some(val.into())
        } else {
            None
        }
    }

    pub fn cell_style(&self) -> CellStyle {
        let mut s = GhosttyStyle {
            size: std::mem::size_of::<GhosttyStyle>(),
            fg_color: GhosttyStyleColor {
                tag: GHOSTTY_STYLE_COLOR_NONE,
                value: GhosttyStyleColorValue { _padding: 0 },
            },
            bg_color: GhosttyStyleColor {
                tag: GHOSTTY_STYLE_COLOR_NONE,
                value: GhosttyStyleColorValue { _padding: 0 },
            },
            underline_color: GhosttyStyleColor {
                tag: GHOSTTY_STYLE_COLOR_NONE,
                value: GhosttyStyleColorValue { _padding: 0 },
            },
            bold: false,
            italic: false,
            faint: false,
            blink: false,
            inverse: false,
            invisible: false,
            strikethrough: false,
            overline: false,
            underline: 0,
        };
        let _ = unsafe {
            ghostty_render_state_row_cells_get(
                self.cells,
                GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE,
                &mut s as *mut GhosttyStyle as *mut c_void,
            )
        };
        CellStyle {
            bold: s.bold,
            italic: s.italic,
            faint: s.faint,
            blink: s.blink,
            inverse: s.inverse,
            invisible: s.invisible,
            strikethrough: s.strikethrough,
            overline: s.overline,
            underline: s.underline,
            fg_color: style_color_to_rgb(&s.fg_color),
            bg_color: style_color_to_rgb(&s.bg_color),
            underline_color: style_color_to_rgb(&s.underline_color),
        }
    }
}

pub struct KeyEncoder {
    encoder: GhosttyKeyEncoder,
    event: GhosttyKeyEvent,
}

unsafe impl Send for KeyEncoder {}

impl Drop for KeyEncoder {
    fn drop(&mut self) {
        unsafe {
            ghostty_key_event_free(self.event);
            ghostty_key_encoder_free(self.encoder);
        }
    }
}

pub const KEY_UNIDENTIFIED: i32 = 0;
pub const KEY_BACKQUOTE: i32 = 1;
pub const KEY_BACKSLASH: i32 = 2;
pub const KEY_BRACKET_LEFT: i32 = 3;
pub const KEY_BRACKET_RIGHT: i32 = 4;
pub const KEY_COMMA: i32 = 5;
pub const KEY_DIGIT_0: i32 = 6;
pub const KEY_EQUAL: i32 = 16;
pub const KEY_A: i32 = 20;
pub const KEY_MINUS: i32 = 46;
pub const KEY_PERIOD: i32 = 47;
pub const KEY_QUOTE: i32 = 48;
pub const KEY_SEMICOLON: i32 = 49;
pub const KEY_SLASH: i32 = 50;
pub const KEY_BACKSPACE: i32 = 53;
pub const KEY_ENTER: i32 = 58;
pub const KEY_SPACE: i32 = 63;
pub const KEY_TAB: i32 = 64;
pub const KEY_DELETE: i32 = 68;
pub const KEY_END: i32 = 69;
pub const KEY_HOME: i32 = 71;
pub const KEY_INSERT: i32 = 72;
pub const KEY_PAGE_DOWN: i32 = 73;
pub const KEY_PAGE_UP: i32 = 74;
pub const KEY_ARROW_DOWN: i32 = 75;
pub const KEY_ARROW_LEFT: i32 = 76;
pub const KEY_ARROW_RIGHT: i32 = 77;
pub const KEY_ARROW_UP: i32 = 78;
pub const KEY_ESCAPE: i32 = 120;
pub const KEY_F1: i32 = 121;
pub const KEY_F2: i32 = 122;
pub const KEY_F3: i32 = 123;
pub const KEY_F4: i32 = 124;
pub const KEY_F5: i32 = 125;
pub const KEY_F6: i32 = 126;
pub const KEY_F7: i32 = 127;
pub const KEY_F8: i32 = 128;
pub const KEY_F9: i32 = 129;
pub const KEY_F10: i32 = 130;
pub const KEY_F11: i32 = 131;
pub const KEY_F12: i32 = 132;

pub const KEY_ACTION_RELEASE: u32 = 0;
pub const KEY_ACTION_PRESS: u32 = 1;
pub const KEY_ACTION_REPEAT: u32 = 2;

pub const KEY_MOD_SHIFT: u16 = 0x0002;
pub const KEY_MOD_CTRL: u16 = 0x0040;
pub const KEY_MOD_ALT: u16 = 0x0004;
pub const KEY_MOD_SUPER: u16 = 0x0008;

impl KeyEncoder {
    pub fn new() -> Result<Self, Error> {
        let mut encoder: GhosttyKeyEncoder = ptr::null_mut();
        let mut event: GhosttyKeyEvent = ptr::null_mut();
        check(unsafe { ghostty_key_encoder_new(ptr::null(), &mut encoder) })?;
        check(unsafe { ghostty_key_event_new(ptr::null(), &mut event) })?;
        Ok(Self { encoder, event })
    }

    pub fn sync_from_terminal(&self, terminal: &Terminal) {
        let _ = unsafe { ghostty_key_encoder_setopt_from_terminal(self.encoder, terminal.ptr) };
    }

    pub fn encode(
        &self,
        key: i32,
        action: u32,
        mods: u16,
        utf8: &str,
        unshifted_codepoint: u32,
    ) -> Option<Vec<u8>> {
        unsafe {
            ghostty_key_event_set_key(self.event, key);
            ghostty_key_event_set_action(self.event, action);
            ghostty_key_event_set_mods(self.event, mods);
            if !utf8.is_empty() {
                ghostty_key_event_set_utf8(self.event, utf8.as_ptr(), utf8.len());
            }
            ghostty_key_event_set_unshifted_codepoint(self.event, unshifted_codepoint);
        }

        let mut buf = [0u8; 256];
        let mut written: usize = 0;
        let result = unsafe {
            ghostty_key_encoder_encode(
                self.encoder,
                self.event,
                buf.as_mut_ptr(),
                buf.len(),
                &mut written,
            )
        };
        if result != GHOSTTY_SUCCESS || written == 0 {
            return None;
        }
        Some(buf[..written].to_vec())
    }
}

pub fn map_key_name(name: &str) -> i32 {
    match name {
        "up" => KEY_ARROW_UP,
        "down" => KEY_ARROW_DOWN,
        "left" => KEY_ARROW_LEFT,
        "right" => KEY_ARROW_RIGHT,
        "home" => KEY_HOME,
        "end" => KEY_END,
        "pageup" | "page_up" | "page-up" => KEY_PAGE_UP,
        "pagedown" | "page_down" | "page-down" => KEY_PAGE_DOWN,
        "insert" => KEY_INSERT,
        "delete" => KEY_DELETE,
        "backspace" => KEY_BACKSPACE,
        "enter" => KEY_ENTER,
        "tab" => KEY_TAB,
        "escape" => KEY_ESCAPE,
        "space" => KEY_SPACE,
        "f1" => KEY_F1,
        "f2" => KEY_F2,
        "f3" => KEY_F3,
        "f4" => KEY_F4,
        "f5" => KEY_F5,
        "f6" => KEY_F6,
        "f7" => KEY_F7,
        "f8" => KEY_F8,
        "f9" => KEY_F9,
        "f10" => KEY_F10,
        "f11" => KEY_F11,
        "f12" => KEY_F12,
        _ if name.len() == 1 => {
            let c = name.as_bytes()[0];
            match c {
                b'a'..=b'z' => KEY_A + (c - b'a') as i32,
                b'0'..=b'9' => KEY_DIGIT_0 + (c - b'0') as i32,
                b'`' => KEY_BACKQUOTE,
                b'\\' => KEY_BACKSLASH,
                b'[' => KEY_BRACKET_LEFT,
                b']' => KEY_BRACKET_RIGHT,
                b',' => KEY_COMMA,
                b'.' => KEY_PERIOD,
                b'/' => KEY_SLASH,
                b';' => KEY_SEMICOLON,
                b'\'' => KEY_QUOTE,
                b'-' => KEY_MINUS,
                b'=' => KEY_EQUAL,
                _ => KEY_UNIDENTIFIED,
            }
        }
        _ => KEY_UNIDENTIFIED,
    }
}

pub fn focus_encode(focused: bool) -> Option<Vec<u8>> {
    let mut buf = [0u8; 8];
    let mut written: usize = 0;
    let result =
        unsafe { ghostty_focus_encode(focused, buf.as_mut_ptr(), buf.len(), &mut written) };
    if result != GHOSTTY_SUCCESS || written == 0 {
        return None;
    }
    Some(buf[..written].to_vec())
}
