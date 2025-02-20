import {
    Settings,
    SourceType,
    RequestBounds,
    EventBounds,
    Methods,
    TransportStrategy,
    Source,
    Device,
} from "@/hylarana";

declare class Route {
    request<
        K extends keyof RequestBounds,
        Q extends RequestBounds[K][0],
        S extends RequestBounds[K][1]
    >(method: K, req?: Q): Promise<S>;

    on<K extends keyof EventBounds, Q extends EventBounds[K][0], S extends EventBounds[K][1]>(
        method: string,
        handle: (request: Q) => Promise<S> | void
    ): void;
}

declare global {
    interface Window {
        Route: Route;
    }
}

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
    window.Route.on(Methods.DevicesChangeNotify, listener);
}

export async function getDevices() {
    return await window.Route.request(Methods.GetDevices);
}

export async function setName(name: string) {
    return await window.Route.request(Methods.SetName, name);
}

export async function createSender(
    addrs: string[],
    transport: TransportStrategy,
    display: Source,
    audio: Source,
    settings: Settings
) {
    return await window.Route.request(Methods.CreateSender, [
        addrs,
        {
            transport: {
                mtu: settings.network.mtu,
                strategy: {
                    ty: transport,
                    address:
                        ({
                            [TransportStrategy.Relay]: settings.network.server,
                            [TransportStrategy.Direct]: settings.network.interface,
                            [TransportStrategy.Multicast]: settings.network.multicast,
                        }[transport] as string) +
                        ":" +
                        settings.network.port,
                },
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
    ]);
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
    window.Route.on(Methods.StatusChangeNotify, listener);
}

export async function createReceiver(device: Device, settings: Settings) {
    let description = Object.assign({}, device.description!);
    
    /**
     * The sender, if using passthrough, will need to replace the ip in the publish 
     * address by replacing the ip address with the sender's ip.
     */
    if (description.transport.strategy.ty == TransportStrategy.Direct) {
        description.transport.strategy.address = `${device.ip}:${
            description.transport.strategy.address.split(":")[1]
        }`;
    }

    return await window.Route.request(Methods.CreateReceiver, [
        settings.codec.decoder,
        settings.system.backend,
        description,
    ]);
}

export async function getSettings() {
    return await window.Route.request(Methods.GetSettings);
}

export async function setSettings(value: Settings) {
    return await window.Route.request(Methods.SetSettings, value);
}
