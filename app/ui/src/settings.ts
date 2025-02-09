import { atomWithStorage, createJSONStorage } from "jotai/utils";
import { MessageRouter, Methods } from "./message";
import { LanguageOptions } from "./locales";
import { useSetAtom } from "jotai";

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
    SystemDeviceName: string;
    SystemLanguage: keyof typeof LanguageOptions;
    SystemSenderBroadcast: boolean;
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
    SystemDeviceName: "",
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

export const settingsAtom = atomWithStorage(
    "settings",
    DefaultSettings,
    createJSONStorage(() => localStorage),
    {
        getOnInit: true,
    }
);

{
    MessageRouter.call(Methods.GetName).then((SystemDeviceName) => {
        useSetAtom(settingsAtom)((prev) => ({
            ...prev,
            SystemDeviceName,
        }));
    });
}
