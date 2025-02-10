"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const electron_1 = require("electron");
const node_child_process_1 = require("node:child_process");
const node_fs_1 = require("node:fs");
const node_path_1 = require("node:path");
const faker_1 = require("@faker-js/faker");
const LINE_START = "::MESSAGE_TRANSPORTS-";
const appUserPath = electron_1.app.getPath("userData");
const settingsPath = (0, node_path_1.join)(appUserPath, "./settings");
console.log("app settings path = ", settingsPath);
try {
    (0, node_fs_1.accessSync)(settingsPath);
}
catch {
    (0, node_fs_1.writeFileSync)(settingsPath, JSON.stringify({
        name: faker_1.faker.person.fullName(),
    }));
}
const settings = JSON.parse((0, node_fs_1.readFileSync)(settingsPath, "utf8"));
const window = new electron_1.BrowserWindow({
    width: 1000,
    height: 600,
    webPreferences: {
        devTools: process.env.DEVTOOLS == "1",
        preload: "../preload.js",
    },
});
let core = null;
function reloadCoreProcess() {
    core = (0, node_child_process_1.execFile)("../../target/build/hylarana-app-core", ["--name", settings.name]);
    let isClosed = false;
    for (const event of ["close", "disconnect", "error", "exit"]) {
        core.on(event, () => {
            if (!isClosed) {
                isClosed = true;
                electron_1.app.exit();
            }
        });
    }
    core.stderr?.pipe(process.stderr);
    core.stdout?.on("data", (message) => {
        if (message.startsWith(LINE_START)) {
            window.webContents.send("MessageTransport", message.slice(LINE_START.length));
        }
        else {
            process.stdout.write(message);
        }
    });
}
// const tray = new Tray("../../logo.ico");
// tray.setToolTip("hylarana - cross platform screencast");
// tray.setContextMenu(
//     Menu.buildFromTemplate([
//         {
//             label: "退出",
//             type: "normal",
//             click: () => {
//                 app.exit();
//             },
//         },
//     ])
// );
// tray.on("click", () => {
//     window.show();
// });
electron_1.app.on("window-all-closed", () => {
    if (process.platform !== "darwin") {
        core?.kill();
    }
});
{
    if (process.env.MAIN_URL) {
        console.log(process.env.MAIN_URL);
        window.loadURL(process.env.MAIN_URL);
    }
    else {
        window.loadFile("./ui/dist/index.html");
    }
    // reloadCoreProcess();
}
electron_1.ipcMain.on("MessageTransport", (_, message) => {
    core?.stdin?.write(message);
});
electron_1.ipcMain.handle("GetName", async () => {
    return settings.name;
});
electron_1.ipcMain.handle("SetName", (_, name) => {
    settings.name = name;
    reloadCoreProcess();
});
