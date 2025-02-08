import { ONCE } from "./utils";

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

const transport: {
    sequence: number;
    requests: {
        [key: number]: (response: unknown) => void;
    };
    listeners: {
        [key: string]: (request: unknown) => void;
    };
} = ONCE("message-transport", () => {
    // window.MessageTransport.on((message) => {
    //     try {
    //         const payload: Payload<unknown> = JSON.parse(message);
    //         console.log("message transport recv payload = ", payload);

    //         if (payload.ty == "Request") {
    //             const { method } = payload.content as Request<unknown>;
    //             if (window.__MESSAGE_TRANSPORT!.listeners[method]) {
    //                 window.__MESSAGE_TRANSPORT!.listeners[method](payload.content);
    //             }
    //         } else {
    //             const { sequence, content } = payload.content as Response<unknown>;
    //             if (window.__MESSAGE_TRANSPORT!.requests[sequence]) {
    //                 window.__MESSAGE_TRANSPORT!.requests[sequence](content);
    //             }
    //         }
    //     } catch (e) {
    //         console.log(e);
    //     }
    // });

    return {
        sequence: 0,
        requests: {},
        listeners: {},
    };
});

function sendMessage<T>(payload: T) {
    console.log("message transport send payload = ", payload);

    window.MessageTransport.send(JSON.stringify(payload));
}

export class MessageRouter {
    static on<Q, S>(method: string, handle: (request: Q) => Promise<S> | S) {
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

    static call<Q, S>(method: string, req?: Q): Promise<S> {
        return new Promise((resolve, reject) => {
            const sequence = transport.sequence;
            if (transport.sequence == 65535) {
                transport.sequence = 0;
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
