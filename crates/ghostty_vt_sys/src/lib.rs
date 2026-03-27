use core::ffi::c_void;

pub type GhosttyTerminal = *mut c_void;
pub type GhosttyRenderState = *mut c_void;
pub type GhosttyRenderStateRowIterator = *mut c_void;
pub type GhosttyRenderStateRowCells = *mut c_void;
pub type GhosttyKeyEncoder = *mut c_void;
pub type GhosttyKeyEvent = *mut c_void;
pub type GhosttyMouseEncoder = *mut c_void;
pub type GhosttyMouseEvent = *mut c_void;
pub type GhosttyMode = u16;

pub type GhosttyResult = i32;
pub const GHOSTTY_SUCCESS: GhosttyResult = 0;
pub const GHOSTTY_OUT_OF_MEMORY: GhosttyResult = -1;
pub const GHOSTTY_INVALID_VALUE: GhosttyResult = -2;
pub const GHOSTTY_OUT_OF_SPACE: GhosttyResult = -3;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyString {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyColorRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyTerminalOptions {
    pub cols: u16,
    pub rows: u16,
    pub max_scrollback: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyTerminalScrollViewport {
    pub tag: u32,
    pub value: [u64; 2],
}

pub const GHOSTTY_SCROLL_VIEWPORT_TOP: u32 = 0;
pub const GHOSTTY_SCROLL_VIEWPORT_BOTTOM: u32 = 1;
pub const GHOSTTY_SCROLL_VIEWPORT_DELTA: u32 = 2;

pub const GHOSTTY_TERMINAL_OPT_USERDATA: u32 = 0;
pub const GHOSTTY_TERMINAL_OPT_WRITE_PTY: u32 = 1;
pub const GHOSTTY_TERMINAL_OPT_BELL: u32 = 2;
pub const GHOSTTY_TERMINAL_OPT_ENQUIRY: u32 = 3;
pub const GHOSTTY_TERMINAL_OPT_XTVERSION: u32 = 4;
pub const GHOSTTY_TERMINAL_OPT_TITLE_CHANGED: u32 = 5;
pub const GHOSTTY_TERMINAL_OPT_SIZE: u32 = 6;
pub const GHOSTTY_TERMINAL_OPT_COLOR_SCHEME: u32 = 7;
pub const GHOSTTY_TERMINAL_OPT_DEVICE_ATTRIBUTES: u32 = 8;
pub const GHOSTTY_TERMINAL_OPT_TITLE: u32 = 9;
pub const GHOSTTY_TERMINAL_OPT_PWD: u32 = 10;

pub const GHOSTTY_TERMINAL_DATA_COLS: u32 = 1;
pub const GHOSTTY_TERMINAL_DATA_ROWS: u32 = 2;
pub const GHOSTTY_TERMINAL_DATA_CURSOR_X: u32 = 3;
pub const GHOSTTY_TERMINAL_DATA_CURSOR_Y: u32 = 4;
pub const GHOSTTY_TERMINAL_DATA_CURSOR_VISIBLE: u32 = 7;
pub const GHOSTTY_TERMINAL_DATA_KITTY_KEYBOARD_FLAGS: u32 = 8;
pub const GHOSTTY_TERMINAL_DATA_SCROLLBAR: u32 = 9;
pub const GHOSTTY_TERMINAL_DATA_MOUSE_TRACKING: u32 = 11;
pub const GHOSTTY_TERMINAL_DATA_TITLE: u32 = 12;
pub const GHOSTTY_TERMINAL_DATA_TOTAL_ROWS: u32 = 14;
pub const GHOSTTY_TERMINAL_DATA_SCROLLBACK_ROWS: u32 = 15;

pub const GHOSTTY_RENDER_STATE_DATA_COLS: u32 = 1;
pub const GHOSTTY_RENDER_STATE_DATA_ROWS: u32 = 2;
pub const GHOSTTY_RENDER_STATE_DATA_DIRTY: u32 = 3;
pub const GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR: u32 = 4;
pub const GHOSTTY_RENDER_STATE_DATA_COLOR_BACKGROUND: u32 = 5;
pub const GHOSTTY_RENDER_STATE_DATA_COLOR_FOREGROUND: u32 = 6;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VISUAL_STYLE: u32 = 10;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VISIBLE: u32 = 11;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_BLINKING: u32 = 12;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_HAS_VALUE: u32 = 14;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_X: u32 = 15;
pub const GHOSTTY_RENDER_STATE_DATA_CURSOR_VIEWPORT_Y: u32 = 16;

pub const GHOSTTY_RENDER_STATE_OPTION_DIRTY: u32 = 0;

pub const GHOSTTY_RENDER_STATE_DIRTY_FALSE: u32 = 0;
pub const GHOSTTY_RENDER_STATE_DIRTY_PARTIAL: u32 = 1;
pub const GHOSTTY_RENDER_STATE_DIRTY_FULL: u32 = 2;

pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR: u32 = 0;
pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK: u32 = 1;
pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE: u32 = 2;
pub const GHOSTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW: u32 = 3;

pub const GHOSTTY_RENDER_STATE_ROW_DATA_DIRTY: u32 = 1;
pub const GHOSTTY_RENDER_STATE_ROW_DATA_CELLS: u32 = 3;
pub const GHOSTTY_RENDER_STATE_ROW_OPTION_DIRTY: u32 = 0;

pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE: u32 = 2;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN: u32 = 3;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_BUF: u32 = 4;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR: u32 = 5;
pub const GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR: u32 = 6;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyStyle {
    pub size: usize,
    pub fg_color: GhosttyStyleColor,
    pub bg_color: GhosttyStyleColor,
    pub underline_color: GhosttyStyleColor,
    pub bold: bool,
    pub italic: bool,
    pub faint: bool,
    pub blink: bool,
    pub inverse: bool,
    pub invisible: bool,
    pub strikethrough: bool,
    pub overline: bool,
    pub underline: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyStyleColor {
    pub tag: u32,
    pub value: GhosttyStyleColorValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union GhosttyStyleColorValue {
    pub palette: u8,
    pub rgb: GhosttyColorRgb,
    pub _padding: u64,
}

impl std::fmt::Debug for GhosttyStyleColorValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GhosttyStyleColorValue")
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyDeviceAttributesPrimary {
    pub conformance_level: u16,
    pub features: [u16; 64],
    pub num_features: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyDeviceAttributesSecondary {
    pub device_type: u16,
    pub firmware_version: u16,
    pub rom_cartridge: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyDeviceAttributesTertiary {
    pub unit_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyDeviceAttributes {
    pub primary: GhosttyDeviceAttributesPrimary,
    pub secondary: GhosttyDeviceAttributesSecondary,
    pub tertiary: GhosttyDeviceAttributesTertiary,
}

pub const GHOSTTY_STYLE_COLOR_NONE: u32 = 0;
pub const GHOSTTY_STYLE_COLOR_PALETTE: u32 = 1;
pub const GHOSTTY_STYLE_COLOR_RGB: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyTerminalScrollbar {
    pub total: u64,
    pub offset: u64,
    pub len: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GhosttyRenderStateColors {
    pub size: usize,
    pub background: GhosttyColorRgb,
    pub foreground: GhosttyColorRgb,
    pub cursor: GhosttyColorRgb,
    pub cursor_has_value: bool,
    pub palette: [GhosttyColorRgb; 256],
}

pub type GhosttyTerminalWritePtyFn = Option<
    unsafe extern "C" fn(
        terminal: GhosttyTerminal,
        userdata: *mut c_void,
        data: *const u8,
        len: usize,
    ),
>;

pub type GhosttyTerminalBellFn =
    Option<unsafe extern "C" fn(terminal: GhosttyTerminal, userdata: *mut c_void)>;

pub type GhosttyTerminalTitleChangedFn =
    Option<unsafe extern "C" fn(terminal: GhosttyTerminal, userdata: *mut c_void)>;

pub type GhosttyTerminalDeviceAttributesFn = Option<
    unsafe extern "C" fn(
        terminal: GhosttyTerminal,
        userdata: *mut c_void,
        out_attrs: *mut c_void,
    ) -> bool,
>;

pub type GhosttyTerminalSizeFn = Option<
    unsafe extern "C" fn(
        terminal: GhosttyTerminal,
        userdata: *mut c_void,
        out_size: *mut c_void,
    ) -> bool,
>;

unsafe extern "C" {
    pub fn ghostty_terminal_new(
        allocator: *const c_void,
        terminal: *mut GhosttyTerminal,
        options: GhosttyTerminalOptions,
    ) -> GhosttyResult;

    pub fn ghostty_terminal_free(terminal: GhosttyTerminal);
    pub fn ghostty_terminal_reset(terminal: GhosttyTerminal);

    pub fn ghostty_terminal_resize(
        terminal: GhosttyTerminal,
        cols: u16,
        rows: u16,
        cell_width_px: u32,
        cell_height_px: u32,
    ) -> GhosttyResult;

    pub fn ghostty_terminal_set(
        terminal: GhosttyTerminal,
        option: u32,
        value: *const c_void,
    ) -> GhosttyResult;

    pub fn ghostty_terminal_vt_write(terminal: GhosttyTerminal, data: *const u8, len: usize);

    pub fn ghostty_terminal_scroll_viewport(
        terminal: GhosttyTerminal,
        behavior: GhosttyTerminalScrollViewport,
    );

    pub fn ghostty_terminal_mode_get(
        terminal: GhosttyTerminal,
        mode: GhosttyMode,
        out_value: *mut bool,
    ) -> GhosttyResult;

    pub fn ghostty_terminal_get(
        terminal: GhosttyTerminal,
        data: u32,
        out: *mut c_void,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_new(
        allocator: *const c_void,
        state: *mut GhosttyRenderState,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_free(state: GhosttyRenderState);

    pub fn ghostty_render_state_update(
        state: GhosttyRenderState,
        terminal: GhosttyTerminal,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_get(
        state: GhosttyRenderState,
        data: u32,
        out: *mut c_void,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_set(
        state: GhosttyRenderState,
        option: u32,
        value: *const c_void,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_colors_get(
        state: GhosttyRenderState,
        out_colors: *mut GhosttyRenderStateColors,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_row_iterator_new(
        allocator: *const c_void,
        out_iterator: *mut GhosttyRenderStateRowIterator,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_row_iterator_free(iterator: GhosttyRenderStateRowIterator);

    pub fn ghostty_render_state_row_iterator_next(iterator: GhosttyRenderStateRowIterator) -> bool;

    pub fn ghostty_render_state_row_get(
        iterator: GhosttyRenderStateRowIterator,
        data: u32,
        out: *mut c_void,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_row_set(
        iterator: GhosttyRenderStateRowIterator,
        option: u32,
        value: *const c_void,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_row_cells_new(
        allocator: *const c_void,
        out_cells: *mut GhosttyRenderStateRowCells,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_row_cells_next(cells: GhosttyRenderStateRowCells) -> bool;

    pub fn ghostty_render_state_row_cells_select(
        cells: GhosttyRenderStateRowCells,
        x: u16,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_row_cells_get(
        cells: GhosttyRenderStateRowCells,
        data: u32,
        out: *mut c_void,
    ) -> GhosttyResult;

    pub fn ghostty_render_state_row_cells_free(cells: GhosttyRenderStateRowCells);

    pub fn ghostty_key_encoder_new(
        allocator: *const c_void,
        encoder: *mut GhosttyKeyEncoder,
    ) -> GhosttyResult;

    pub fn ghostty_key_encoder_free(encoder: GhosttyKeyEncoder);

    pub fn ghostty_key_encoder_setopt_from_terminal(
        encoder: GhosttyKeyEncoder,
        terminal: GhosttyTerminal,
    ) -> GhosttyResult;

    pub fn ghostty_key_encoder_encode(
        encoder: GhosttyKeyEncoder,
        event: GhosttyKeyEvent,
        buf: *mut u8,
        buf_len: usize,
        out_len: *mut usize,
    ) -> GhosttyResult;

    pub fn ghostty_key_event_new(
        allocator: *const c_void,
        event: *mut GhosttyKeyEvent,
    ) -> GhosttyResult;

    pub fn ghostty_key_event_free(event: GhosttyKeyEvent);

    pub fn ghostty_key_event_set_action(event: GhosttyKeyEvent, action: u32);
    pub fn ghostty_key_event_set_key(event: GhosttyKeyEvent, key: i32);
    pub fn ghostty_key_event_set_mods(event: GhosttyKeyEvent, mods: u16);
    pub fn ghostty_key_event_set_utf8(event: GhosttyKeyEvent, text: *const u8, len: usize);
    pub fn ghostty_key_event_set_unshifted_codepoint(event: GhosttyKeyEvent, cp: u32);

    pub fn ghostty_mouse_encoder_new(
        allocator: *const c_void,
        encoder: *mut GhosttyMouseEncoder,
    ) -> GhosttyResult;

    pub fn ghostty_mouse_encoder_free(encoder: GhosttyMouseEncoder);

    pub fn ghostty_mouse_encoder_setopt_from_terminal(
        encoder: GhosttyMouseEncoder,
        terminal: GhosttyTerminal,
    ) -> GhosttyResult;

    pub fn ghostty_mouse_encoder_encode(
        encoder: GhosttyMouseEncoder,
        event: GhosttyMouseEvent,
        buf: *mut u8,
        buf_len: usize,
        out_len: *mut usize,
    ) -> GhosttyResult;

    pub fn ghostty_mouse_event_new(
        allocator: *const c_void,
        event: *mut GhosttyMouseEvent,
    ) -> GhosttyResult;

    pub fn ghostty_mouse_event_free(event: GhosttyMouseEvent);
    pub fn ghostty_mouse_event_set_action(event: GhosttyMouseEvent, action: u32);
    pub fn ghostty_mouse_event_set_button(event: GhosttyMouseEvent, button: u32);
    pub fn ghostty_mouse_event_clear_button(event: GhosttyMouseEvent);
    pub fn ghostty_mouse_event_set_mods(event: GhosttyMouseEvent, mods: u16);
    pub fn ghostty_mouse_event_set_position(event: GhosttyMouseEvent, x: u32, y: u32);

    pub fn ghostty_focus_encode(
        focused: bool,
        buf: *mut u8,
        buf_len: usize,
        out_len: *mut usize,
    ) -> GhosttyResult;
}
