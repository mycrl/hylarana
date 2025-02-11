import { atomWithStorage, createJSONStorage } from "jotai/utils";
import { atom, getDefaultStore } from "jotai";
import { Backend, VideoDecoders, VideoEncoders } from "./hylarana";

export interface SettingsType {
    NetworkInterface: string;
    NetworkMulticast: string;
    NetworkServer: string | null;
    NetworkPort: number;
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
    RendererBackend: Backend;
}

export const DefaultSettings: SettingsType = {
    NetworkInterface: "0.0.0.0",
    NetworkMulticast: "239.0.0.1",
    NetworkServer: null,
    NetworkPort: 8080,
    NetworkMtu: 1500,
    CodecEncoder: "X264",
    CodecDecoder: "H264",
    VideoSizeWidth: 1280,
    VideoSizeHeight: 720,
    VideoFrameRate: 24,
    VideoBitRate: 10000000,
    VideoKeyFrameInterval: 24,
    AudioSampleRate: 48000,
    AudioBitRate: 64000,
    RendererBackend: Backend.WebGPU,
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

    window.MessageTransport.getName().then((name) => {
        store.set(deviceNameAtom, name);

        store.sub(settingsAtom, () => {
            const value = store.get(deviceNameAtom);
            if (value != name) {
                window.MessageTransport.setName(value).then(() => {
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
