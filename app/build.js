const { existsSync } = require("node:fs");
const { cp, readdir } = require("node:fs/promises");
const { exec } = require("node:child_process");
const { join } = require("node:path");

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
                    stdio: "inherit",
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

const FindCrateOutdir = async (package, dir) => {
    const target = join(__dirname, "../target/release/build/");
    for (const item of await readdir(target)) {
        if (item.startsWith(package)) {
            const path = join(target, item, "./out/", dir);
            if (existsSync(path)) {
                return path;
            }
        }
    }

    throw new Error("not found cargo crate outdir");
};

void (async () => {
    await Command("cargo build --release", {
        cwd: "./core",
    });

    if (process.platform == "win32") {
        const outdir = await FindCrateOutdir("ffmpeg-dev-sys", "ffmpeg");
        for (const item of ["avcodec-61.dll", "avutil-59.dll", "swresample-5.dll"]) {
            const path = join("../target/release/", item);
            if (!existsSync(path)) {
                await cp(join(outdir, "./bin", item), path);
            }
        }
    }

    await Command("npm run build", {
        cwd: "./ui",
    });
})();
