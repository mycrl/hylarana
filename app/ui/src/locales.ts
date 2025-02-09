import { useSyncExternalStore } from "react";
import events from "./events";
import { Settings } from "./settings";

const Chinase = {
    DeviceName: "设备名称",
    Type: "类型",
    System: "系统",
    Language: "语言",
    Settings: "设置",
    Sender: "发送",
    Receiver: "接收",
    Broadcast: "广播",
    BroadcastHelp: "启用广播会自动发送给所有接收端",
    AutoAllow: "自动允许",
    AutoAllowHelp: "无需手动确认即可自动接收投屏",
    Network: "网络",
    NetworkInterface: "接口",
    NetworkInterfaceHelp: "绑定的网卡接口，0.0.0.0表示所有网卡都绑定。",
    NetworkMulticast: "多播",
    NetworkMulticastHelp: "用于多播的IP地址，默认为239.0.0.1。",
    NetworkServer: "服务器",
    NetworkServerHelp: "转发服务器的地址，例如192.168.1.100:8080。",
    NetworkMtu: "最大传输单元",
    NetworkMtuHelp:
        "最大传输单元（英语：Maximum Transmission Unit，缩写MTU）是指数据链路层上面所能通过的最大数据包大小（以字节为单位）",
    Codec: "编解码器",
    CodecDecoder: "解码器",
    CodecDecoderHelp: "视频解码器，H264是兼容性最好的软件解码器。",
    CodecEncoder: "编码器",
    CodecEncoderHelp: "视频编码器，X264是兼容性最好的软件编码器。",
    Video: "视频",
    VideoSize: "宽高",
    VideoSizeHelp: "发送方视频的宽度和高度。",
    VideoFrameRate: "刷新率",
    VideoFormat: "格式",
    VideoFrameRateHelp: "视频的刷新率通常为24/30/60。",
    VideoBitRateHelp: "视频流的比特率，以bit/s为单位。",
    VideoKeyFrameInterval: "关键帧间隔",
    VideoKeyFrameIntervalHelp: "建议关键帧间隔与视频帧率保持一致，有利于减小视频流的大小。",
    Audio: "音频",
    AudioChannel: "通道",
    AudioSampleBit: "精度",
    AudioSampleRate: "采样率",
    AudioSampleRateHelp: "音频采样率建议为48Khz。",
    AudioBitRateHelp: "音频流的比特率，以比特/秒为单位。",
    AudioStereo: "立体声",
    BitRate: "比特率",
    Apply: "应用",
    BackToHome: "返回",
    Devices: "设备列表",
    DevicesHelp: "查找到的所有可以投屏的设备",
    DevicesReceiverHelp: "查找到的所有正在投屏的设备",
    DevicesSearchHelp: "正在查找可用设备...",
    Direct: "直连",
    Relay: "转发",
    Multicast: "多播",
    SenderStart: "开始投屏",
};

const English = {
    DeviceName: "device name",
    Type: "type",
    System: "system",
    Language: "language",
    Settings: "Settings",
    Sender: "Sender",
    Receiver: "Receiver",
    Broadcast: "Broadcast",
    BroadcastHelp: "enable broadcast is automatically sent to all recipients",
    AutoAllow: "Auto Allow",
    AutoAllowHelp: "no manual confirmation is required to receive the screen cast automatically",
    Network: "network",
    NetworkInterface: "interface",
    NetworkInterfaceHelp: "Bound NIC interfaces, 0.0.0.0 means all NICs are bound.",
    NetworkMulticast: "multicast",
    NetworkMulticastHelp: "The IP address used for multicast, the default is 239.0.0.1.",
    NetworkServer: "server",
    NetworkServerHelp: "The address of the forwarding server, such as 192.168.1.100:8080.",
    NetworkMtu: "mtu",
    NetworkMtuHelp:
        "In computer networking, the maximum transmission unit (MTU) is the size of the largest protocol data unit (PDU) that can be communicated in a single network layer transaction.",
    Codec: "codec",
    CodecDecoder: "decoder",
    CodecDecoderHelp: "Video decoder, H264 is a software decoder with the best compatibility.",
    CodecEncoder: "encoder",
    CodecEncoderHelp: "Video encoder, X264 is a software encoder with the best compatibility.",
    Video: "video",
    VideoSize: "size",
    VideoSizeHelp: "The width and height of the video on the sender side.",
    VideoFrameRate: "frame rate",
    VideoFormat: "format",
    VideoFrameRateHelp: "The refresh rate of the video is usually 24 / 30 / 60.",
    VideoBitRateHelp: "The bit rate of the video stream, in bit/s.",
    VideoKeyFrameInterval: "key frame interval",
    VideoKeyFrameIntervalHelp:
        "It is recommended that the key frame interval be consistent with the video frame rate, which helps reduce the size of the video stream.",
    Audio: "audio",
    AudioChannel: "channel",
    AudioSampleBit: "sample bit",
    AudioSampleRate: "sample rate",
    AudioSampleRateHelp: "The audio sampling rate is recommended to be 48Khz.",
    AudioBitRateHelp: "The bit rate of the audio stream, in bit/s.",
    AudioStereo: "stereo",
    BitRate: "bit rate",
    Apply: "apply",
    BackToHome: "Back to Home",
    Devices: "devices",
    DevicesHelp: "all devices that can be screen cast are found.",
    DevicesReceiverHelp: "All devices that are casting the screen.",
    DevicesSearchHelp: "Searching for available devices...",
    Direct: "Direct",
    Relay: "Relay",
    Multicast: "Multicast",
    SenderStart: "Start",
};

export type Language = typeof English;
export const Languages = { Chinase, English };
export const LanguageOptions = {
    Chinase: "简体中文",
    English: "English",
};

export function languageChange() {
    events.emit("language.change");
}

export function createLocalesStore() {
    return useSyncExternalStore(
        (callback) => {
            const sequence = events.on("language.change", () => callback());
            return () => events.remove(sequence);
        },
        () => Languages[Settings.SystemLanguage as keyof typeof Languages]
    );
}
