import { spawn } from "node:child_process";
import { join } from "node:path";
import { app, ipcMain } from "electron";
import { Settings, SettingsValue } from "./settings";
import { RequestBounds, Methods } from "./hylarana";
import { Window } from "./window";
import { TrayIcon } from "./tray";

interface Payload<T> {
    ty: "Response" | "Events";
    content: Events | Response<T>;
}

interface Events {
    method: string;
}

interface ResponseContent<T> {
    ty: "Ok" | "Err";
    content: T | string;
}

interface Response<T> {
    sequence: number;
    content: ResponseContent<T>;
}

/**
 * Start the core service sub-process.
 *
 * All are captured by the subprocess module except stderr, which is passed
 * directly to the outside, which is necessary because the core service passes
 * messages via stdout and stdin.
 */
const service = spawn(process.env.CORE_EXE || join(process.cwd(), "./hylarana-app-core.exe"), {
    stdio: ["pipe", "pipe", "inherit"],
    windowsHide: true,
    shell: false,
});

const TRANSPORT: {
    sequence: number;
    listeners: {
        [key: number]: (response: unknown) => void;
    };
} = {
    sequence: 0,
    listeners: {},
};

// Sends a message to the child process.
function sendMessage<T>(message: T) {
    const payload = JSON.stringify(message);

    /**
     * Subprocesses need to distinguish individual messages by line, so add
     * trailing line breaks.
     */
    service.stdin.write(payload + "\n");

    console.log("message transport send payload = ", payload);
}

function request<
    K extends keyof RequestBounds,
    Q extends RequestBounds[K][0],
    S extends RequestBounds[K][1]
>(method: K, req?: Q): Promise<S> {
    return new Promise((resolve, reject) => {
        const sequence = TRANSPORT.sequence;

        /**
         * Although the maximum value supported by js is much larger than this,
         * but in order to deal with the simplicity, it is directly stipulated
         * that the maximum value is 65535, and if it exceeds it, it restarts from 0.
         */
        if (TRANSPORT.sequence == 65535) {
            TRANSPORT.sequence = 0;
        } else {
            TRANSPORT.sequence += 1;
        }

        /**
         * The timeout is fixed at 5 seconds, and if a response is not received after
         * that time, a timeout error is triggered.
         */
        const timeout = setTimeout(() => {
            delete TRANSPORT.listeners[sequence];

            reject("request timeout");
        }, 5000);

        TRANSPORT.listeners[sequence] = (response: unknown) => {
            clearTimeout(timeout);

            {
                const { ty, content } = response as ResponseContent<S>;
                if (ty == "Ok") {
                    resolve(content as S);
                } else {
                    reject(content as string);
                }
            }

            delete TRANSPORT.listeners[sequence];
        };

        sendMessage({
            ty: "Request",
            content: {
                /**
                 * In rust, there are no scenarios that deal with missing fields, but in js,
                 * undefined will simply ignore the field when serialising, which will
                 * result in a serialisation error in rust, so the undefined rewrite will
                 * null when encountered.
                 */
                content: req == undefined ? null : req,
                sequence,
                method,
            },
        });
    });
}

/**
 * These events are triggered to indicate that the child process has exited, this application
 * needs to rely on the child process to run, if the child process has already exited, there
 * is no point in directly exiting the current application.
 */
for (const it of ["close", "disconnect", "error", "exit"]) {
    service.on(it, () => {
        app.exit();
    });
}

service.stdout?.on("data", (buffer: Buffer) => {
    /**
     * In node.js, there is no way to read by line, so you may read multiple lines at once, here
     * it is split into individual messages by line breaks.
     */
    buffer
        .toString("utf8")
        .split("\n")
        .filter((it) => it.length > 0)
        .forEach((message) => {
            console.log("message transport recv payload = ", message);

            try {
                const payload: Payload<unknown> = JSON.parse(message);

                if (payload.ty == "Response") {
                    const { sequence, content } = payload.content as Response<unknown>;
                    if (TRANSPORT.listeners[sequence]) {
                        TRANSPORT.listeners[sequence](content);
                    }
                } else {
                    const { method } = payload.content as Events;

                    /**
                     * By default web pages are not loaded because they need to request a child
                     * process when they finish loading, and if the child process is not ready,
                     * this can cause the page to load with an error. Here we wait for the child
                     * process to notify us that it is ready before loading the page.
                     *
                     * And by default the child process has no device name, the child process
                     * is told the name of the current device when it is ready.
                     */
                    if (method == Methods.ReadyNotify) {
                        request(Methods.SetName, SettingsValue.system.name).then(() => {
                            Window.open();
                        });
                    }

                    Window.sendMessage(method);
                }
            } catch (e) {
                console.error(e);
            }
        });
});

{
    for (const name of Object.keys(Methods)) {
        /**
         * Some methods are implemented on the electron side, not the child process, and some
         * methods are events, not requests, which need to be filtered out here.
         */
        if (
            name != Methods.GetSettings &&
            name != Methods.SetSettings &&
            !name.includes("Notify")
        ) {
            ipcMain.handle(name, async (_, content) => {
                return await request(name as any, content);
            });
        }
    }
}

ipcMain.handle(Methods.GetSettings, async () => {
    return SettingsValue;
});

ipcMain.handle(Methods.SetSettings, async (_, value) => {
    /**
     * When the system language is changed, tray needs to change the language at the same time.
     */
    if (value.system.language != SettingsValue.system.language) {
        TrayIcon.update(value.system.language);
    }

    await Settings.update(value);
});

export namespace Service {
    export function shutdown() {
        service?.kill();
    }
}
