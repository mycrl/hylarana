import { ref, computed } from "vue";

const Chinase = {
    System: "系统",
    Language: "语言",
    Settings: "设置",
    Sender: "发送",
    Receiver: "接收",
    Broadcast: "广播",
    BroadcastTips: "启用广播会自动发送给所有接收端",
    AutoAllow: "自动允许",
    AutoAllowTips: "无需手动确认即可自动接收投屏",
    Network: "网络",
    NetworkInterface: "接口",
    NetworkInterfaceTips: "绑定的网卡接口，0.0.0.0表示所有网卡都绑定。",
    NetworkMtu: "最大传输单元",
    NetworkMtuTips:
        "最大传输单元（英语：Maximum Transmission Unit，缩写MTU）是指数据链路层上面所能通过的最大数据包大小（以字节为单位）",
    Codec: "编解码器",
    CodecDecoder: "解码器",
    CodecDecoderTips: "视频解码器，H264是兼容性最好的软件解码器。",
    CodecEncoder: "编码器",
    CodecEncoderTips: "视频编码器，X264是兼容性最好的软件编码器。",
    Video: "视频",
    VideoSize: "尺寸",
    VideoSizeTips: "发送方视频的宽度和高度。",
    VideoFrameRate: "刷新率",
    VideoFrameRateTips: "视频的刷新率通常为24/30/60。",
    VideoBitRateTips: "视频流的比特率，以bit/s为单位。",
    VideoKeyFrameInterval: "关键帧间隔",
    VideoKeyFrameIntervalTips: "建议关键帧间隔与视频帧率保持一致，有利于减小视频流的大小。",
    Audio: "音频",
    AudioSampleRate: "采样率",
    AudioSampleRateTips: "音频采样率建议为48Khz。",
    AudioBitRateTips: "音频流的比特率，以比特/秒为单位。",
    BitRate: "比特率",
    Apply: "应用",
    BackToHome: "返回",
};

const English = {
    System: "system",
    Language: "language",
    Settings: "Settings",
    Sender: "Sender",
    Receiver: "Receiver",
    Broadcast: "Broadcast",
    BroadcastTips: "enable broadcast is automatically sent to all recipients",
    AutoAllow: "Auto Allow",
    AutoAllowTips: "no manual confirmation is required to receive the screen cast automatically",
    Network: "network",
    NetworkInterface: "interface",
    NetworkInterfaceTips: "Bound NIC interfaces, 0.0.0.0 means all NICs are bound.",
    NetworkMtu: "mtu",
    NetworkMtuTips:
        "In computer networking, the maximum transmission unit (MTU) is the size of the largest protocol data unit (PDU) that can be communicated in a single network layer transaction.",
    Codec: "codec",
    CodecDecoder: "decoder",
    CodecDecoderTips: "Video decoder, H264 is a software decoder with the best compatibility.",
    CodecEncoder: "encoder",
    CodecEncoderTips: "Video encoder, X264 is a software encoder with the best compatibility.",
    Video: "video",
    VideoSize: "size",
    VideoSizeTips: "The width and height of the video on the sender side.",
    VideoFrameRate: "frame rate",
    VideoFrameRateTips: "The refresh rate of the video is usually 24 / 30 / 60.",
    VideoBitRateTips: "The bit rate of the video stream, in bit/s.",
    VideoKeyFrameInterval: "key frame interval",
    VideoKeyFrameIntervalTips:
        "It is recommended that the key frame interval be consistent with the video frame rate, which helps reduce the size of the video stream.",
    Audio: "audio",
    AudioSampleRate: "sample rate",
    AudioSampleRateTips: "The audio sampling rate is recommended to be 48Khz.",
    AudioBitRateTips: "The bit rate of the audio stream, in bit/s.",
    BitRate: "bit rate",
    Apply: "apply",
    BackToHome: "Back to Home",
};

export const Language = ref(localStorage.Language);

export const LanguageMapping = {
    chinase: Chinase,
    english: English,
};

export function setLanguage(value) {
    localStorage.Language = value;
    Language.value = value;
}

export const I18n = computed(() => LanguageMapping[Language.value]);
