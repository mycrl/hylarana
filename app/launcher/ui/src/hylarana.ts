import { Route, Methods } from "./message";

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

export type TransportStrategyStruct = {
    [key in TransportStrategy]: string;
};

export interface Transport {
    strategy: TransportStrategyStruct;
    mtu: number;
}

export interface SenderOptions {
    transport: Transport;
    media: {
        video: {
            source: Source;
            options: {
                codec: keyof typeof VideoEncoders;
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

export interface MediaStreamDescription {
    id: string;
    transport: Transport;
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

export interface ReceiverOptions {
    video_decoder: keyof typeof VideoDecoders;
}

export enum Backend {
    Direct3D11 = "Direct3D11",
    WebGPU = "WebGPU",
}

export enum Status {
    Sending = "Sending",
    Receiving = "Receiving",
    Idle = "Idle",
}

export const events = new EventTarget();

export async function onDevicesChange(listener: () => void) {
    return await Route.on(Methods.DevicesChangeNotify, listener);
}

export async function getDevices() {
    return await Route.call(Methods.GetDevices);
}

export async function createSender(addrs: string[], options: SenderOptions) {
    await Route.call(Methods.CreateSender, [addrs, options]);
    events.dispatchEvent(new Event("senderCreated"));
}

export async function getCaptureSources(type: SourceType) {
    return await Route.call(Methods.GetCaptureSources, type);
}

export async function getStatus() {
    return await Route.call(Methods.GetStatus);
}

export async function closeSender() {
    return await Route.call(Methods.CloseSender);
}

export async function closeReceiver() {
    return await Route.call(Methods.CloseReceiver);
}

export async function onSenderClose(listener: () => void) {
    return await Route.on(Methods.SenderClosedNotify, listener);
}

export async function onReceiverClose(listener: () => void) {
    return await Route.on(Methods.ReceiverClosedNotify, listener);
}

export async function createReceiver(
    options: ReceiverOptions,
    backend: Backend,
    description: MediaStreamDescription
) {
    await Route.call(Methods.CreateReceiver, [options, backend, description]);
    events.dispatchEvent(new Event("receiverCreated"));
}
