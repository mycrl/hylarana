const { exec } = require("node:child_process");
const { join } = require("node:path");
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
            ps = exec(
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

const Replace = (file, filters) => {
    let src = fs.readFileSync(file).toString();
    for (const item of filters) {
        src = src.replaceAll(...item);
    }

    fs.writeFileSync(file, src);
};

/* async block */
void (async () => {
    const Profile = Args.release ? "Release" : "Debug";

    for (const path of ["./target", "./build", "./build/bin", "./build/lib"]) {
        if (!fs.existsSync(path)) {
            fs.mkdirSync(path);
        }
    }

    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-shared`);
    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-example`);
    await Command(`cargo build ${Args.release ? "--release" : ""} -p hylarana-server`);

    /* download ffmpeg librarys for windows */
    if (process.platform == "win32" || process.platform == "linux") {
        const name = `ffmpeg-n7.1-latest-${
            process.platform == "win32" ? "win64" : "linux64"
        }-gpl-shared-7.1`;
        const baseUri = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest";

        if (!fs.existsSync("./target/ffmpeg")) {
            if (process.platform == "win32") {
                await Command(`Invoke-WebRequest -Uri ${baseUri}/${name}.zip -OutFile ffmpeg.zip`, {
                    cwd: "./target",
                });
            } else {
                await Command(`wget ${baseUri}/${name}.tar.xz -O ffmpeg.tar.xz -q`, {
                    cwd: "./target",
                });
            }

            if (process.platform == "win32") {
                await Command("Expand-Archive -Path ffmpeg.zip -DestinationPath ./", {
                    cwd: "./target",
                });
            } else {
                await Command("tar -xf ffmpeg.tar.xz", { cwd: "./target" });
            }

            fs.renameSync(`./target/${name}`, "./target/ffmpeg");
            fs.rmSync(`./target/ffmpeg.${process.platform == "win32" ? "zip" : "tar.xz"}`);
        }
    }

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
            [`./target/ffmpeg/bin/avcodec-61.dll`, "./build/bin/avcodec-61.dll"],
            [`./target/ffmpeg/bin/avutil-59.dll`, "./build/bin/avutil-59.dll"],
            [`./target/ffmpeg/bin/swresample-5.dll`, "./build/bin/swresample-5.dll"],
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
            [`./target/ffmpeg/lib`, "./build/lib"],
        ]) {
            fs.cpSync(...item, { force: true, recursive: true });
        }
    }

    if (process.platform == "win32") {
        for (const item of [
            ["./target/debug/hylarana.pdb", "./build/bin/hylarana.pdb"],
            ["./target/debug/hylarana_server.pdb", "./build/bin/hylarana_server.pdb"],
            ["./target/debug/hylarana_example.pdb", "./build/bin/hylarana_example.pdb"],
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