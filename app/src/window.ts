import { BrowserWindow } from "electron";
import { join } from "node:path";
import { Service } from "./service";

const view = new BrowserWindow({
    title: "Hylarana",
    width: 1000,
    height: 600,
    show: false,
    useContentSize: true,
    resizable: false,
    maximizable: false,
    fullscreenable: false,
    autoHideMenuBar: true,
    webPreferences: {
        preload: join(__dirname, "../preload.js"),
    },
});

view.on("close", (event) => {
    if (process.platform == "darwin") {
        Service.shutdown();
    } else {
        event.preventDefault();
        view.hide();
    }
});

export namespace Window {
    const URI = process.env.MAIN_URL || join(__dirname, "../ui/dist/index.html");

    export function open() {
        URI.startsWith("http://") || URI.startsWith("https://")
            ? view.loadURL(URI)
            : view.loadFile(URI);

        if (process.env.DEVTOOLS) {
            view.webContents.openDevTools();
        }
    }

    export function sendMessage(name: string, content?: any) {
        view.webContents.send(name, content);
    }

    export function show() {
        view.show();
    }
}
