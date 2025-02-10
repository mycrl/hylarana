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
            ps = child_process.spawn(
                process.platform == "win32"
                    ? "$ProgressPreference = 'SilentlyContinue';" + cmd
                    : cmd,
                {
                    shell: process.platform == "win32" ? "powershell.exe" : "bash",
                    stdio: "inherit",
                    stderr: "inherit",
                    cwd: __dirname,
                    ...options,
                }
            )
        ) => {
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

const DefaultFsOptions = { force: true, recursive: true };

/* async block */
void (async () => {
    const Profile = Args.release ? "release" : "debug";

    for (const path of [
        "./target",
        "./target/build",
        "./target/build",
        "./target/build/resources",
    ]) {
        if (!fs.existsSync(path)) {
            fs.mkdirSync(path);
        }
    }

    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-example`);
    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-server`);
    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-app-core`);

    const FFmpegOutputDir = SearchBuild(
        "hylarana-ffmpeg-sys",
        "ffmpeg-n7.1-latest-win64-gpl-shared-7.1"
    );

    if (process.platform == "win32") {
        for (const item of [
            [`./target/${Profile}/hylarana-example.exe`, "./target/build/hylarana-example.exe"],
            [`./target/${Profile}/hylarana-server.exe`, "./target/build/hylarana-server.exe"],
            [`./target/${Profile}/hylarana-app-core.exe`, "./target/build/hylarana-app-core.exe"],
            [`${FFmpegOutputDir}/bin/avcodec-61.dll`, "./target/build/avcodec-61.dll"],
            [`${FFmpegOutputDir}/bin/avutil-59.dll`, "./target/build/avutil-59.dll"],
            [`${FFmpegOutputDir}/bin/swresample-5.dll`, "./target/build/swresample-5.dll"],
        ]) {
            fs.cpSync(...item, DefaultFsOptions);
        }
    } else if (process.platform == "darwin") {
        for (const item of [
            [`./target/${Profile}/hylarana-example`, "./target/build/hylarana-example"],
            [`./target/${Profile}/hylarana-server`, "./target/build/hylarana-server"],
            [`./target/${Profile}/hylarana-app-core`, "./target/build/hylarana-app-core"],
        ]) {
            fs.cpSync(...item, DefaultFsOptions);
        }
    } else if (process.platform == "linux") {
        for (const item of [
            [`./target/${Profile}/hylarana-example`, "./target/build/hylarana-example"],
            [`./target/${Profile}/hylarana-server`, "./target/build/hylarana-server"],
            [`./target/${Profile}/hylarana-app-core`, "./target/build/hylarana-app-core"],
            [`${FFmpegOutputDir}/lib`, "./target/build/"],
        ]) {
            fs.cpSync(...item, DefaultFsOptions);
        }
    }

    if (process.platform == "win32") {
        for (const item of [
            ["./target/debug/hylarana_example.pdb", "./target/build/hylarana_example.pdb"],
            ["./target/debug/hylarana_server.pdb", "./target/build/hylarana_server.pdb"],
            ["./target/debug/hylarana_app_core.pdb", "./target/build/hylarana_app_core.pdb"],
        ]) {
            if (!Args.release) {
                fs.cpSync(...item, DefaultFsOptions);
            } else {
                fs.rmSync(item[1], DefaultFsOptions);
            }
        }
    }

    /* async block end */
})().catch((e) => {
    console.error(e);
    process.exit(-1);
});
