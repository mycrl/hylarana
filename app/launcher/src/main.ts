import { app, BrowserWindow, Menu, Tray, ipcMain } from "electron";
import { ChildProcess, execFile } from "node:child_process";
import { accessSync, readFileSync, writeFileSync } from "node:fs";
import { join as pathJoin } from "node:path";
import { faker } from "@faker-js/faker";

const LINE_START: string = "::MESSAGE_TRANSPORTS-";

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
        devTools: process.env.DEVTOOLS == "1",
        preload: pathJoin(__dirname, "../preload.js"),
    },
});

let core: ChildProcess | null = null;

function reloadCoreProcess() {
    core = execFile("../../target/build/hylarana-app-core", ["--name", settings.name]);

    let isClosed = false;
    for (const event of ["close", "disconnect", "error", "exit"]) {
        core.on(event, () => {
            if (!isClosed) {
                isClosed = true;

                app.exit();
            }
        });
    }

    core.stderr?.pipe(process.stderr);
    core.stdout?.on("data", (message: string) => {
        if (message.startsWith(LINE_START)) {
            console.log("==========================", message);
            window.webContents.send("MessageTransport", message.slice(LINE_START.length));
        } else {
            process.stdout.write(message);
        }
    });
}

const tray = new Tray("../../logo.png");

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
    core?.stdin?.write(`${LINE_START}${message}`);
});

ipcMain.handle("GetName", async () => {
    return settings.name;
});

ipcMain.handle("SetName", (_, name) => {
    settings.name = name;

    reloadCoreProcess();
});
