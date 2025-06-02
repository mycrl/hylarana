import { Device, EventMethods, Methods, Settings, Source, SourceType } from "./bridge";

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

export function onDevicesChange(listener: () => void) {
    window.Route.on(EventMethods.DevicesChangeNotify, listener);
}

export async function getDevices() {
    return await window.Route.request(Methods.GetDevices);
}

export async function createSender(
    targets: string[],
    display: Source,
    audio: Source,
    settings: Settings
) {
    return await window.Route.request(Methods.CreateSender, {
        bind: `${settings.network.bind.split(":")[0]}:0`,
        targets,
        options: {
            transport: {
                mtu: settings.network.mtu,
                max_bandwidth: settings.network.max_bandwidth,
                timeout: settings.network.timeout,
                fec: settings.network.fec,
                fc: settings.network.fc,
                latency: settings.network.latency,
            },
            media: {
                video: {
                    source: display,
                    options: {
                        codec: settings.codec.encoder,
                        frame_rate: settings.video.frame_rate,
                        width: settings.video.width,
                        height: settings.video.height,
                        bit_rate: settings.video.bit_rate,
                        key_frame_interval: settings.video.key_frame_interval,
                    },
                },
                audio: {
                    source: audio,
                    options: {
                        sample_rate: settings.audio.sample_rate,
                        bit_rate: settings.audio.bit_rate,
                    },
                },
            },
        },
    });
}

export async function getCaptureSources(type: SourceType) {
    return await window.Route.request(Methods.GetCaptureSources, type);
}

export async function getStatus() {
    return await window.Route.request(Methods.GetStatus);
}

export async function closeSender() {
    return await window.Route.request(Methods.CloseSender);
}

export async function closeReceiver() {
    return await window.Route.request(Methods.CloseReceiver);
}

export function onStatusChange(listener: () => void) {
    window.Route.on(EventMethods.StatusChangeNotify, listener);
}

export async function createReceiver(device: Device, settings: Settings) {
    let description = Object.assign({}, device.metadata!.description);

    return await window.Route.request(Methods.CreateReceiver, {
        addr: `${device.ip}:${device.metadata!.port}`,
        options: {
            transport: {
                mtu: settings.network.mtu,
                max_bandwidth: settings.network.max_bandwidth,
                timeout: settings.network.timeout,
                fec: settings.network.fec,
                fc: settings.network.fc,
                latency: settings.network.latency,
            },
            codec: settings.codec.decoder,
        },
        description,
    });
}

export async function getSettings() {
    return await window.Route.request(Methods.GetSettings);
}

export async function setSettings(value: Settings) {
    return await window.Route.request(Methods.SetSettings, value);
}
