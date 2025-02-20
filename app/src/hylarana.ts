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
    X264 = "X264",
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
    H264 = "H264",
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
 * Transport layer strategies.
 */
export enum TransportStrategy {
    /**
     * In straight-through mode, the sender creates an SRT server and the
     * receiver connects directly to the sender via the SRT protocol.
     *
     * For the sender, the network address is the address to which the SRT
     * server binds and listens.
     *
     * ```text
     * example: 0.0.0.0:8080
     * ```
     *
     * For the receiving end, the network address is the address of the SRT
     * server on the sending end.
     *
     * ```text
     * example: 192.168.1.100:8080
     * ```
     */
    Direct = "Direct",
    /**
     * Forwarding mode, where the sender and receiver pass data through a relay
     * server.
     *
     * The network address is the address of the transit server.
     */
    Relay = "Relay",
    /**
     * UDP multicast mode, where the sender sends multicast packets into the
     * current network and the receiver processes the multicast packets.
     *
     * The sender and receiver use the same address, which is a combination of
     * multicast address + port.
     *
     * ```text
     * example: 239.0.0.1:8080
     * ```
     */
    Multicast = "Multicast",
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

export interface Device {
    ip: string;
    name: string;
    kind: DeviceType;
    description: MediaStreamDescription | null;
}

/**
 * Video source type or Audio source type.
 */
export enum SourceType {
    /**
     * Camera or video capture card and other devices (and support virtual
     * camera)
     */
    Camera = "Camera",
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

export enum Backend {
    Direct3D11 = "Direct3D11",
    WebGPU = "WebGPU",
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
    strategy: {
        ty: TransportStrategy;
        address: string;
    };
    /**
     * see: [Maximum_transmission_unit](https://en.wikipedia.org/wiki/Maximum_transmission_unit)
     */
    mtu: number;
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

export interface SystemSettings {
    name: string;
    language: "english" | "chinase";
    backend: Backend;
}

export interface NetWorkSettings {
    interface: string;
    multicast: string;
    server: string | null;
    port: number;
    mtu: number;
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

export enum Methods {
    SetName = "SetName",
    GetDevices = "GetDevices",
    DevicesChangeNotify = "DevicesChangeNotify",
    GetCaptureSources = "GetCaptureSources",
    CreateSender = "CreateSender",
    CloseSender = "CloseSender",
    CreateReceiver = "CreateReceiver",
    CloseReceiver = "CloseReceiver",
    GetStatus = "GetStatus",
    ReadyNotify = "ReadyNotify",
    StatusChangeNotify = "StatusChangeNotify",
    GetSettings = "GetSettings",
    SetSettings = "SetSettings",
}

export interface RequestBounds {
    [Methods.GetDevices]: [void, Device[]];
    [Methods.GetCaptureSources]: [SourceType, Source[]];
    [Methods.CreateSender]: [[Array<string>, SenderOptions], void];
    [Methods.CloseSender]: [void, void];
    [Methods.CreateReceiver]: [[VideoDecoder, Backend, MediaStreamDescription], void];
    [Methods.CloseReceiver]: [void, void];
    [Methods.GetStatus]: [void, Status];
    [Methods.SetName]: [string, void];
    [Methods.GetSettings]: [void, Settings];
    [Methods.SetSettings]: [Settings, void];
}

export interface EventBounds {
    [Methods.ReadyNotify]: [void, void];
    [Methods.DevicesChangeNotify]: [void, void];
    [Methods.StatusChangeNotify]: [void, void];
}
