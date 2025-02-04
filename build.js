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
    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-app`);

    const CefOutputDir = SearchBuild("webview-sys", "cef");
    const FFmpegOutputDir = SearchBuild(
        "hylarana-ffmpeg-sys",
        "ffmpeg-n7.1-latest-win64-gpl-shared-7.1"
    );

    if (process.platform == "win32") {
        for (const item of [
            [`./target/${Profile}/hylarana-example.exe`, "./target/build/hylarana-example.exe"],
            [`./target/${Profile}/hylarana-server.exe`, "./target/build/hylarana-server.exe"],
            [`./target/${Profile}/hylarana-app.exe`, "./target/build/hylarana-app.exe"],
            [`${FFmpegOutputDir}/bin/avcodec-61.dll`, "./target/build/avcodec-61.dll"],
            [`${FFmpegOutputDir}/bin/avutil-59.dll`, "./target/build/avutil-59.dll"],
            [`${FFmpegOutputDir}/bin/swresample-5.dll`, "./target/build/swresample-5.dll"],
            [`${CefOutputDir}/Release/`, "./target/build/"],
            [`${CefOutputDir}/Resources/`, "./target/build/"],
        ]) {
            fs.cpSync(...item, DefaultFsOptions);
        }
    } else if (process.platform == "darwin") {
        for (const item of [
            [`./target/${Profile}/hylarana-example`, "./target/build/hylarana-example"],
            [`./target/${Profile}/hylarana-server`, "./target/build/hylarana-server"],
            [`./target/${Profile}/hylarana-app`, "./target/build/hylarana-app"],
        ]) {
            fs.cpSync(...item, DefaultFsOptions);
        }
    } else if (process.platform == "linux") {
        for (const item of [
            [`./target/${Profile}/hylarana-example`, "./target/build/hylarana-example"],
            [`./target/${Profile}/hylarana-server`, "./target/build/hylarana-server"],
            [`./target/${Profile}/hylarana-app`, "./target/build/hylarana-app"],
            [`${FFmpegOutputDir}/lib`, "./target/build/"],
        ]) {
            fs.cpSync(...item, DefaultFsOptions);
        }
    }

    if (process.platform == "win32") {
        for (const item of [
            ["./target/debug/hylarana_example.pdb", "./target/build/hylarana-example.pdb"],
            ["./target/debug/hylarana_server.pdb", "./target/build/hylarana-server.pdb"],
            ["./target/debug/hylarana_app.pdb", "./target/build/hylarana-app.pdb"],
        ]) {
            if (!Args.release) {
                fs.cpSync(...item, DefaultFsOptions);
            } else {
                fs.rmSync(item[1], DefaultFsOptions);
            }
        }
    }

    await Command("npm run build", { cwd: __dirname + "/app/ui" });
    fs.cpSync("./app/ui/dist/", "./target/build/resources/", DefaultFsOptions);

    fs.rmSync("./target/build/cef_sandbox.lib", DefaultFsOptions);
    fs.rmSync("./target/build/libcef.lib", DefaultFsOptions);

    /* async block end */
})().catch((e) => {
    console.error(e);
    process.exit(-1);
});
