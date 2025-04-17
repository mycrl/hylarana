import { cp, readdir, mkdir, rm, readFile, writeFile } from "node:fs/promises";
import { exec, execSync } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

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

const Command = (cmd, options = {}, output = true) =>
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
                    env: process.env,
                    stdio: "inherit",
                    cwd: __dirname,
                    ...options,
                }
            )
        ) => {
            if (output) {
                ps.stdout.pipe(process.stdout);
                ps.stderr.pipe(process.stderr);
            }

            ps.on("error", reject);
            ps.on("close", (code) => {
                code == 0 ? resolve() : reject(code || 0);
            });
        }
    );

const GetCrateOutdir = async (target, crate, dir) => {
    const build = join(__dirname, target, "./build");
    for (const item of await readdir(build)) {
        if (item.startsWith(crate)) {
            const path = join(build, item, "./out/", dir);
            if (existsSync(path)) {
                return path;
            }
        }
    }

    throw new Error("not found cargo crate outdir");
};

void (async () => {
    const profile = Args.release ? "release" : "debug";

    await Command("yarn build", {
        cwd: "../frontend",
    });

    await Command(Args.release ? "cargo build --release" : "cargo build", {
        cwd: "./",
        env: {
            ...process.env,
            MACOSX_DEPLOYMENT_TARGET: "15.4",
        },
    });

    if (!existsSync("../../target/app")) {
        await mkdir("../../target/app");
    }

    if (process.platform == "win32") {
        const cefOutDir = join(
            await GetCrateOutdir(`../../target/${profile}`, "webview", "./cef/Release"),
            "../"
        );
        const ffOutDir = await GetCrateOutdir(`../../target/${profile}`, "ffmpeg", "./ffmpeg");

        for (const item of [
            ["../frontend/dist", "../../target/app/webview"],
            [`../../target/${profile}/hylarana-app.exe`, "../../target/app/hylarana-app.exe"],
            [
                `../../target/${profile}/hylarana-app-helper.exe`,
                "../../target/app/hylarana-app-helper.exe",
            ],
            [`${cefOutDir}/Release`, "../../target/app/"],
            [`${cefOutDir}/Resources`, "../../target/app/"],
            [`${ffOutDir}/bin/avcodec-61.dll`, "../../target/app/avcodec-61.dll"],
            [`${ffOutDir}/bin/avutil-59.dll`, "../../target/app/avutil-59.dll"],
            [`${ffOutDir}/bin/swresample-5.dll`, "../../target/app/swresample-5.dll"],
        ]) {
            await cp(...item, { force: true, recursive: true });
        }

        for (const path of ["../../target/app/cef_sandbox.lib", "../../target/app/libcef.lib"]) {
            if (existsSync(path)) {
                await rm(path, { force: true, recursive: true });
            }
        }
    } else if (process.platform == "darwin") {
        const cefReleasePath = await GetCrateOutdir(
            `../../target/${profile}`,
            "webview",
            "./cef/Release"
        );

        for (const path of [
            "../../target/app/Hylarana.app",
            "../../target/app/Hylarana.app/Contents",
            "../../target/app/Hylarana.app/Contents/MacOS",
            "../../target/app/Hylarana.app/Contents/Frameworks",
            "../../target/app/Hylarana.app/Contents/Resources",
            "../../target/app/Hylarana.app/Contents/Resources/webview",
        ]) {
            if (!existsSync(path)) {
                await mkdir(path);
            }
        }

        // generate icon
        {
            const iconsetPath = join(__dirname, "../../target/app/icon.iconset");
            if (!existsSync(iconsetPath)) {
                await mkdir(iconsetPath);
            }

            for (const [width, size] of [
                [16, "16x16"],
                [32, "16x16@2x"],
                [32, "32x32"],
                [64, "32x32@2x"],
                [128, "128x128"],
                [256, "128x128@2x"],
                [256, "256x256"],
            ]) {
                await Command(
                    `sips -z ${width} ${width} ${join(
                        __dirname,
                        "./../../logo.png"
                    )} --out ${iconsetPath}/icon_${size}.png`,
                    {},
                    false
                );
            }

            await Command("iconutil -c icns icon.iconset", {
                cwd: "../../target/app",
            });
        }

        for (const item of [
            ["./Info.plist", "../../target/app/Hylarana.app/Contents/Info.plist"],
            [
                "../../target/app/icon.icns",
                "../../target/app/Hylarana.app/Contents/Resources/icon.icns",
            ],
            [
                `../../target/${profile}/hylarana-app`,
                "../../target/app/Hylarana.app/Contents/MacOS/Hylarana",
            ],
            [
                join(cefReleasePath, "./Chromium Embedded Framework.framework"),
                "../../target/app/Hylarana.app/Contents/Frameworks/Chromium Embedded Framework.framework",
            ],
            ["../frontend/dist", "../../target/app/Hylarana.app/Contents/Resources/webview"],
        ]) {
            await cp(...item, { force: true, recursive: true });
        }

        // generate helper
        {
            for (const [name, identifier] of [
                ["Hylarana Helper", "com.github.mycrl.hylarana.helper"],
                ["Hylarana Helper (GPU)", "com.github.mycrl.hylarana.helper.gpu"],
                ["Hylarana Helper (Plugin)", "com.github.mycrl.hylarana.helper.plugin"],
                ["Hylarana Helper (Renderer)", "com.github.mycrl.hylarana.helper.renderer"],
            ]) {
                const helperPath = join(
                    __dirname,
                    "../../target/app/Hylarana.app/Contents/Frameworks",
                    `./${name}.app`
                );

                for (const path of ["", "Contents", "Contents/MacOS", "Contents/Resources"]) {
                    if (!existsSync(join(helperPath, path))) {
                        await mkdir(join(helperPath, path));
                    }
                }

                {
                    await writeFile(
                        join(helperPath, "Contents/Info.plist"),
                        (await readFile("./helper.Info.plist", "utf8"))
                            .replace("{{CFBundleName}}", name)
                            .replace("{{CFBundleExecutable}}", name)
                            .replace("{{CFBundleIdentifier}}", identifier)
                    );
                }

                for (const item of [
                    [
                        "../../target/app/icon.icns",
                        join(helperPath, "Contents/Resources/icon.icns"),
                    ],
                    [
                        `../../target/${profile}/hylarana-app-helper`,
                        join(helperPath, `Contents/MacOS/${name}`),
                    ],
                ]) {
                    await cp(...item, { force: true, recursive: true });
                }

                await Command(`install_name_tool -change \
                    "@executable_path/../Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework" \
                    "@rpath/Chromium Embedded Framework.framework/Chromium Embedded Framework" \
                    "${join(helperPath, `Contents/MacOS/${name}`)}"`);

                await Command(`install_name_tool \
                    -add_rpath "@executable_path/../../../../Frameworks" \
                    "${join(helperPath, `Contents/MacOS/${name}`)}"`);
            }
        }

        {
            const ffDylibPath = join(
                execSync("brew --prefix ffmpeg@7").toString().replace(/\n/g, ""),
                "./lib"
            );

            const dirFiles = await readdir(ffDylibPath);
            for (const item of ["libavcodec", "libavutil", "libswresample"]) {
                for (const file of dirFiles.filter(
                    (it) => it.startsWith(item) && it.endsWith(".dylib")
                )) {
                    const dst = "../../target/app/Hylarana.app/Contents/Frameworks/" + file;
                    if (!existsSync(dst)) {
                        await cp(join(ffDylibPath, file), dst, { force: true, recursive: true });
                    }
                }

                await Command(`install_name_tool -change ${join(ffDylibPath, item + ".dylib")} \
                    @executable_path/../Frameworks/${item}.dylib \
                    ../../target/app/Hylarana.app/Contents/MacOS/Hylarana`);
            }
        }

        for (const path of ["../../target/app/icon.iconset", "../../target/app/icon.icns"]) {
            if (existsSync(path)) {
                await rm(path, { force: true, recursive: true });
            }
        }
    }
})();
