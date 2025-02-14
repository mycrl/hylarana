export enum VideoEncoder {
    X264 = "X264",
    Qsv = "Qsv",
    VideoToolBox = "VideoToolBox",
}

export enum VideoDecoder {
    H264 = "H264",
    D3D11 = "D3D11",
    Qsv = "Qsv",
    VideoToolBox = "VideoToolBox",
}

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

export enum Backend {
    Direct3D11 = "Direct3D11",
    WebGPU = "WebGPU",
}

export enum Status {
    Sending = "Sending",
    Receiving = "Receiving",
    Idle = "Idle",
}

export interface Source {
    id: string;
    index: number;
    is_default: boolean;
    kind: SourceType;
    name: string;
}

export interface Transport {
    strategy: {
        ty: TransportStrategy;
        address: string;
    };
    mtu: number;
}

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

export interface Settings {
    system_name: string;
    system_language: string;
    system_renderer_backend: Backend;
    network_interface: string;
    networkk_multicast: string;
    network_server: string | null;
    network_port: number;
    network_mtu: number;
    codec_encoder: VideoEncoder;
    codec_decoder: VideoDecoder;
    video_size_width: number;
    video_size_height: number;
    video_frame_rate: number;
    video_bit_rate: number;
    video_key_frame_interval: number;
    audio_sample_rate: number;
    audio_bit_rate: number;
}
