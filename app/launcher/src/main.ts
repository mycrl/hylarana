import { app, BrowserWindow, Menu, Tray, ipcMain } from "electron";
import { ChildProcess, spawn } from "node:child_process";
import { accessSync, readFileSync, writeFileSync } from "node:fs";
import { join as pathJoin } from "node:path";
import { faker } from "@faker-js/faker";

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

const window = new BrowserWindow({
    width: 1000,
    height: 600,
    webPreferences: {
        preload: pathJoin(__dirname, "../preload.js"),
    },
});

let core: ChildProcess | null = null;

function reloadCoreProcess() {
    core = spawn("../../target/build/hylarana-app-core", ["--name", settings.name], {
        stdio: ["pipe", "pipe", "inherit"],
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
        window.webContents.send("MessageTransport", buffer.toString("utf8"));
    });
}

const tray = new Tray("./assets/logoTemplate.png");

tray.setToolTip("hylarana - cross platform screencast");
tray.setContextMenu(
    Menu.buildFromTemplate([
        {
            label: "退出",
            type: "normal",
            click: () => {
                app.exit();
            },
        },
    ])
);

tray.on("click", () => {
    window.show();
});

app.on("window-all-closed", () => {
    if (process.platform !== "darwin") {
        core?.kill();
    }
});

{
    if (process.env.MAIN_URL) {
        window.loadURL(process.env.MAIN_URL);
    } else {
        window.loadFile("./ui/dist/index.html");
    }

    reloadCoreProcess();
}

ipcMain.on("MessageTransport", (_, message: string) => {
    core?.stdin?.write(message + "\n");
});

ipcMain.handle("GetName", async () => {
    return settings.name;
});

ipcMain.handle("SetName", (_, name) => {
    settings.name = name;

    reloadCoreProcess();
});
