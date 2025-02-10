import { atom } from "jotai";
import { Observable } from "rxjs";
import { Route, Methods } from "./message";
import { atomWithObservable } from "jotai/utils";

export const VideoEncoders = {
    X264: "X264",
    Qsv: "Intel QSV - Windows",
    VideoToolBox: "VideoToolbox - Apple",
};

export const VideoDecoders = {
    H264: "H264",
    D3D11: "D3D11VA - Windows",
    Qsv: "Intel QSV - Windows",
    VideoToolBox: "VideoToolbox - Apple",
};

export enum TransportStrategy {
    Direct = "Direct",
    Relay = "Relay",
    Multicast = "Multicast",
}

export enum VideoFormat {
    BGRA = "BGRA",
    RGBA = "RGBA",
    NV12 = "NV12",
    I420 = "I420",
}

export interface SenderOptions {
    transport: {
        mtu: number;
        strategy: {
            strategy: TransportStrategy;
            address: string;
        };
    };
    video: {
        size: {
            width: number;
            height: number;
        };
        fps: number;
        bitRate: number;
    };
    audio: {
        sampleRate: number;
        bitRate: number;
    };
}

export interface MediaStreamDescription extends SenderOptions {
    id: string;
    transport: {
        mtu: number;
        strategy: {
            strategy: TransportStrategy;
            address: string;
        };
    };
    video: {
        format: VideoFormat;
        size: {
            width: number;
            height: number;
        };
        fps: number;
        bitRate: number;
    };
    audio: {
        sampleRate: number;
        channels: number;
        bitRate: number;
    };
}

function intoMediaStreamDescription(value: any): MediaStreamDescription {
    let strategy = TransportStrategy.Direct;
    {
        if (value.t.s.t == "r") {
            strategy = TransportStrategy.Relay;
        } else if (value.t.s.t == "m") {
            strategy = TransportStrategy.Multicast;
        }
    }

    return {
        id: value.i,
        transport: {
            mtu: value.t.m,
            strategy: {
                address: value.t.s.v,
                strategy,
            },
        },
        video: {
            format: value.v.f,
            size: {
                width: value.v.s.w,
                height: value.v.s.h,
            },
            fps: value.v.fps,
            bitRate: value.v.br,
        },
        audio: {
            sampleRate: value.a.sr,
            channels: value.a.cs,
            bitRate: value.a.br,
        },
    };
}

export enum DeviceType {
    Windows = "Windows",
    Android = "Android",
    Apple = "Apple",
}

export interface Device {
    addrs: string[];
    kind: DeviceType;
    name: string;
    port: number;
    description: MediaStreamDescription | null;
}

export enum SourceType {
    Camera = "Camera",
    Screen = "Screen",
    Audio = "Audio",
}

export interface Source {
    id: string;
    index: number;
    is_default: boolean;
    kind: SourceType;
    name: string;
}

const devicesChangeObserver = new Observable<Device[]>((subscriber) => {
    console.log("init devices change notify observer");

    function notify() {
        Route.call(Methods.GetDevices).then((data) => {
            subscriber.next(
                data.map((it) => {
                    return {
                        ...it,
                        description: it.description
                            ? intoMediaStreamDescription(it.description)
                            : null,
                    };
                })
            );
        });
    }

    notify();

    let closed = false;
    Route.on(Methods.DevicesChangeNotify, () => {
        if (!closed) {
            notify();
        }
    });

    return () => {
        closed = true;
    };
});

export const DevicesAtom = atomWithObservable<Device[]>(() => devicesChangeObserver);

export const DisplaysAtom = atom(() => {
    return Route.call(Methods.GetCaptureSources, SourceType.Screen);
});

export const AudiosAtom = atom(() => {
    return Route.call(Methods.GetCaptureSources, SourceType.Audio);
});

export async function createSender(addrs: string[], options: SenderOptions) {
    return await Route.call(Methods.CreateSender, [addrs, options]);
}
