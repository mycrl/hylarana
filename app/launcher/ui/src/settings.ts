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
