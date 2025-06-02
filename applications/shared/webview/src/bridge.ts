import { cfg, TargetOs } from "./utils";

/**
 * Video encoder type.
 */
export enum VideoEncoder {
    /**
     * [X264](https://www.videolan.org/developers/x264.html)
     *
     * x264 is a free software library and application for encoding video
     * streams into the H.264/MPEG-4 AVC compression format, and is released
     * under the terms of the GNU GPL.
     */
    X265 = "X265",
    /**
     * [H264 QSV](https://en.wikipedia.org/wiki/Intel_Quick_Sync_Video)
     *
     * Intel Quick Sync Video is Intel's brand for its dedicated video encoding
     * and decoding hardware core.
     */
    Qsv = "Qsv",
    /**
     * [Video Toolbox](https://developer.apple.com/documentation/videotoolbox)
     *
     * VideoToolbox is a low-level framework that provides direct access to
     * hardware encoders and decoders.
     */
    VideoToolBox = "VideoToolBox",
}

/**
 * Video decoder type.
 */
export enum VideoDecoder {
    /**
     * [Open H264](https://www.openh264.org/)
     *
     * OpenH264 is a codec library which supports H.264 encoding and decoding.
     */
    HEVC = "HEVC",
    /**
     * [D3D11VA](https://learn.microsoft.com/en-us/windows/win32/medfound/direct3d-11-video-apis)
     *
     * Accelerated video decoding using Direct3D 11 Video APIs.
     */
    D3D11 = "D3D11",
    /**
     * [H264 QSV](https://en.wikipedia.org/wiki/Intel_Quick_Sync_Video)
     *
     * Intel Quick Sync Video is Intel's brand for its dedicated video encoding
     * and decoding hardware core.
     */
    Qsv = "Qsv",
    /**
     * [Video Toolbox](https://developer.apple.com/documentation/videotoolbox)
     *
     * VideoToolbox is a low-level framework that provides direct access to
     * hardware encoders and decoders.
     */
    VideoToolBox = "VideoToolBox",
}

/**
 * Video frame format.
 */
export enum VideoFormat {
    BGRA = "BGRA",
    RGBA = "RGBA",
    NV12 = "NV12",
    I420 = "I420",
}

export enum DeviceType {
    Windows = "Windows",
    Android = "Android",
    Apple = "Apple",
}

export interface DeviceMetadata {
    port: number;
    description: MediaStreamDescription;
}

export interface Device {
    ip: string;
    name: string;
    kind: DeviceType;
    metadata: DeviceMetadata | null;
}

/**
 * Video source type or Audio source type.
 */
export enum SourceType {
    /**
     * The desktop or monitor corresponds to the desktop in the operating
     * system.
     */
    Screen = "Screen",
    /**
     * Audio input and output devices.
     */
    Audio = "Audio",
}

export enum Status {
    Sending = "Sending",
    Receiving = "Receiving",
    Idle = "Idle",
}

/**
 * Video source or Audio source.
 */
export interface Source {
    /**
     * Device ID, usually the symbolic link to the device or the address of the
     * device file handle.
     */
    id: string;
    /**
     * Sequence number, which can normally be ignored, in most cases this field
     * has no real meaning and simply indicates the order in which the device
     * was acquired internally.
     */
    index: number;
    /**
     * Whether or not it is the default device, normally used to indicate
     * whether or not it is the master device.
     */
    is_default: boolean;
    kind: SourceType;
    name: string;
}

/**
 * Transport configuration.
 */
export interface Transport {
    mtu: number;
    max_bandwidth: number;
    latency: number;
    timeout: number;
    fec: string;
    fc: number;
}

/**
 * Sender configuration.
 */
export interface SenderOptions {
    transport: Transport;
    media: {
        video: {
            source: Source;
            options: {
                codec: VideoEncoder;
                frame_rate: number;
                width: number;
                height: number;
                bit_rate: number;
                key_frame_interval: number;
            };
        } | null;
        audio: {
            source: Source;
            options: {
                sample_rate: number;
                bit_rate: number;
            };
        } | null;
    };
}

export interface ReceiverOptions {
    codec: VideoDecoder;
    transport: Transport;
}

export interface MediaStreamDescription {
    video?: {
        format: VideoFormat;
        size: {
            width: number;
            height: number;
        };
        fps: number;
        bit_rate: number;
    };
    audio?: {
        sample_rate: number;
        channels: number;
        bit_rate: number;
    };
}

export interface SystemSettings {
    name: string;
    language: "english" | "chinase";
}

export interface NetWorkSettings {
    bind: string;
    mtu: number;
    max_bandwidth: number;
    latency: number;
    timeout: number;
    fec: string;
    fc: number;
}

export interface CodecSettings {
    encoder: VideoEncoder;
    decoder: VideoDecoder;
}

export interface VideoSettings {
    width: number;
    height: number;
    frame_rate: number;
    bit_rate: number;
    key_frame_interval: number;
}

export interface AudioSettings {
    sample_rate: number;
    bit_rate: number;
}

export interface Settings {
    system: SystemSettings;
    network: NetWorkSettings;
    codec: CodecSettings;
    video: VideoSettings;
    audio: AudioSettings;
}

export interface Payload<T> {
    ty: "Request" | "Response" | "Events";
    content: Events | Request<T> | Response<T>;
}

export interface Events {
    method: string;
}

export interface Request<T> {
    method: string;
    sequence: number;
    content: T | null;
}

export interface ResponseContent<T> {
    ty: "Ok" | "Err";
    content: T | string | null;
}

export interface Response<T> {
    sequence: number;
    content: ResponseContent<T | null>;
}

export enum Methods {
    GetDevices = "GetDevices",
    GetCaptureSources = "GetCaptureSources",
    CreateSender = "CreateSender",
    CloseSender = "CloseSender",
    CreateReceiver = "CreateReceiver",
    CloseReceiver = "CloseReceiver",
    GetStatus = "GetStatus",
    GetSettings = "GetSettings",
    SetSettings = "SetSettings",
}

export interface CreateSenderParams {
    bind: string;
    targets: Array<string>;
    options: SenderOptions;
}

export interface CreateReceiverParams {
    addr: string;
    options: ReceiverOptions;
    description: MediaStreamDescription;
}

export interface RequestBounds {
    [Methods.GetDevices]: [void, Device[]];
    [Methods.GetCaptureSources]: [SourceType, Source[]];
    [Methods.CreateSender]: [CreateSenderParams, void];
    [Methods.CloseSender]: [void, void];
    [Methods.CreateReceiver]: [CreateReceiverParams, void];
    [Methods.CloseReceiver]: [void, void];
    [Methods.GetStatus]: [void, Status];
    [Methods.GetSettings]: [void, Settings];
    [Methods.SetSettings]: [Settings, void];
}

export enum EventMethods {
    StatusChangeNotify = "StatusChangeNotify",
    DevicesChangeNotify = "DevicesChangeNotify",
    ReadyNotify = "ReadyNotify",
}

export class RouteExt {
    sequence: number = 0;
    listeners: {
        [key: number | string]: (response: unknown) => void;
    } = {};

    request<
        K extends keyof RequestBounds,
        Q extends RequestBounds[K][0],
        S extends RequestBounds[K][1]
    >(method: K, req?: Q): Promise<S> {
        return new Promise((resolve, reject) => {
            const sequence = this.sequence;

            /**
             * Although the maximum value supported by js is much larger than this,
             * but in order to deal with the simplicity, it is directly stipulated
             * that the maximum value is 65535, and if it exceeds it, it restarts from 0.
             */
            if (this.sequence == 65535) {
                this.sequence = 0;
            } else {
                this.sequence += 1;
            }

            /**
             * The timeout is fixed at 5 seconds, and if a response is not received after
             * that time, a timeout error is triggered.
             */
            const timeout = setTimeout(() => {
                delete this.listeners[sequence];

                reject("request timeout");
            }, 5000);

            this.listeners[sequence] = (response: unknown) => {
                clearTimeout(timeout);

                {
                    const { ty, content } = response as ResponseContent<S>;
                    if (ty == "Ok") {
                        resolve(content as S);
                    } else {
                        reject(content as string);
                    }
                }

                delete this.listeners[sequence];
            };

            if (window.MessageTransport) {
                window.MessageTransport.send(
                    JSON.stringify({
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
                    })
                );
            }
        });
    }

    on(method: EventMethods, handle: () => Promise<void> | void) {
        this.listeners[method] = handle;
    }
}

declare global {
    interface Window {
        Route: RouteExt;
        MessageTransport?: {
            send: (message: string) => void;
            on: ((handle: (message: string) => void) => void) | ((message: string) => void);
        };
    }
}

function recvMessage(message: string) {
    console.log("message transport recv payload = ", message);

    try {
        const payload: Payload<unknown> = JSON.parse(message);

        if (payload.ty == "Response") {
            const { sequence, content } = payload.content as Response<unknown>;
            if (window.Route.listeners[sequence]) {
                window.Route.listeners[sequence](content);
            }
        } else {
            const { method } = payload.content as Events;
            if (window.Route.listeners[method]) {
                window.Route.listeners[method](undefined);
            }
        }
    } catch (e) {
        console.error(e);
    }
}

if (!window.Route) {
    window.Route = new RouteExt();

    if (window.MessageTransport) {
        if (cfg(TargetOs.Android)) {
            window.MessageTransport.on = recvMessage;
        } else {
            window.MessageTransport.on(recvMessage as any);
        }
    }
}
