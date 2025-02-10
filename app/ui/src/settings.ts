import { atomWithStorage, createJSONStorage } from "jotai/utils";
import { Route, Methods } from "./message";
import { atom, getDefaultStore } from "jotai";

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

export interface SettingsType {
    NetworkInterface: string;
    NetworkMulticast: string;
    NetworkServer: string | null;
    NetworkMtu: number;
    CodecEncoder: keyof typeof VideoEncoders;
    CodecDecoder: keyof typeof VideoDecoders;
    VideoSizeWidth: number;
    VideoSizeHeight: number;
    VideoFrameRate: number;
    VideoBitRate: number;
    VideoKeyFrameInterval: number;
    AudioSampleRate: number;
    AudioBitRate: number;
}

export const DefaultSettings: SettingsType = {
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

export const settingsAtom = atomWithStorage(
    "settings",
    DefaultSettings,
    createJSONStorage(() => localStorage),
    {
        getOnInit: true,
    }
);

export const deviceNameAtom = atom("");

{
    const store = getDefaultStore();

    Route.call(Methods.GetName).then((name) => {
        store.set(deviceNameAtom, name);

        store.sub(settingsAtom, () => {
            const value = store.get(deviceNameAtom);
            if (value != name) {
                Route.call(Methods.SetName, value).then(() => {
                    name = value;
                });
            }
        });
    });
}

export const broadcastAtom = atomWithStorage<boolean>(
    "broadcast",
    false,
    {
        getItem(key) {
            return localStorage[key] == "true";
        },
        setItem(key, value) {
            localStorage[key] = value;
        },
        removeItem(key) {
            localStorage.removeItem(key);
        },
    },
    {
        getOnInit: true,
    }
);
