import { Device, SenderOptions, Source, SourceType } from "./hylarana";

declare global {
    interface Window {
        MessageTransport: {
            send: (message: string) => void;
            on: (handle: (message: string) => void) => void;
        };
    }
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

let transport: {
    sequence: number;
    requests: {
        [key: number]: (response: unknown) => void;
    };
    listeners: {
        [key: string]: (request: unknown) => void;
    };
} = {
    sequence: 0,
    requests: {},
    listeners: {},
};

if (window.MessageTransport) {
    window.MessageTransport.on((message) => {
        try {
            const payload: Payload<unknown> = JSON.parse(message);
            console.log("message transport recv payload = ", payload);

            if (payload.ty == "Request") {
                const { method } = payload.content as Request<unknown>;
                if (transport.listeners[method]) {
                    transport.listeners[method](payload.content);
                }
            } else {
                const { sequence, content } = payload.content as Response<unknown>;
                if (transport.requests[sequence]) {
                    transport.requests[sequence](content);
                }
            }
        } catch (e) {
            console.log(e);
        }
    });
}

function sendMessage<T>(payload: T) {
    console.log("message transport send payload = ", payload);

    if (window.MessageTransport) {
        window.MessageTransport.send(JSON.stringify(payload));
    }
}

export class Route {
    static on<K extends keyof OnTypes, Q extends OnTypes[K][0], S extends OnTypes[K][1]>(
        method: string,
        handle: (request: Q) => Promise<S> | S
    ) {
        transport.listeners[method] = async (request: unknown) => {
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

            sendMessage({
                ty: "Response",
                content: {
                    sequence,
                    content: body,
                },
            });
        };
    }

    static call<K extends keyof CallTypes, Q extends CallTypes[K][0], S extends CallTypes[K][1]>(
        method: K,
        req?: Q
    ): Promise<S> {
        return new Promise((resolve, reject) => {
            const sequence = transport.sequence;
            if (transport.sequence == 65535) {
                transport.sequence = 0;
            } else {
                transport.sequence += 1;
            }

            const timeout = setTimeout(() => {
                delete transport.requests[sequence];

                reject("request timeout");
            }, 5000);

            transport.requests[sequence] = (response: unknown) => {
                clearTimeout(timeout);

                {
                    const { ty, content } = response as ResponseContent<S>;
                    if (ty == "Ok") {
                        resolve(content as S);
                    } else {
                        reject(content as string);
                    }
                }

                delete transport.requests[sequence];
            };

            sendMessage({
                ty: "Request",
                content: {
                    content: req == undefined ? null : req,
                    sequence,
                    method,
                },
            });
        });
    }
}

export enum Methods {
    GetName = "GetName",
    SetName = "SetName",
    GetDevices = "GetDevices",
    DevicesChangeNotify = "DevicesChangeNotify",
    ReadyNotify = "ReadyNotify",
    GetCaptureSources = "GetCaptureSources",
    CreateSender = "CreateSender",
}

interface CallTypes {
    [Methods.GetName]: [void, string];
    [Methods.SetName]: [string, void];
    [Methods.GetDevices]: [void, Device[]];
    [Methods.GetCaptureSources]: [SourceType, Source[]];
    [Methods.CreateSender]: [[Array<string>, SenderOptions], void];
}

interface OnTypes {
    [Methods.DevicesChangeNotify]: [void, void];
    [Methods.ReadyNotify]: [void, void];
}
