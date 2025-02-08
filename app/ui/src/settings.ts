import { useSyncExternalStore } from "react";
import events from "./events";
import { ONCE } from "./utils";

export const VideoEncoders = {
    x264: "X264",
    qsv: "Intel QSV - Windows",
    videotoolbox: "VideoToolbox - Apple",
};

export const VideoDecoders = {
    h264: "H264",
    d3d11va: "D3D11VA - Windows",
    qsv: "Intel QSV - Windows",
    videotoolbox: "VideoToolbox - Apple",
};

export const DefaultSettings = {
    SystemDeviceName: Date.now().toString(),
    SystemLanguage: "English",
    SystemSenderBroadcast: false,
    NetworkInterface: "0.0.0.0",
    NetworkMulticast: "239.0.0.1",
    NetworkServer: null,
    NetworkMtu: 1500,
    CodecEncoder: "x264",
    CodecDecoder: "h264",
    VideoSizeWidth: 1280,
    VideoSizeHeight: 720,
    VideoFrameRate: 24,
    VideoBitRate: 10000000,
    VideoKeyFrameInterval: 24,
    AudioSampleRate: 48000,
    AudioBitRate: 64000,
};

export type SettingsType = typeof DefaultSettings;

export const Settings: SettingsType = ONCE("settings", () => {
    if (!localStorage.Settings) {
        localStorage.Settings = JSON.stringify(DefaultSettings);
    }

    return JSON.parse(localStorage.Settings);
});

export function setSettings(value: SettingsType) {
    {
        localStorage.Settings = JSON.stringify(value);
    }

    Object.assign(Settings, value);
    events.emit("settings.change");
}

export default function () {
    return useSyncExternalStore(
        (callback) => {
            const sequence = events.on("settings.change", () => callback());
            return () => events.remove(sequence);
        },
        () => Settings
    );
}
