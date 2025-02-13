import "dotenv/config";

import { app, BrowserWindow, Menu, Tray, ipcMain } from "electron";
import { ChildProcess, spawn } from "node:child_process";
import { accessSync, readFileSync, writeFileSync } from "node:fs";
import { join as pathJoin } from "node:path";
import { faker } from "@faker-js/faker";
import EventEmitter from "node:events";

const appUserPath = app.getPath("userData");
const settingsPath = pathJoin(appUserPath, "./settings");

console.log("app settings path = ", settingsPath);

interface Settings {
    name: string;
}

try {
    accessSync(settingsPath);
} catch {
    writeFileSync(
        settingsPath,
        JSON.stringify({
            name: faker.person.fullName(),
        })
    );
}

const settings = JSON.parse(readFileSync(settingsPath, "utf8")) as Settings;

if (process.env.RANDOM_NAME) {
    settings.name = faker.person.fullName();
}

const events = new EventEmitter();

const window = new BrowserWindow({
    width: 1000,
    height: 600,
    useContentSize: true,
    resizable: false,
    maximizable: false,
    fullscreenable: false,
    webPreferences: {
        preload: pathJoin(__dirname, "../preload.js"),
    },
});

let core: ChildProcess | null = null;

events.on("reloadCoreProcess", () => {
    if (core) {
        core.kill();
    }

    core = spawn(process.env.CORE_EXE || "./hylarana-app-core", ["--name", settings.name], {
        stdio: ["pipe", "pipe", "inherit"],
        windowsHide: true,
        shell: false,
    });

    let isClosed = false;
    for (const event of ["close", "disconnect", "error", "exit"]) {
        core.on(event, () => {
            if (!isClosed) {
                isClosed = true;

                app.exit();
            }
        });
    }

    core.stdout?.on("data", (buffer: Buffer) => {
        const message = buffer.toString("utf8");

        // Intercept the event that the child process is ready.
        if (message.includes(`"method":"ReadyNotify"`)) {
            events.emit("ready");
        }

        window.webContents.send("MessageTransport", message);
    });
});

const tray = new Tray("./assets/logoTemplate.png");

tray.setToolTip("hylarana - cross platform screencast");
tray.setContextMenu(
    Menu.buildFromTemplate([
        {
            label: "退出",
            type: "normal",
            click: () => {
                core?.kill();
                app.exit();
            },
        },
    ])
);

tray.on("click", () => {
    window.show();
});

{
    events.emit("reloadCoreProcess");

    let created = false;
    events.on("ready", () => {
        if (!created) {
            created = true;

            if (process.env.MAIN_URL) {
                window.loadURL(process.env.MAIN_URL);
            } else {
                window.loadFile("./ui/dist/index.html");
            }
        }
    });
}

ipcMain.on("MessageTransport", (_, message: string) => {
    core?.stdin?.write(message + "\n");
});

ipcMain.handle("GetName", async () => {
    return settings.name;
});

ipcMain.handle("SetName", (_, name) => {
    settings.name = name;

    events.emit("reloadCoreProcess");
});
