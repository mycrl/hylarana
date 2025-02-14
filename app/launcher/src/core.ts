import { spawn } from "node:child_process";
import { Settings } from "./settings";
import { app } from "electron";
import {
    Backend,
    Device,
    MediaStreamDescription,
    SenderOptions,
    Source,
    SourceType,
    Status,
} from "../common";

export enum Methods {
    GetDevices = "GetDevices",
    DevicesChangeNotify = "DevicesChangeNotify",
    ReadyNotify = "ReadyNotify",
    GetCaptureSources = "GetCaptureSources",
    CreateSender = "CreateSender",
    CloseSender = "CloseSender",
    CreateReceiver = "CreateReceiver",
    CloseReceiver = "CloseReceiver",
    GetStatus = "GetStatus",
    SenderClosedNotify = "SenderClosedNotify",
    ReceiverClosedNotify = "ReceiverClosedNotify",
    SenderCreatedNotify = "SenderCreatedNotify",
    ReceiverCreatedNotify = "ReceiverCreatedNotify",
}

interface CallParams {
    [Methods.GetDevices]: [void, Device[]];
    [Methods.GetCaptureSources]: [SourceType, Source[]];
    [Methods.CreateSender]: [[Array<string>, SenderOptions], void];
    [Methods.CloseSender]: [void, void];
    [Methods.CreateReceiver]: [[VideoDecoder, Backend, MediaStreamDescription], void];
    [Methods.CloseReceiver]: [void, void];
    [Methods.GetStatus]: [void, Status];
}

interface OnParams {
    [Methods.DevicesChangeNotify]: [void, void];
    [Methods.ReadyNotify]: [void, void];
    [Methods.SenderClosedNotify]: [void, void];
    [Methods.ReceiverClosedNotify]: [void, void];
    [Methods.SenderCreatedNotify]: [void, void];
    [Methods.ReceiverCreatedNotify]: [void, void];
}

interface Payload<T> {
    ty: "Request" | "Response";
    content: Request<T> | Response<T>;
}

interface Request<T> {
    method: string;
    sequence: number;
    content: T;
}

interface ResponseContent<T> {
    ty: "Ok" | "Err";
    content: T | string;
}

interface Response<T> {
    sequence: number;
    content: ResponseContent<T>;
}

export class Service {
    private sequence: number = 0;

    private requests: {
        [key: number]: (response: unknown) => void;
    } = {};

    private listeners: {
        [key: string]: (request: unknown) => void;
    } = {};

    private service = spawn(
        process.env.CORE_EXE || "./hylarana-app-core",
        ["--name", Settings.get().system_name],
        {
            stdio: ["pipe", "pipe", "inherit"],
            windowsHide: true,
            shell: false,
        }
    );

    constructor() {
        for (const event of ["close", "disconnect", "error", "exit"]) {
            this.service.on(event, () => {
                app.exit();
            });
        }

        this.service.stdout?.on("data", (buffer: Buffer) => {
            const message = buffer.toString("utf8");

            if (message.startsWith("::MESSAGE-")) {
                try {
                    const payload: Payload<unknown> = JSON.parse(message);
                    console.log("message transport recv payload = ", payload);

                    if (payload.ty == "Request") {
                        const { method } = payload.content as Request<unknown>;
                        if (this.listeners[method]) {
                            this.listeners[method](payload.content);
                        }
                    } else {
                        const { sequence, content } = payload.content as Response<unknown>;
                        if (this.requests[sequence]) {
                            this.requests[sequence](content);
                        }
                    }
                } catch (e) {
                    console.error(e);
                }
            }
        });
    }

    public on<K extends keyof OnParams, Q extends OnParams[K][0], S extends OnParams[K][1]>(
        method: string,
        handle: (request: Q) => Promise<S> | S
    ) {
        this.listeners[method] = async (request: unknown) => {
            const { sequence, content } = request as Request<Q>;

            let body = null;
            try {
                const future = handle(content);
                const res = future instanceof Promise ? await future : future;
                body = {
                    ty: "Ok",
                    content: res == undefined ? null : res,
                };
            } catch (e: any) {
                body = {
                    ty: "Err",
                    content: e.message,
                };
            }

            this.send_payload({
                ty: "Response",
                content: {
                    sequence,
                    content: body,
                },
            });
        };
    }

    public call<K extends keyof CallParams, Q extends CallParams[K][0], S extends CallParams[K][1]>(
        method: K,
        req?: Q
    ): Promise<S> {
        return new Promise((resolve, reject) => {
            const sequence = this.sequence;
            if (this.sequence == 65535) {
                this.sequence = 0;
            } else {
                this.sequence += 1;
            }

            const timeout = setTimeout(() => {
                delete this.requests[sequence];

                reject("request timeout");
            }, 5000);

            this.requests[sequence] = (response: unknown) => {
                clearTimeout(timeout);

                {
                    const { ty, content } = response as ResponseContent<S>;
                    if (ty == "Ok") {
                        resolve(content as S);
                    } else {
                        reject(content as string);
                    }
                }

                delete this.requests[sequence];
            };

            this.send_payload({
                ty: "Request",
                content: {
                    content: req == undefined ? null : req,
                    sequence,
                    method,
                },
            });
        });
    }

    private send_payload(payload: any) {
        this.service.stdin?.write("::MESSAGE-" + JSON.stringify(payload) + "\n");
    }
}
