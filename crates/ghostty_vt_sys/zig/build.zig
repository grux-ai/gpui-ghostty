const std = @import("std");

pub fn build(b: *std.Build) void {
    const optimize = b.standardOptimizeOption(.{});
    const target = b.standardTargetOptions(.{});

    const uucode_dep = b.dependency("uucode", .{
        .target = b.graph.host,
        .build_config_path = b.path("ghostty_src/build/uucode_config.zig"),
    });

    const uucode_tables = uucode_dep.namedLazyPath("tables.zig");

    const uucode_target_dep = b.dependency("uucode", .{
        .target = target,
        .optimize = optimize,
        .tables_path = uucode_tables,
        .build_config_path = b.path("ghostty_src/build/uucode_config.zig"),
    });

    const uucode_host_dep = b.dependency("uucode", .{
        .target = b.graph.host,
        .optimize = optimize,
        .tables_path = uucode_tables,
        .build_config_path = b.path("ghostty_src/build/uucode_config.zig"),
    });

    const props_exe = b.addExecutable(.{
        .name = "props-unigen",
        .root_module = b.createModule(.{
            .root_source_file = b.path("ghostty_src/unicode/props_uucode.zig"),
            .target = b.graph.host,
            .optimize = optimize,
        }),
    });
    props_exe.root_module.addImport("uucode", uucode_host_dep.module("uucode"));

    const symbols_exe = b.addExecutable(.{
        .name = "symbols-unigen",
        .root_module = b.createModule(.{
            .root_source_file = b.path("ghostty_src/unicode/symbols_uucode.zig"),
            .target = b.graph.host,
            .optimize = optimize,
        }),
    });
    symbols_exe.root_module.addImport("uucode", uucode_host_dep.module("uucode"));

    const props_run = b.addRunArtifact(props_exe);
    const symbols_run = b.addRunArtifact(symbols_exe);
    const props_output = props_run.captureStdOut();
    const symbols_output = symbols_run.captureStdOut();

    // Terminal build options required by Ghostty's terminal module
    const terminal_options = b.addOptions();
    terminal_options.addOption(@import("ghostty_src/terminal/build_options.zig").Artifact, "artifact", .lib);
    terminal_options.addOption(bool, "c_abi", false);
    terminal_options.addOption(bool, "oniguruma", false);
    terminal_options.addOption(bool, "simd", false);
    terminal_options.addOption(bool, "slow_runtime_safety", false);
    terminal_options.addOption(bool, "kitty_graphics", false);
    terminal_options.addOption(bool, "tmux_control_mode", false);

    const lib = b.addLibrary(.{
        .name = "ghostty_vt",
        .root_module = b.createModule(.{
            .root_source_file = b.path("lib.zig"),
            .target = target,
            .optimize = optimize,
        }),
        .linkage = .static,
    });
    lib.linkLibC();
    lib.root_module.addImport("uucode", uucode_target_dep.module("uucode"));
    lib.root_module.addOptions("terminal_options", terminal_options);

    const wf = b.addWriteFiles();
    const props_file = wf.addCopyFile(props_output, "unicode_tables.zig");
    const symbols_file = wf.addCopyFile(symbols_output, "symbols_tables.zig");

    lib.root_module.addImport("unicode_tables", b.createModule(.{
        .root_source_file = props_file,
    }));
    lib.root_module.addImport("symbols_tables", b.createModule(.{
        .root_source_file = symbols_file,
    }));

    const include_step = b.addInstallHeaderFile(
        b.path("../include/ghostty_vt.h"),
        "ghostty_vt.h",
    );

    const lib_install = b.addInstallLibFile(lib.getEmittedBin(), "libghostty_vt.a");
    b.getInstallStep().dependOn(&include_step.step);
    b.getInstallStep().dependOn(&lib_install.step);
}
