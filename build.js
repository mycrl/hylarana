const child_process = require("node:child_process");
const fs = require("node:fs");

const Args = process.argv
    .slice(process.argv.indexOf("--") + 1)
    .map((item) => item.replace("--", ""))
    .reduce(
        (args, item) =>
            Object.assign(args, {
                [item]: true,
            }),
        {}
    );

const Command = (cmd, options = {}) =>
    new Promise(
        (
            resolve,
            reject,
            ps = child_process.exec(
                process.platform == "win32"
                    ? "$ProgressPreference = 'SilentlyContinue';" + cmd
                    : cmd,
                {
                    shell: process.platform == "win32" ? "powershell.exe" : "bash",
                    cwd: __dirname,
                    ...options,
                }
            )
        ) => {
            ps.stdout.pipe(process.stdout);
            ps.stderr.pipe(process.stderr);

            ps.on("error", reject);
            ps.on("close", (code) => {
                code == 0 ? resolve() : reject(code || 0);
            });
        }
    );

const SearchBuild = (package, subdir) => {
    const Profile = Args.release ? "release" : "debug";

    for (const dir of fs.readdirSync(`./target/${Profile}/build`)) {
        const output = `./target/${Profile}/build/${dir}/out/${subdir}`;

        if (dir.startsWith(package) && fs.existsSync(output)) {
            return output;
        }
    }

    return null;
};

/* async block */
void (async () => {
    const Profile = Args.release ? "Release" : "Debug";

    for (const path of ["./target", "./build", "./build/bin"]) {
        if (!fs.existsSync(path)) {
            fs.mkdirSync(path);
        }
    }

    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-example`);
    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-server`);

    const CefOutputDir = SearchBuild("webview-sys", "cef");
    const FFmpegOutputDir = SearchBuild(
        "hylarana-ffmpeg-sys",
        "ffmpeg-n7.1-latest-win64-gpl-shared-7.1"
    );

    for (const item of [
        ["./README.md", "./build/README.md"],
        ["./LICENSE", "./build/LICENSE"],
    ]) {
        fs.cpSync(...item, { force: true, recursive: true });
    }

    if (process.platform == "win32") {
        for (const item of [
            [`./target/${Profile.toLowerCase()}/hylarana-example.exe`, "./build/bin/example.exe"],
            [
                `./target/${Profile.toLowerCase()}/hylarana-server.exe`,
                "./build/bin/hylarana-server.exe",
            ],
            [`${FFmpegOutputDir}/bin/avcodec-61.dll`, "./build/bin/avcodec-61.dll"],
            [`${FFmpegOutputDir}/bin/avutil-59.dll`, "./build/bin/avutil-59.dll"],
            [`${FFmpegOutputDir}/bin/swresample-5.dll`, "./build/bin/swresample-5.dll"],
            [`${CefOutputDir}/Release`, "./build/bin"],
            [`${CefOutputDir}/Resources`, "./build/bin"],
        ]) {
            fs.cpSync(...item, { force: true, recursive: true });
        }
    } else if (process.platform == "darwin") {
        for (const item of [
            [`./target/${Profile.toLowerCase()}/hylarana-example`, "./build/bin/example"],
            [`./target/${Profile.toLowerCase()}/hylarana-server`, "./build/bin/hylarana-server"],
        ]) {
            fs.cpSync(...item, { force: true, recursive: true });
        }
    } else if (process.platform == "linux") {
        for (const item of [
            [`./target/${Profile.toLowerCase()}/hylarana-example`, "./build/bin/example"],
            [`./target/${Profile.toLowerCase()}/hylarana-server`, "./build/bin/hylarana-server"],
            [`${FFmpegOutputDir}/lib`, "./build/bin"],
        ]) {
            fs.cpSync(...item, { force: true, recursive: true });
        }
    }

    if (process.platform == "win32") {
        for (const item of [
            ["./target/debug/hylarana_server.pdb", "./build/bin/server.pdb"],
            ["./target/debug/hylarana_example.pdb", "./build/bin/example.pdb"],
        ]) {
            if (!Args.release) {
                fs.cpSync(...item, { force: true, recursive: true });
            } else {
                fs.rmSync(item[1], { force: true, recursive: true });
            }
        }
    }

    /* async block end */
})().catch((e) => {
    console.error(e);
    process.exit(-1);
});
