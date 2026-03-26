const std = @import("std");
const ghostty_input = @import("ghostty_src/input.zig");
const terminal = @import("ghostty_src/terminal/main.zig");

const Allocator = std.mem.Allocator;
const Action = terminal.StreamAction;

const CursorStyle = enum(u8) {
    block_blink = 0,
    block_steady = 1,
    underline_blink = 2,
    underline_steady = 3,
    bar_blink = 4,
    bar_steady = 5,
};

const TerminalHandle = struct {
    alloc: Allocator,
    terminal: terminal.Terminal,
    stream: terminal.Stream(*Handler),
    handler: Handler,
    default_fg: terminal.color.RGB,
    default_bg: terminal.color.RGB,
    viewport_top_y_screen: u32,
    has_viewport_top_y_screen: bool,
    response_buf: std.array_list.AlignedManaged(u8, null),
    cursor_style: CursorStyle,
    default_cursor: bool,
    title_buf: std.array_list.AlignedManaged(u8, null),
    has_title: bool,

    fn init(alloc: Allocator, cols: u16, rows: u16) !*TerminalHandle {
        const handle = try alloc.create(TerminalHandle);
        errdefer alloc.destroy(handle);

        const t = try terminal.Terminal.init(alloc, .{
            .cols = cols,
            .rows = rows,
        });
        errdefer {
            var tmp = t;
            tmp.deinit(alloc);
        }

        handle.* = .{
            .alloc = alloc,
            .terminal = t,
            .handler = .{ .terminal = undefined, .handle = undefined },
            .stream = undefined,
            .default_fg = .{ .r = 0xFF, .g = 0xFF, .b = 0xFF },
            .default_bg = .{ .r = 0x00, .g = 0x00, .b = 0x00 },
            .viewport_top_y_screen = 0,
            .has_viewport_top_y_screen = true,
            .response_buf = std.array_list.AlignedManaged(u8, null).init(alloc),
            .cursor_style = .bar_blink,
            .default_cursor = true,
            .title_buf = std.array_list.AlignedManaged(u8, null).init(alloc),
            .has_title = false,
        };
        handle.handler.terminal = &handle.terminal;
        handle.handler.handle = handle;
        handle.stream = terminal.Stream(*Handler).initAlloc(alloc, &handle.handler);
        return handle;
    }

    fn deinit(self: *TerminalHandle) void {
        self.title_buf.deinit();
        self.response_buf.deinit();
        self.stream.deinit();
        self.terminal.deinit(self.alloc);
        self.alloc.destroy(self);
    }
};

const Handler = struct {
    terminal: *terminal.Terminal,
    handle: *TerminalHandle,

    pub fn deinit(_: *Handler) void {}

    fn writeResponse(self: *Handler, bytes: []const u8) void {
        self.handle.response_buf.appendSlice(bytes) catch {};
    }

    fn writeCursorPositionReport(self: *Handler) void {
        const t = self.terminal;
        const y = t.screens.active.cursor.y;
        const x = t.screens.active.cursor.x;
        const row = if (t.modes.get(.origin))
            y -| t.scrolling_region.top + 1
        else
            y + 1;
        const col = if (t.modes.get(.origin))
            x -| t.scrolling_region.left + 1
        else
            x + 1;
        var buf: [32]u8 = undefined;
        const slice = std.fmt.bufPrint(&buf, "\x1b[{d};{d}R", .{ row, col }) catch return;
        self.writeResponse(slice);
    }

    fn writeColorReport(self: *Handler, ps: u8, rgb: terminal.color.RGB) void {
        const r16: u32 = @as(u32, rgb.r) * 0x0101;
        const g16: u32 = @as(u32, rgb.g) * 0x0101;
        const b16: u32 = @as(u32, rgb.b) * 0x0101;
        var buf: [64]u8 = undefined;
        const slice = std.fmt.bufPrint(&buf, "\x1b]{d};rgb:{x:0>4}/{x:0>4}/{x:0>4}\x1b\\", .{ ps, r16, g16, b16 }) catch return;
        self.writeResponse(slice);
    }

    fn setMode(self: *Handler, mode: anytype, enabled: bool) !void {
        const t = self.terminal;
        t.modes.set(mode, enabled);
        if (@intFromEnum(mode) == @intFromEnum(@as(@TypeOf(mode), .reverse_colors))) {
            t.flags.dirty.reverse_colors = true;
        } else if (@intFromEnum(mode) == @intFromEnum(@as(@TypeOf(mode), .origin))) {
            if (enabled) t.setCursorPos(1, 1);
        } else if (@intFromEnum(mode) == @intFromEnum(@as(@TypeOf(mode), .alt_screen_legacy))) {
            try t.switchScreenMode(.@"47", enabled);
        } else if (@intFromEnum(mode) == @intFromEnum(@as(@TypeOf(mode), .alt_screen))) {
            try t.switchScreenMode(.@"1047", enabled);
        } else if (@intFromEnum(mode) == @intFromEnum(@as(@TypeOf(mode), .alt_screen_save_cursor_clear_enter))) {
            try t.switchScreenMode(.@"1049", enabled);
        } else if (@intFromEnum(mode) == @intFromEnum(@as(@TypeOf(mode), .save_cursor))) {
            if (enabled) t.saveCursor() else t.restoreCursor();
        } else if (@intFromEnum(mode) == @intFromEnum(@as(@TypeOf(mode), .enable_left_and_right_margin))) {
            if (!enabled) {
                const cols: usize = @intCast(t.cols);
                t.setLeftAndRightMargin(0, cols);
            }
        }
    }

    pub fn vt(
        self: *Handler,
        comptime action: Action.Tag,
        value: Action.Value(action),
    ) !void {
        const t = self.terminal;
        switch (action) {
            .print => try t.print(value.cp),
            .print_repeat => try t.printRepeat(value),
            .bell => {},
            .backspace => t.backspace(),
            .horizontal_tab => {
                for (0..@as(usize, value)) |_| {
                    t.horizontalTab();
                }
            },
            .horizontal_tab_back => {
                for (0..@as(usize, value)) |_| {
                    t.horizontalTabBack();
                }
            },
            .linefeed => try t.linefeed(),
            .carriage_return => t.carriageReturn(),
            .set_attribute => try t.setAttribute(value),
            .invoke_charset => t.invokeCharset(value.bank, value.charset, value.locking),
            .configure_charset => t.configureCharset(value.slot, value.charset),
            .cursor_left => t.cursorLeft(value.value),
            .cursor_right => t.cursorRight(value.value),
            .cursor_down => t.cursorDown(value.value),
            .cursor_up => t.cursorUp(value.value),
            .cursor_col => t.setCursorPos(t.screens.active.cursor.y + 1, value.value),
            .cursor_row => t.setCursorPos(value.value, t.screens.active.cursor.x + 1),
            .cursor_col_relative => t.setCursorPos(t.screens.active.cursor.y + 1, t.screens.active.cursor.x + 1 + value.value),
            .cursor_row_relative => t.setCursorPos(t.screens.active.cursor.y + 1 + value.value, t.screens.active.cursor.x + 1),
            .cursor_pos => t.setCursorPos(value.row, value.col),
            .cursor_style => {
                const h = self.handle;
                if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .default))) {
                    h.default_cursor = true;
                    h.cursor_style = .bar_blink;
                } else if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .blinking_block))) {
                    h.default_cursor = false;
                    h.cursor_style = .block_blink;
                } else if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .steady_block))) {
                    h.default_cursor = false;
                    h.cursor_style = .block_steady;
                } else if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .blinking_underline))) {
                    h.default_cursor = false;
                    h.cursor_style = .underline_blink;
                } else if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .steady_underline))) {
                    h.default_cursor = false;
                    h.cursor_style = .underline_steady;
                } else if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .blinking_bar))) {
                    h.default_cursor = false;
                    h.cursor_style = .bar_blink;
                } else if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .steady_bar))) {
                    h.default_cursor = false;
                    h.cursor_style = .bar_steady;
                }
            },
            .save_cursor => t.saveCursor(),
            .restore_cursor => t.restoreCursor(),
            .erase_display_below => t.eraseDisplay(.below, value),
            .erase_display_above => t.eraseDisplay(.above, value),
            .erase_display_complete => {
                t.scrollViewport(.{ .bottom = {} });
                t.eraseDisplay(.complete, value);
            },
            .erase_display_scrollback => t.eraseDisplay(.scrollback, value),
            .erase_display_scroll_complete => t.eraseDisplay(.scroll_complete, value),
            .erase_line_right => t.eraseLine(.right, value),
            .erase_line_left => t.eraseLine(.left, value),
            .erase_line_complete => t.eraseLine(.complete, value),
            .erase_line_right_unless_pending_wrap => t.eraseLine(.right_unless_pending_wrap, value),
            .delete_chars => t.deleteChars(value),
            .erase_chars => t.eraseChars(value),
            .insert_lines => t.insertLines(value),
            .insert_blanks => t.insertBlanks(value),
            .delete_lines => t.deleteLines(value),
            .scroll_up => try t.scrollUp(value),
            .scroll_down => t.scrollDown(value),
            .index => try t.index(),
            .next_line => {
                t.carriageReturn();
                try t.index();
            },
            .reverse_index => t.reverseIndex(),
            .tab_set => t.tabSet(),
            .tab_clear_current => t.tabClear(.current),
            .tab_clear_all => t.tabClear(.all),
            .tab_reset => t.tabClear(.all),
            .top_and_bottom_margin => t.setTopAndBottomMargin(value.top_left, value.bottom_right),
            .left_and_right_margin => t.setLeftAndRightMargin(value.top_left, value.bottom_right),
            .left_and_right_margin_ambiguous => {
                if (t.modes.get(.enable_left_and_right_margin)) {
                    t.setLeftAndRightMargin(0, 0);
                } else {
                    t.saveCursor();
                }
            },
            .full_reset => {
                t.fullReset();
                self.handle.cursor_style = .bar_blink;
                self.handle.default_cursor = true;
            },
            .decaln => try t.decaln(),
            .start_hyperlink => try t.screens.active.startHyperlink(value.uri, value.id),
            .end_hyperlink => t.screens.active.endHyperlink(),
            .set_mode => try self.setMode(value.mode, true),
            .reset_mode => try self.setMode(value.mode, false),
            .save_mode => t.modes.save(value.mode),
            .restore_mode => {
                const v = t.modes.restore(value.mode);
                try self.setMode(value.mode, v);
            },
            .request_mode => {},
            .request_mode_unknown => {},
            .color_operation => {
                const requests = value.requests;
                if (requests.count() == 0) return;

                var it = requests.constIterator(0);
                while (it.next()) |req| {
                    switch (req.*) {
                        .set => |set| switch (set.target) {
                            .palette => |i| {
                                t.colors.palette.current[i] = set.color;
                                t.colors.palette.mask.set(i);
                                t.flags.dirty.palette = true;
                            },
                            else => {},
                        },
                        .query => |target| switch (target) {
                            .palette => |i| {
                                const rgb = t.colors.palette.current[i];
                                self.writeColorReport(4, rgb);
                            },
                            .dynamic => |dyn| {
                                const ps: u8 = @intFromEnum(dyn);
                                const rgb = switch (dyn) {
                                    .foreground => self.handle.default_fg,
                                    .background => self.handle.default_bg,
                                    .cursor => self.handle.default_fg,
                                    else => continue,
                                };
                                self.writeColorReport(ps, rgb);
                            },
                            else => {},
                        },
                        .reset => |target| switch (target) {
                            .palette => |i| {
                                t.colors.palette.current[i] = t.colors.palette.original[i];
                                t.colors.palette.mask.unset(i);
                                t.flags.dirty.palette = true;
                            },
                            else => {},
                        },
                        .reset_palette => {
                            const mask = &t.colors.palette.mask;
                            var mask_iterator = mask.iterator(.{});
                            while (mask_iterator.next()) |idx| {
                                const i: usize = idx;
                                t.colors.palette.current[i] = t.colors.palette.original[i];
                            }
                            t.colors.palette.mask = .initEmpty();
                            t.flags.dirty.palette = true;
                        },
                        else => {},
                    }
                }
            },
            .enquiry => {},
            .device_attributes => {
                if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .primary))) {
                    self.writeResponse("\x1b[?62;22c");
                } else if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .secondary))) {
                    self.writeResponse("\x1b[>1;10;0c");
                }
            },
            .device_status => {
                if (@intFromEnum(value.request) == @intFromEnum(@as(@TypeOf(value.request), .operating_status))) {
                    self.writeResponse("\x1b[0n");
                } else if (@intFromEnum(value.request) == @intFromEnum(@as(@TypeOf(value.request), .cursor_position))) {
                    self.writeCursorPositionReport();
                }
            },
            .kitty_keyboard_query => {},
            .kitty_keyboard_push => t.screens.active.kitty_keyboard.push(value.flags),
            .kitty_keyboard_pop => t.screens.active.kitty_keyboard.pop(@intCast(value)),
            .kitty_keyboard_set => t.screens.active.kitty_keyboard.set(.set, value.flags),
            .kitty_keyboard_set_or => t.screens.active.kitty_keyboard.set(.@"or", value.flags),
            .kitty_keyboard_set_not => t.screens.active.kitty_keyboard.set(.not, value.flags),
            .dcs_hook => {},
            .dcs_put => {},
            .dcs_unhook => {},
            .apc_start => {},
            .apc_end => {},
            .apc_put => {},
            .window_title => {
                const h = self.handle;
                h.title_buf.clearRetainingCapacity();
                const title = value.title;
                if (title.len > 0) {
                    const max_len = @min(title.len, 256);
                    h.title_buf.appendSlice(title[0..max_len]) catch {};
                }
                h.has_title = true;
            },
            .clipboard_contents => {},
            .show_desktop_notification => {},
            .progress_report => {},
            .report_pwd => {},
            .semantic_prompt => try t.semanticPrompt(value),
            .mouse_shape => {},
            .modify_key_format => {
                t.flags.modify_other_keys_2 = false;
                if (@intFromEnum(value) == @intFromEnum(@as(@TypeOf(value), .other_keys_numeric))) {
                    t.flags.modify_other_keys_2 = true;
                }
            },
            .mouse_shift_capture => {
                t.flags.mouse_shift_capture = if (value) .true else .false;
            },
            .protected_mode_off => t.setProtectedMode(.off),
            .protected_mode_iso => t.setProtectedMode(.iso),
            .protected_mode_dec => t.setProtectedMode(.dec),
            .size_report => {},
            .title_push => {},
            .title_pop => {},
            .xtversion => {},
            .active_status_display => {},
            .kitty_color_report => {},
        }
    }
};

export fn ghostty_vt_terminal_new(cols: u16, rows: u16) callconv(.c) ?*anyopaque {
    const alloc = std.heap.c_allocator;
    const handle = TerminalHandle.init(alloc, cols, rows) catch return null;
    return @ptrCast(handle);
}

export fn ghostty_vt_terminal_free(terminal_ptr: ?*anyopaque) callconv(.c) void {
    if (terminal_ptr == null) return;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));
    handle.deinit();
}

export fn ghostty_vt_terminal_set_default_colors(
    terminal_ptr: ?*anyopaque,
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
) callconv(.c) void {
    if (terminal_ptr == null) return;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));
    handle.default_fg = .{ .r = fg_r, .g = fg_g, .b = fg_b };
    handle.default_bg = .{ .r = bg_r, .g = bg_g, .b = bg_b };
}

export fn ghostty_vt_terminal_feed(
    terminal_ptr: ?*anyopaque,
    bytes: [*]const u8,
    len: usize,
) callconv(.c) c_int {
    if (terminal_ptr == null) return 1;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    handle.stream.nextSlice(bytes[0..len]) catch return 2;

    return 0;
}

export fn ghostty_vt_terminal_resize(
    terminal_ptr: ?*anyopaque,
    cols: u16,
    rows: u16,
) callconv(.c) c_int {
    if (terminal_ptr == null) return 1;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    handle.terminal.resize(
        handle.alloc,
        @as(terminal.size.CellCountInt, @intCast(cols)),
        @as(terminal.size.CellCountInt, @intCast(rows)),
    ) catch return 2;
    return 0;
}

export fn ghostty_vt_terminal_scroll_viewport(
    terminal_ptr: ?*anyopaque,
    delta_lines: i32,
) callconv(.c) c_int {
    if (terminal_ptr == null) return 1;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    handle.terminal.scrollViewport(.{ .delta = @as(isize, delta_lines) });
    return 0;
}

export fn ghostty_vt_terminal_scroll_viewport_top(terminal_ptr: ?*anyopaque) callconv(.c) c_int {
    if (terminal_ptr == null) return 1;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    handle.terminal.scrollViewport(.top);
    return 0;
}

export fn ghostty_vt_terminal_scroll_viewport_bottom(terminal_ptr: ?*anyopaque) callconv(.c) c_int {
    if (terminal_ptr == null) return 1;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    handle.terminal.scrollViewport(.bottom);
    return 0;
}

export fn ghostty_vt_terminal_cursor_position(
    terminal_ptr: ?*anyopaque,
    col_out: ?*u16,
    row_out: ?*u16,
) callconv(.c) bool {
    if (terminal_ptr == null) return false;
    if (col_out == null or row_out == null) return false;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    col_out.?.* = @intCast(handle.terminal.screens.active.cursor.x + 1);
    row_out.?.* = @intCast(handle.terminal.screens.active.cursor.y + 1);
    return true;
}

export fn ghostty_vt_terminal_dump_viewport(terminal_ptr: ?*anyopaque) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const alloc = std.heap.c_allocator;
    const slice = handle.terminal.screens.active.dumpStringAlloc(alloc, .{ .viewport = .{} }) catch {
        return .{ .ptr = null, .len = 0 };
    };

    return .{ .ptr = slice.ptr, .len = slice.len };
}

export fn ghostty_vt_terminal_dump_viewport_row(
    terminal_ptr: ?*anyopaque,
    row: u16,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const pt: terminal.point.Point = .{ .viewport = .{ .x = 0, .y = row } };
    const pin = handle.terminal.screens.active.pages.pin(pt) orelse return .{ .ptr = null, .len = 0 };

    const alloc = std.heap.c_allocator;
    var aw: std.Io.Writer.Allocating = .init(alloc);
    handle.terminal.screens.active.dumpString(&aw.writer, .{
        .tl = pin,
        .br = pin,
        .unwrap = false,
    }) catch return .{ .ptr = null, .len = 0 };
    aw.writer.flush() catch return .{ .ptr = null, .len = 0 };

    const slice = aw.toOwnedSlice() catch return .{ .ptr = null, .len = 0 };
    return .{ .ptr = slice.ptr, .len = slice.len };
}

const PackedCell = extern struct {
    codepoint: u32,
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
    flags: u8,
    wide: u8,
    underline_style: u8,
    ul_color_r: u8,
    ul_color_g: u8,
    ul_color_b: u8,
};

const CellStyle = extern struct {
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
    flags: u8,
    reserved: u8,
};

export fn ghostty_vt_terminal_dump_viewport_row_cell_styles(
    terminal_ptr: ?*anyopaque,
    row: u16,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const pt: terminal.point.Point = .{ .viewport = .{ .x = 0, .y = row } };
    const pin = handle.terminal.screens.active.pages.pin(pt) orelse return .{ .ptr = null, .len = 0 };
    const cells = pin.cells(.all);

    const default_fg: terminal.color.RGB = handle.default_fg;
    const default_bg: terminal.color.RGB = handle.default_bg;
    const palette: *const terminal.color.Palette = &handle.terminal.colors.palette.current;

    const alloc = std.heap.c_allocator;
    var out = std.array_list.AlignedManaged(u8, null).init(alloc);
    errdefer out.deinit();

    out.ensureTotalCapacity(cells.len * @sizeOf(CellStyle)) catch return .{ .ptr = null, .len = 0 };

    for (cells) |*cell| {
        const s = pin.style(cell);

        var fg = s.fg(.{ .default = default_fg, .palette = palette, .bold = null });
        var bg = s.bg(cell, palette) orelse default_bg;

        var flags: u8 = 0;
        if (s.flags.inverse) flags |= 0x01;
        if (s.flags.bold) flags |= 0x02;
        if (s.flags.italic) flags |= 0x04;
        if (s.flags.underline != .none) flags |= 0x08;
        if (s.flags.faint) flags |= 0x10;
        if (s.flags.invisible) flags |= 0x20;
        if (s.flags.strikethrough) flags |= 0x40;

        if (s.flags.inverse) {
            const tmp = fg;
            fg = bg;
            bg = tmp;
        }
        if (s.flags.invisible) {
            fg = bg;
        }

        const rec = CellStyle{
            .fg_r = fg.r,
            .fg_g = fg.g,
            .fg_b = fg.b,
            .bg_r = bg.r,
            .bg_g = bg.g,
            .bg_b = bg.b,
            .flags = flags,
            .reserved = 0,
        };
        out.appendSlice(std.mem.asBytes(&rec)) catch return .{ .ptr = null, .len = 0 };
    }

    const slice = out.toOwnedSlice() catch return .{ .ptr = null, .len = 0 };
    return .{ .ptr = slice.ptr, .len = slice.len };
}

const StyleRun = extern struct {
    start_col: u16,
    end_col: u16,
    fg_r: u8,
    fg_g: u8,
    fg_b: u8,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
    flags: u8,
    reserved: u8,
};

fn resolvedStyle(
    default_fg: terminal.color.RGB,
    default_bg: terminal.color.RGB,
    palette: *const terminal.color.Palette,
    s: anytype,
) struct {
    fg: terminal.color.RGB,
    bg: terminal.color.RGB,
    flags: u8,
} {
    var flags: u8 = 0;
    if (s.flags.inverse) flags |= 0x01;
    if (s.flags.bold) flags |= 0x02;
    if (s.flags.italic) flags |= 0x04;
    if (s.flags.underline != .none) flags |= 0x08;
    if (s.flags.faint) flags |= 0x10;
    if (s.flags.invisible) flags |= 0x20;
    if (s.flags.strikethrough) flags |= 0x40;

    const fg = s.fg(.{ .default = default_fg, .palette = palette, .bold = null });
    return .{ .fg = fg, .bg = default_bg, .flags = flags };
}

export fn ghostty_vt_terminal_dump_viewport_row_style_runs(
    terminal_ptr: ?*anyopaque,
    row: u16,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const pt: terminal.point.Point = .{ .viewport = .{ .x = 0, .y = row } };
    const pin = handle.terminal.screens.active.pages.pin(pt) orelse return .{ .ptr = null, .len = 0 };
    const cells = pin.cells(.all);

    const default_fg: terminal.color.RGB = handle.default_fg;
    const default_bg: terminal.color.RGB = handle.default_bg;
    const palette: *const terminal.color.Palette = &handle.terminal.colors.palette.current;

    const alloc = std.heap.c_allocator;
    var out = std.array_list.AlignedManaged(u8, null).init(alloc);
    errdefer out.deinit();

    if (cells.len == 0) {
        const slice = out.toOwnedSlice() catch return .{ .ptr = null, .len = 0 };
        return .{ .ptr = slice.ptr, .len = slice.len };
    }

    var current_style_id = cells[0].style_id;
    var current_style = pin.style(&cells[0]);
    const defaults = resolvedStyle(default_fg, default_bg, palette, current_style);

    var current_flags = defaults.flags;
    var current_base_fg = defaults.fg;
    var current_inverse = current_style.flags.inverse;
    var current_invisible = current_style.flags.invisible;

    var current_bg = current_style.bg(&cells[0], palette) orelse default_bg;
    var current_fg = current_base_fg;
    if (current_inverse) {
        const tmp = current_fg;
        current_fg = current_bg;
        current_bg = tmp;
    }
    if (current_invisible) {
        current_fg = current_bg;
    }

    var current_resolved = .{ .fg = current_fg, .bg = current_bg, .flags = current_flags };
    var run_start: u16 = 1;

    var col_idx: usize = 1;
    while (col_idx < cells.len) : (col_idx += 1) {
        const cell = &cells[col_idx];
        if (cell.style_id != current_style_id) {
            const end_col: u16 = @intCast(col_idx);
            const rec = StyleRun{
                .start_col = run_start,
                .end_col = end_col,
                .fg_r = current_resolved.fg.r,
                .fg_g = current_resolved.fg.g,
                .fg_b = current_resolved.fg.b,
                .bg_r = current_resolved.bg.r,
                .bg_g = current_resolved.bg.g,
                .bg_b = current_resolved.bg.b,
                .flags = current_resolved.flags,
                .reserved = 0,
            };
            out.appendSlice(std.mem.asBytes(&rec)) catch return .{ .ptr = null, .len = 0 };

            current_style_id = cell.style_id;
            current_style = pin.style(cell);
            const resolved = resolvedStyle(default_fg, default_bg, palette, current_style);
            current_flags = resolved.flags;
            current_base_fg = resolved.fg;
            current_inverse = current_style.flags.inverse;
            current_invisible = current_style.flags.invisible;

            run_start = @intCast(col_idx + 1);

            const bg_cell = current_style.bg(cell, palette) orelse default_bg;
            var fg_cell = current_base_fg;
            var bg = bg_cell;
            if (current_inverse) {
                const tmp = fg_cell;
                fg_cell = bg;
                bg = tmp;
            }
            if (current_invisible) {
                fg_cell = bg;
            }

            current_resolved = .{ .fg = fg_cell, .bg = bg, .flags = current_flags };
            continue;
        }

        const bg_cell = current_style.bg(cell, palette) orelse default_bg;
        var fg_cell = current_base_fg;
        var bg = bg_cell;
        if (current_inverse) {
            const tmp = fg_cell;
            fg_cell = bg;
            bg = tmp;
        }
        if (current_invisible) {
            fg_cell = bg;
        }

        const same = fg_cell.r == current_resolved.fg.r and fg_cell.g == current_resolved.fg.g and fg_cell.b == current_resolved.fg.b and
            bg.r == current_resolved.bg.r and bg.g == current_resolved.bg.g and bg.b == current_resolved.bg.b and
            current_flags == current_resolved.flags;
        if (same) continue;

        const end_col: u16 = @intCast(col_idx);
        const rec = StyleRun{
            .start_col = run_start,
            .end_col = end_col,
            .fg_r = current_resolved.fg.r,
            .fg_g = current_resolved.fg.g,
            .fg_b = current_resolved.fg.b,
            .bg_r = current_resolved.bg.r,
            .bg_g = current_resolved.bg.g,
            .bg_b = current_resolved.bg.b,
            .flags = current_resolved.flags,
            .reserved = 0,
        };
        out.appendSlice(std.mem.asBytes(&rec)) catch return .{ .ptr = null, .len = 0 };

        run_start = @intCast(col_idx + 1);
        current_resolved = .{ .fg = fg_cell, .bg = bg, .flags = current_flags };
    }

    const last = StyleRun{
        .start_col = run_start,
        .end_col = @intCast(cells.len),
        .fg_r = current_resolved.fg.r,
        .fg_g = current_resolved.fg.g,
        .fg_b = current_resolved.fg.b,
        .bg_r = current_resolved.bg.r,
        .bg_g = current_resolved.bg.g,
        .bg_b = current_resolved.bg.b,
        .flags = current_resolved.flags,
        .reserved = 0,
    };
    out.appendSlice(std.mem.asBytes(&last)) catch return .{ .ptr = null, .len = 0 };

    const slice = out.toOwnedSlice() catch return .{ .ptr = null, .len = 0 };
    return .{ .ptr = slice.ptr, .len = slice.len };
}

export fn ghostty_vt_terminal_get_row_cells(
    terminal_ptr: ?*anyopaque,
    row: u16,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const pt: terminal.point.Point = .{ .viewport = .{ .x = 0, .y = row } };
    const pin = handle.terminal.screens.active.pages.pin(pt) orelse return .{ .ptr = null, .len = 0 };
    const cells = pin.cells(.all);

    const default_fg = handle.default_fg;
    const default_bg = handle.default_bg;
    const palette = &handle.terminal.colors.palette.current;

    const alloc = std.heap.c_allocator;
    var out = std.array_list.AlignedManaged(u8, null).init(alloc);
    errdefer out.deinit();

    out.ensureTotalCapacity(cells.len * @sizeOf(PackedCell)) catch return .{ .ptr = null, .len = 0 };

    for (cells) |*cell| {
        const s = pin.style(cell);

        var fg = s.fg(.{ .default = default_fg, .palette = palette, .bold = null });
        var bg = s.bg(cell, palette) orelse default_bg;

        var flags: u8 = 0;
        if (s.flags.inverse) flags |= 0x01;
        if (s.flags.bold) flags |= 0x02;
        if (s.flags.italic) flags |= 0x04;
        if (s.flags.underline != .none) flags |= 0x08;
        if (s.flags.faint) flags |= 0x10;
        if (s.flags.invisible) flags |= 0x20;
        if (s.flags.strikethrough) flags |= 0x40;

        if (s.flags.inverse) {
            const tmp = fg;
            fg = bg;
            bg = tmp;
        }
        if (s.flags.invisible) {
            fg = bg;
        }

        const cp: u32 = switch (cell.content_tag) {
            .codepoint, .codepoint_grapheme => @intCast(cell.content.codepoint),
            else => 0,
        };

        const wide_val: u8 = switch (cell.wide) {
            .narrow => 0,
            .wide => 1,
            .spacer_tail => 2,
            .spacer_head => 3,
        };

        const ul_style: u8 = @intFromEnum(s.flags.underline);
        const ul_color = s.underlineColor(palette);

        const pc = PackedCell{
            .codepoint = cp,
            .fg_r = fg.r,
            .fg_g = fg.g,
            .fg_b = fg.b,
            .bg_r = bg.r,
            .bg_g = bg.g,
            .bg_b = bg.b,
            .flags = flags,
            .wide = wide_val,
            .underline_style = ul_style,
            .ul_color_r = if (ul_color) |c| c.r else fg.r,
            .ul_color_g = if (ul_color) |c| c.g else fg.g,
            .ul_color_b = if (ul_color) |c| c.b else fg.b,
        };
        out.appendSlice(std.mem.asBytes(&pc)) catch return .{ .ptr = null, .len = 0 };
    }

    const slice = out.toOwnedSlice() catch return .{ .ptr = null, .len = 0 };
    return .{ .ptr = slice.ptr, .len = slice.len };
}

export fn ghostty_vt_terminal_take_dirty_viewport_rows(
    terminal_ptr: ?*anyopaque,
    rows: u16,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null or rows == 0) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const alloc = std.heap.c_allocator;

    var out = std.array_list.AlignedManaged(u8, null).init(alloc);
    errdefer out.deinit();

    const dirty = handle.terminal.flags.dirty;
    const force_full_redraw = dirty.clear or dirty.palette or dirty.reverse_colors or dirty.preedit;
    if (force_full_redraw) {
        handle.terminal.flags.dirty.clear = false;
        handle.terminal.flags.dirty.palette = false;
        handle.terminal.flags.dirty.reverse_colors = false;
        handle.terminal.flags.dirty.preedit = false;
    }

    var y: u32 = 0;
    while (y < rows) : (y += 1) {
        const pt: terminal.point.Point = .{ .viewport = .{ .x = 0, .y = y } };
        const pin = handle.terminal.screens.active.pages.pin(pt) orelse continue;
        if (!force_full_redraw and !pin.isDirty()) continue;

        const v: u16 = @intCast(y);
        out.append(@intCast(v & 0xFF)) catch return .{ .ptr = null, .len = 0 };
        out.append(@intCast((v >> 8) & 0xFF)) catch return .{ .ptr = null, .len = 0 };

        pin.rowAndCell().row.dirty = false;
    }

    const slice = out.toOwnedSlice() catch return .{ .ptr = null, .len = 0 };
    return .{ .ptr = slice.ptr, .len = slice.len };
}

fn pinScreenRow(pin: terminal.Pin) u32 {
    var y: u32 = @intCast(pin.y);
    var node_ = pin.node;
    while (node_.prev) |node| {
        y += @intCast(node.data.size.rows);
        node_ = node;
    }
    return y;
}

export fn ghostty_vt_terminal_take_viewport_scroll_delta(
    terminal_ptr: ?*anyopaque,
) callconv(.c) i32 {
    if (terminal_ptr == null) return 0;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const tl = handle.terminal.screens.active.pages.getTopLeft(.viewport);
    const current: u32 = pinScreenRow(tl);

    if (!handle.has_viewport_top_y_screen) {
        handle.viewport_top_y_screen = current;
        handle.has_viewport_top_y_screen = true;
        return 0;
    }

    const prev: u32 = handle.viewport_top_y_screen;
    handle.viewport_top_y_screen = current;

    const delta64: i64 = @as(i64, @intCast(current)) - @as(i64, @intCast(prev));
    if (delta64 > std.math.maxInt(i32)) return std.math.maxInt(i32);
    if (delta64 < std.math.minInt(i32)) return std.math.minInt(i32);
    return @intCast(delta64);
}

export fn ghostty_vt_terminal_hyperlink_at(
    terminal_ptr: ?*anyopaque,
    col: u16,
    row: u16,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null or col == 0 or row == 0) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    const x: terminal.size.CellCountInt = @intCast(col - 1);
    const y: u32 = @intCast(row - 1);
    const pt: terminal.point.Point = .{ .viewport = .{ .x = x, .y = y } };
    const pin = handle.terminal.screens.active.pages.pin(pt) orelse return .{ .ptr = null, .len = 0 };
    const rac = pin.rowAndCell();
    if (!rac.cell.hyperlink) return .{ .ptr = null, .len = 0 };

    const id = pin.node.data.lookupHyperlink(rac.cell) orelse return .{ .ptr = null, .len = 0 };
    const entry = pin.node.data.hyperlink_set.get(pin.node.data.memory, id).*;
    const uri = entry.uri.offset.ptr(pin.node.data.memory)[0..entry.uri.len];

    const alloc = std.heap.c_allocator;
    const duped = alloc.dupe(u8, uri) catch return .{ .ptr = null, .len = 0 };
    return .{ .ptr = duped.ptr, .len = duped.len };
}

export fn ghostty_vt_encode_key_named(
    name_ptr: ?[*]const u8,
    name_len: usize,
    modifiers: u16,
) callconv(.c) ghostty_vt_bytes_t {
    if (name_ptr == null or name_len == 0) return .{ .ptr = null, .len = 0 };

    const name = name_ptr.?[0..name_len];

    const key_value: ghostty_input.Key = if (std.mem.eql(u8, name, "up"))
        .arrow_up
    else if (std.mem.eql(u8, name, "down"))
        .arrow_down
    else if (std.mem.eql(u8, name, "left"))
        .arrow_left
    else if (std.mem.eql(u8, name, "right"))
        .arrow_right
    else if (std.mem.eql(u8, name, "home"))
        .home
    else if (std.mem.eql(u8, name, "end"))
        .end
    else if (std.mem.eql(u8, name, "pageup") or std.mem.eql(u8, name, "page_up") or std.mem.eql(u8, name, "page-up"))
        .page_up
    else if (std.mem.eql(u8, name, "pagedown") or std.mem.eql(u8, name, "page_down") or std.mem.eql(u8, name, "page-down"))
        .page_down
    else if (std.mem.eql(u8, name, "insert"))
        .insert
    else if (std.mem.eql(u8, name, "delete"))
        .delete
    else if (std.mem.eql(u8, name, "backspace"))
        .backspace
    else if (std.mem.eql(u8, name, "enter"))
        .enter
    else if (std.mem.eql(u8, name, "tab"))
        .tab
    else if (std.mem.eql(u8, name, "escape"))
        .escape
    else if (name.len >= 2 and name[0] == 'f')
        parse_function_key(name[1..]) orelse return .{ .ptr = null, .len = 0 }
    else
        return .{ .ptr = null, .len = 0 };

    var mods: ghostty_input.Mods = .{};
    if ((modifiers & 0x0001) != 0) mods.shift = true;
    if ((modifiers & 0x0002) != 0) mods.ctrl = true;
    if ((modifiers & 0x0004) != 0) mods.alt = true;
    if ((modifiers & 0x0008) != 0) mods.super = true;

    const event: ghostty_input.KeyEvent = .{
        .action = .press,
        .key = key_value,
        .mods = mods,
    };

    const opts: ghostty_input.key_encode.Options = .{
        .alt_esc_prefix = true,
    };

    const alloc = std.heap.c_allocator;
    var aw: std.Io.Writer.Allocating = .init(alloc);
    ghostty_input.key_encode.encode(&aw.writer, event, opts) catch return .{ .ptr = null, .len = 0 };
    aw.writer.flush() catch return .{ .ptr = null, .len = 0 };

    const slice = aw.toOwnedSlice() catch {
        return .{ .ptr = null, .len = 0 };
    };
    if (slice.len == 0) {
        alloc.free(slice);
        return .{ .ptr = null, .len = 0 };
    }
    return .{ .ptr = slice.ptr, .len = slice.len };
}

fn parse_function_key(digits: []const u8) ?ghostty_input.Key {
    if (digits.len == 1) {
        return switch (digits[0]) {
            '1' => .f1,
            '2' => .f2,
            '3' => .f3,
            '4' => .f4,
            '5' => .f5,
            '6' => .f6,
            '7' => .f7,
            '8' => .f8,
            '9' => .f9,
            else => null,
        };
    }

    if (digits.len == 2 and digits[0] == '1') {
        return switch (digits[1]) {
            '0' => .f10,
            '1' => .f11,
            '2' => .f12,
            else => null,
        };
    }

    return null;
}

const ghostty_vt_bytes_t = extern struct {
    ptr: ?[*]const u8,
    len: usize,
};

export fn ghostty_vt_terminal_get_mode(
    terminal_ptr: ?*anyopaque,
    mode_value: u16,
    is_ansi: bool,
) callconv(.c) bool {
    if (terminal_ptr == null) return false;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));
    const tag: terminal.modes.ModeTag = .{ .value = @intCast(mode_value), .ansi = is_ansi };
    const mode: terminal.modes.Mode = @enumFromInt(@as(u16, @bitCast(tag)));
    return handle.terminal.modes.get(mode);
}

const CursorInfo = extern struct {
    col: u16,
    row: u16,
    style: u8,
    visible: u8,
};

export fn ghostty_vt_terminal_cursor_info(
    terminal_ptr: ?*anyopaque,
) callconv(.c) CursorInfo {
    const empty: CursorInfo = .{ .col = 0, .row = 0, .style = 0, .visible = 1 };
    if (terminal_ptr == null) return empty;
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));
    const t = &handle.terminal;
    return .{
        .col = @intCast(t.screens.active.cursor.x + 1),
        .row = @intCast(t.screens.active.cursor.y + 1),
        .style = @intFromEnum(handle.cursor_style),
        .visible = @intFromBool(t.modes.get(.cursor_visible)),
    };
}

export fn ghostty_vt_terminal_take_title(
    terminal_ptr: ?*anyopaque,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));
    if (!handle.has_title) return .{ .ptr = null, .len = 0 };
    handle.has_title = false;

    if (handle.title_buf.items.len == 0) return .{ .ptr = null, .len = 0 };

    const alloc = std.heap.c_allocator;
    const duped = alloc.dupe(u8, handle.title_buf.items) catch return .{ .ptr = null, .len = 0 };
    return .{ .ptr = duped.ptr, .len = duped.len };
}

export fn ghostty_vt_terminal_take_response_bytes(
    terminal_ptr: ?*anyopaque,
) callconv(.c) ghostty_vt_bytes_t {
    if (terminal_ptr == null) return .{ .ptr = null, .len = 0 };
    const handle: *TerminalHandle = @ptrCast(@alignCast(terminal_ptr.?));

    if (handle.response_buf.items.len == 0) return .{ .ptr = null, .len = 0 };

    const alloc = std.heap.c_allocator;
    const duped = alloc.dupe(u8, handle.response_buf.items) catch return .{ .ptr = null, .len = 0 };
    handle.response_buf.clearRetainingCapacity();
    return .{ .ptr = duped.ptr, .len = duped.len };
}

export fn ghostty_vt_bytes_free(bytes: ghostty_vt_bytes_t) callconv(.c) void {
    if (bytes.ptr == null or bytes.len == 0) return;
    std.heap.c_allocator.free(bytes.ptr.?[0..bytes.len]);
}

export fn ghostty_simd_decode_utf8_until_control_seq(
    input: [*]const u8,
    count: usize,
    output: [*]u32,
    output_count: *usize,
) callconv(.c) usize {
    var i: usize = 0;
    var out_i: usize = 0;
    while (i < count) {
        if (input[i] == 0x1B) break;

        const b0 = input[i];
        var cp: u32 = 0xFFFD;
        var need: usize = 1;

        if (b0 < 0x80) {
            cp = b0;
            need = 1;
        } else if (b0 & 0xE0 == 0xC0) {
            need = 2;
            if (i + need > count) break;
            const b1 = input[i + 1];
            if (b1 & 0xC0 != 0x80) {
                cp = 0xFFFD;
                need = 1;
            } else {
                cp = ((@as(u32, b0 & 0x1F)) << 6) | (@as(u32, b1 & 0x3F));
            }
        } else if (b0 & 0xF0 == 0xE0) {
            need = 3;
            if (i + need > count) break;
            const b1 = input[i + 1];
            const b2 = input[i + 2];
            if (b1 & 0xC0 != 0x80 or b2 & 0xC0 != 0x80) {
                cp = 0xFFFD;
                need = 1;
            } else {
                cp = ((@as(u32, b0 & 0x0F)) << 12) |
                    ((@as(u32, b1 & 0x3F)) << 6) |
                    (@as(u32, b2 & 0x3F));
            }
        } else if (b0 & 0xF8 == 0xF0) {
            need = 4;
            if (i + need > count) break;
            const b1 = input[i + 1];
            const b2 = input[i + 2];
            const b3 = input[i + 3];
            if (b1 & 0xC0 != 0x80 or b2 & 0xC0 != 0x80 or b3 & 0xC0 != 0x80) {
                cp = 0xFFFD;
                need = 1;
            } else {
                cp = ((@as(u32, b0 & 0x07)) << 18) |
                    ((@as(u32, b1 & 0x3F)) << 12) |
                    ((@as(u32, b2 & 0x3F)) << 6) |
                    (@as(u32, b3 & 0x3F));
            }
        } else {
            cp = 0xFFFD;
            need = 1;
        }

        output[out_i] = cp;
        out_i += 1;
        i += need;
    }

    output_count.* = out_i;
    return i;
}
