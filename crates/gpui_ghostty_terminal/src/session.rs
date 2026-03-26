use ghostty_vt::{CursorInfo, Error, KeyAction, KeyModifiers, PackedCell, Rgb, Terminal};

use crate::TerminalConfig;

pub struct TerminalSession {
    config: TerminalConfig,
    terminal: Terminal,
    clipboard_write: Option<String>,
    parse_tail: Vec<u8>,
}

impl TerminalSession {
    pub fn new(config: TerminalConfig) -> Result<Self, Error> {
        let mut terminal = Terminal::new(config.cols, config.rows)?;
        terminal.set_default_colors(config.default_fg, config.default_bg);
        Ok(Self {
            config,
            terminal,
            clipboard_write: None,
            parse_tail: Vec::new(),
        })
    }

    pub fn cols(&self) -> u16 {
        self.config.cols
    }

    pub fn rows(&self) -> u16 {
        self.config.rows
    }

    pub fn default_foreground(&self) -> Rgb {
        self.config.default_fg
    }

    pub fn default_background(&self) -> Rgb {
        self.config.default_bg
    }

    pub fn bracketed_paste_enabled(&self) -> bool {
        self.terminal.get_mode(2004, false)
    }

    pub fn mouse_reporting_enabled(&self) -> bool {
        self.terminal.get_mode(9, false)
            || self.terminal.get_mode(1000, false)
            || self.terminal.get_mode(1002, false)
            || self.terminal.get_mode(1003, false)
    }

    pub fn mouse_sgr_enabled(&self) -> bool {
        self.terminal.get_mode(1006, false)
    }

    pub fn mouse_button_event_enabled(&self) -> bool {
        self.terminal.get_mode(1002, false)
    }

    pub fn mouse_any_event_enabled(&self) -> bool {
        self.terminal.get_mode(1003, false)
    }

    pub fn encode_key(
        &self,
        key_name: &str,
        utf8: &str,
        modifiers: KeyModifiers,
        action: KeyAction,
    ) -> Option<Vec<u8>> {
        self.terminal.encode_key(key_name, utf8, modifiers, action)
    }

    pub fn cursor_info(&self) -> CursorInfo {
        self.terminal.cursor_info()
    }

    pub fn take_title(&mut self) -> Option<String> {
        self.terminal.take_title()
    }

    pub(crate) fn window_title_updates_enabled(&self) -> bool {
        self.config.update_window_title
    }

    pub fn hyperlink_at(&self, col: u16, row: u16) -> Option<String> {
        self.terminal.hyperlink_at(col, row)
    }

    pub fn take_clipboard_write(&mut self) -> Option<String> {
        self.clipboard_write.take()
    }

    fn scan_clipboard_write(&mut self, bytes: &[u8]) {
        const TAIL_LIMIT: usize = 2048;

        self.parse_tail.extend_from_slice(bytes);
        if self.parse_tail.len() > TAIL_LIMIT {
            let drop_len = self.parse_tail.len() - TAIL_LIMIT;
            self.parse_tail.drain(0..drop_len);
        }
        let buf = self.parse_tail.as_slice();

        let mut j = 0usize;
        while j + 1 < buf.len() {
            if buf[j] != 0x1b || buf[j + 1] != b']' {
                j += 1;
                continue;
            }

            let mut k = j + 2;
            let mut ps: u32 = 0;
            let mut saw_digit = false;
            while k < buf.len() {
                let b = buf[k];
                if b.is_ascii_digit() {
                    saw_digit = true;
                    ps = ps.saturating_mul(10).saturating_add((b - b'0') as u32);
                    k += 1;
                    continue;
                }
                if b == b';' {
                    k += 1;
                    break;
                }
                break;
            }
            if !saw_digit || k >= buf.len() {
                j += 1;
                continue;
            }

            let payload_start = k;
            while k < buf.len() {
                match buf[k] {
                    0x07 => {
                        if ps == 52 {
                            if let Some(clip) = decode_osc_52(&buf[payload_start..k]) {
                                self.clipboard_write = Some(clip);
                            }
                        }
                        k += 1;
                        break;
                    }
                    0x1b if k + 1 < buf.len() && buf[k + 1] == b'\\' => {
                        if ps == 52 {
                            if let Some(clip) = decode_osc_52(&buf[payload_start..k]) {
                                self.clipboard_write = Some(clip);
                            }
                        }
                        k += 2;
                        break;
                    }
                    _ => k += 1,
                }
            }

            j = k.max(j + 1);
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.scan_clipboard_write(bytes);
        self.terminal.feed(bytes)
    }

    pub fn feed_with_pty_responses(
        &mut self,
        bytes: &[u8],
        mut send: impl FnMut(&[u8]),
    ) -> Result<(), Error> {
        self.scan_clipboard_write(bytes);
        self.terminal.feed(bytes)?;

        if let Some(response) = self.terminal.take_response_bytes() {
            send(&response);
        }

        Ok(())
    }

    pub fn dump_viewport(&self) -> Result<String, Error> {
        self.terminal.dump_viewport()
    }

    pub fn dump_viewport_row(&self, row: u16) -> Result<String, Error> {
        self.terminal.dump_viewport_row(row)
    }

    pub fn get_row_cells(&self, row: u16) -> Result<Vec<PackedCell>, Error> {
        self.terminal.get_row_cells(row)
    }

    pub fn dump_viewport_row_cell_styles(
        &self,
        row: u16,
    ) -> Result<Vec<ghostty_vt::CellStyle>, Error> {
        self.terminal.dump_viewport_row_cell_styles(row)
    }

    pub fn dump_viewport_row_style_runs(
        &self,
        row: u16,
    ) -> Result<Vec<ghostty_vt::StyleRun>, Error> {
        self.terminal.dump_viewport_row_style_runs(row)
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        self.terminal.cursor_position()
    }

    pub fn scroll_viewport(&mut self, delta_lines: i32) -> Result<(), Error> {
        self.terminal.scroll_viewport(delta_lines)
    }

    pub fn scroll_viewport_top(&mut self) -> Result<(), Error> {
        self.terminal.scroll_viewport_top()
    }

    pub fn scroll_viewport_bottom(&mut self) -> Result<(), Error> {
        self.terminal.scroll_viewport_bottom()
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        self.config.cols = cols;
        self.config.rows = rows;
        self.terminal.resize(cols, rows)
    }

    pub(crate) fn take_dirty_viewport_rows(&mut self) -> Vec<u16> {
        self.terminal
            .take_dirty_viewport_rows(self.config.rows)
            .unwrap_or_default()
    }

    pub(crate) fn take_viewport_scroll_delta(&mut self) -> i32 {
        self.terminal.take_viewport_scroll_delta()
    }
}

fn decode_osc_52(payload: &[u8]) -> Option<String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;

    let mut split = payload.splitn(2, |b| *b == b';');
    let selection = split.next()?;
    let data = split.next()?;

    if !selection.contains(&b'c') {
        return None;
    }
    if data.is_empty() {
        return None;
    }

    let decoded = STANDARD.decode(data).ok()?;
    Some(String::from_utf8_lossy(&decoded).into_owned())
}
