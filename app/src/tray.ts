import { Menu, Tray } from "electron";
import { join } from "node:path";
import { Window } from "./window";
import { Service } from "./service";
import { SettingsValue } from "./settings";

const LOCALES = {
    english: {
        Shutdown: "Shutdown",
        Tip: "Hylarana - cross platform screencast",
    },
    chinase: {
        Shutdown: "关闭",
        Tip: "Hylarana - 跨平台投屏",
    },
};

let tray: Tray | null = null;

export namespace TrayIcon {
    export function update(language: "english" | "chinase") {
        tray?.setToolTip(LOCALES[language].Tip);
        tray?.setContextMenu(
            Menu.buildFromTemplate([
                {
                    type: "normal",
                    click: () => {
                        Service.shutdown();
                    },
                    label: LOCALES[language].Shutdown,
                },
            ])
        );
    }
}

/**
 * On macos, instead of using the tray, just open the window.
 *
 * But on windows, the experience of using the tray icon is very good, so windows gives preference to the tray.
 */
if (process.platform == "darwin") {
    Window.show();
} else {
    tray = new Tray(join(__dirname, "../assets/tray-icon.png"));
    tray.on("click", () => {
        Window.show();
    });

    TrayIcon.update(SettingsValue.system.language);
}