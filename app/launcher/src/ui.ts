import { BrowserWindow } from "electron";
import { join } from "node:path";

const DefaultWindowOptions = {
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
};

const window = new BrowserWindow(DefaultWindowOptions);
