use std::ffi::c_void;
use std::fmt;
use std::ptr::NonNull;

#[derive(Debug)]
pub enum Error {
    CreateFailed,
    FeedFailed(i32),
    ScrollFailed(i32),
    DumpFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CreateFailed => write!(f, "terminal create failed"),
            Error::FeedFailed(code) => write!(f, "terminal feed failed: {code}"),
            Error::ScrollFailed(code) => write!(f, "terminal scroll failed: {code}"),
            Error::DumpFailed => write!(f, "terminal dump failed"),
        }
    }
}

impl std::error::Error for Error {}

pub struct Terminal {
    ptr: NonNull<c_void>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellStyle {
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleRun {
    pub start_col: u16,
    pub end_col: u16,
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum UnderlineStyle {
    None = 0,
    Single = 1,
    Double = 2,
    Curly = 3,
    Dotted = 4,
    Dashed = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedCell {
    pub codepoint: u32,
    pub fg: Rgb,
    pub bg: Rgb,
    pub flags: u8,
    pub wide: u8,
    pub underline_style: UnderlineStyle,
    pub underline_color: Rgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CursorShape {
    BlockBlink = 0,
    BlockSteady = 1,
    UnderlineBlink = 2,
    UnderlineSteady = 3,
    BarBlink = 4,
    BarSteady = 5,
}

#[derive(Clone, Copy, Debug)]
pub struct CursorInfo {
    pub col: u16,
    pub row: u16,
    pub shape: CursorShape,
    pub visible: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

impl KeyModifiers {
    fn bits(self) -> u16 {
        let mut bits = 0u16;
        if self.shift {
            bits |= 0x0001;
        }
        if self.control {
            bits |= 0x0002;
        }
        if self.alt {
            bits |= 0x0004;
        }
        if self.super_key {
            bits |= 0x0008;
        }
        bits
    }
}

pub fn encode_key_named(name: &str, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    if name.is_empty() {
        return None;
    }

    let bytes = unsafe {
        ghostty_vt_sys::ghostty_vt_encode_key_named(name.as_ptr(), name.len(), modifiers.bits())
    };
    if bytes.ptr.is_null() || bytes.len == 0 {
        return None;
    }

    let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
    let out = slice.to_vec();
    unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
    Some(out)
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Result<Self, Error> {
        let ptr = unsafe { ghostty_vt_sys::ghostty_vt_terminal_new(cols, rows) };
        let ptr = NonNull::new(ptr).ok_or(Error::CreateFailed)?;
        Ok(Self { ptr })
    }

    pub fn set_default_colors(&mut self, fg: Rgb, bg: Rgb) {
        unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_set_default_colors(
                self.ptr.as_ptr(),
                fg.r,
                fg.g,
                fg.b,
                bg.r,
                bg.g,
                bg.b,
            )
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let rc = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_feed(self.ptr.as_ptr(), bytes.as_ptr(), bytes.len())
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(Error::FeedFailed(rc))
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        let rc =
            unsafe { ghostty_vt_sys::ghostty_vt_terminal_resize(self.ptr.as_ptr(), cols, rows) };
        if rc == 0 {
            Ok(())
        } else {
            Err(Error::ScrollFailed(rc))
        }
    }

    pub fn dump_viewport(&self) -> Result<String, Error> {
        let bytes = unsafe { ghostty_vt_sys::ghostty_vt_terminal_dump_viewport(self.ptr.as_ptr()) };
        if bytes.ptr.is_null() {
            return Err(Error::DumpFailed);
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let s = String::from_utf8_lossy(slice).into_owned();
        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Ok(s)
    }

    pub fn dump_viewport_row(&self, row: u16) -> Result<String, Error> {
        let bytes = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_dump_viewport_row(self.ptr.as_ptr(), row)
        };
        if bytes.ptr.is_null() {
            return Err(Error::DumpFailed);
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let s = String::from_utf8_lossy(slice).into_owned();
        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Ok(s)
    }

    pub fn get_row_cells(&self, row: u16) -> Result<Vec<PackedCell>, Error> {
        let bytes =
            unsafe { ghostty_vt_sys::ghostty_vt_terminal_get_row_cells(self.ptr.as_ptr(), row) };
        if bytes.ptr.is_null() {
            return Err(Error::DumpFailed);
        }
        let cell_size = std::mem::size_of::<ghostty_vt_sys::ghostty_vt_packed_cell_t>();
        if bytes.len == 0 || bytes.len % cell_size != 0 {
            unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
            return Ok(Vec::new());
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let count = bytes.len / cell_size;
        let mut out = Vec::with_capacity(count);
        for chunk in slice.chunks_exact(cell_size) {
            let raw =
                unsafe { &*(chunk.as_ptr() as *const ghostty_vt_sys::ghostty_vt_packed_cell_t) };
            out.push(PackedCell {
                codepoint: raw.codepoint,
                fg: Rgb {
                    r: raw.fg_r,
                    g: raw.fg_g,
                    b: raw.fg_b,
                },
                bg: Rgb {
                    r: raw.bg_r,
                    g: raw.bg_g,
                    b: raw.bg_b,
                },
                flags: raw.flags,
                wide: raw.wide,
                underline_style: match raw.underline_style {
                    1 => UnderlineStyle::Single,
                    2 => UnderlineStyle::Double,
                    3 => UnderlineStyle::Curly,
                    4 => UnderlineStyle::Dotted,
                    5 => UnderlineStyle::Dashed,
                    _ => UnderlineStyle::None,
                },
                underline_color: Rgb {
                    r: raw.ul_color_r,
                    g: raw.ul_color_g,
                    b: raw.ul_color_b,
                },
            });
        }

        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Ok(out)
    }

    pub fn dump_viewport_row_cell_styles(&self, row: u16) -> Result<Vec<CellStyle>, Error> {
        let bytes = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_dump_viewport_row_cell_styles(
                self.ptr.as_ptr(),
                row,
            )
        };
        if bytes.ptr.is_null() {
            return Err(Error::DumpFailed);
        }
        if bytes.len == 0 {
            unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
            return Ok(Vec::new());
        }
        if bytes.len % 8 != 0 {
            unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
            return Err(Error::DumpFailed);
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let mut out = Vec::with_capacity(bytes.len / 8);
        for chunk in slice.chunks_exact(8) {
            out.push(CellStyle {
                fg: Rgb {
                    r: chunk[0],
                    g: chunk[1],
                    b: chunk[2],
                },
                bg: Rgb {
                    r: chunk[3],
                    g: chunk[4],
                    b: chunk[5],
                },
                flags: chunk[6],
            });
        }

        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Ok(out)
    }

    pub fn dump_viewport_row_style_runs(&self, row: u16) -> Result<Vec<StyleRun>, Error> {
        let bytes = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_dump_viewport_row_style_runs(self.ptr.as_ptr(), row)
        };
        if bytes.ptr.is_null() {
            return Err(Error::DumpFailed);
        }
        if bytes.len == 0 {
            unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
            return Ok(Vec::new());
        }
        if bytes.len % 12 != 0 {
            unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
            return Err(Error::DumpFailed);
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let mut out = Vec::with_capacity(bytes.len / 12);
        for chunk in slice.chunks_exact(12) {
            out.push(StyleRun {
                start_col: u16::from_ne_bytes([chunk[0], chunk[1]]),
                end_col: u16::from_ne_bytes([chunk[2], chunk[3]]),
                fg: Rgb {
                    r: chunk[4],
                    g: chunk[5],
                    b: chunk[6],
                },
                bg: Rgb {
                    r: chunk[7],
                    g: chunk[8],
                    b: chunk[9],
                },
                flags: chunk[10],
            });
        }

        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Ok(out)
    }

    pub fn take_dirty_viewport_rows(&mut self, rows: u16) -> Result<Vec<u16>, Error> {
        let bytes = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_take_dirty_viewport_rows(self.ptr.as_ptr(), rows)
        };
        if bytes.ptr.is_null() || bytes.len == 0 {
            return Ok(Vec::new());
        }
        if bytes.len % 2 != 0 {
            unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
            return Err(Error::DumpFailed);
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let mut out = Vec::with_capacity(bytes.len / 2);
        for chunk in slice.chunks_exact(2) {
            out.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Ok(out)
    }

    pub fn take_viewport_scroll_delta(&mut self) -> i32 {
        unsafe { ghostty_vt_sys::ghostty_vt_terminal_take_viewport_scroll_delta(self.ptr.as_ptr()) }
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        let mut col: u16 = 0;
        let mut row: u16 = 0;
        let ok = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_cursor_position(
                self.ptr.as_ptr(),
                &mut col as *mut u16,
                &mut row as *mut u16,
            )
        };
        ok.then_some((col, row))
    }

    pub fn hyperlink_at(&self, col: u16, row: u16) -> Option<String> {
        let bytes = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_hyperlink_at(self.ptr.as_ptr(), col, row)
        };
        if bytes.ptr.is_null() || bytes.len == 0 {
            return None;
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let s = String::from_utf8_lossy(slice).into_owned();
        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Some(s)
    }

    pub fn get_mode(&self, mode_value: u16, is_ansi: bool) -> bool {
        unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_get_mode(self.ptr.as_ptr(), mode_value, is_ansi)
        }
    }

    pub fn cursor_info(&self) -> CursorInfo {
        let raw = unsafe { ghostty_vt_sys::ghostty_vt_terminal_cursor_info(self.ptr.as_ptr()) };
        CursorInfo {
            col: raw.col,
            row: raw.row,
            shape: match raw.style {
                0 => CursorShape::BlockBlink,
                1 => CursorShape::BlockSteady,
                2 => CursorShape::UnderlineBlink,
                3 => CursorShape::UnderlineSteady,
                4 => CursorShape::BarBlink,
                5 => CursorShape::BarSteady,
                _ => CursorShape::BarBlink,
            },
            visible: raw.visible != 0,
        }
    }

    pub fn take_title(&mut self) -> Option<String> {
        let bytes = unsafe { ghostty_vt_sys::ghostty_vt_terminal_take_title(self.ptr.as_ptr()) };
        if bytes.ptr.is_null() || bytes.len == 0 {
            return None;
        }
        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let s = String::from_utf8_lossy(slice).into_owned();
        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Some(s)
    }

    pub fn take_response_bytes(&mut self) -> Option<Vec<u8>> {
        let bytes =
            unsafe { ghostty_vt_sys::ghostty_vt_terminal_take_response_bytes(self.ptr.as_ptr()) };
        if bytes.ptr.is_null() || bytes.len == 0 {
            return None;
        }
        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len) };
        let out = slice.to_vec();
        unsafe { ghostty_vt_sys::ghostty_vt_bytes_free(bytes) };
        Some(out)
    }

    pub fn scroll_viewport(&mut self, delta_lines: i32) -> Result<(), Error> {
        let rc = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_scroll_viewport(self.ptr.as_ptr(), delta_lines)
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(Error::ScrollFailed(rc))
        }
    }

    pub fn scroll_viewport_top(&mut self) -> Result<(), Error> {
        let rc =
            unsafe { ghostty_vt_sys::ghostty_vt_terminal_scroll_viewport_top(self.ptr.as_ptr()) };
        if rc == 0 {
            Ok(())
        } else {
            Err(Error::ScrollFailed(rc))
        }
    }

    pub fn scroll_viewport_bottom(&mut self) -> Result<(), Error> {
        let rc = unsafe {
            ghostty_vt_sys::ghostty_vt_terminal_scroll_viewport_bottom(self.ptr.as_ptr())
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(Error::ScrollFailed(rc))
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe { ghostty_vt_sys::ghostty_vt_terminal_free(self.ptr.as_ptr()) }
    }
}

pub fn terminal_new(cols: u16, rows: u16) -> Result<Terminal, Error> {
    Terminal::new(cols, rows)
}
