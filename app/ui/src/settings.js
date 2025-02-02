import { ref } from "vue";

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

export const DefaultSystemSender = {
    broadcast: false,
};

export const DefaultSystem = {
    language: "english",
    sender: DefaultSystemSender,
};

export const DefaultNetwork = {
    interface: "0.0.0.0",
    multicast: "239.0.0.1",
    server: null,
    mtu: 1500,
};

export const DefaultCodec = {
    encoder: "x264",
    decoder: "h264",
};

export const DefaultVideoSize = {
    width: 1280,
    height: 720,
};

export const DefaultVideo = {
    size: DefaultVideoSize,
    frameRate: 24,
    bitRate: 10000000,
    keyFrameInterval: 24,
};

export const DefaultAudio = {
    sampleRate: 48000,
    bitRate: 64000,
};

export const DefaultSettings = {
    system: DefaultSystem,
    network: DefaultNetwork,
    codec: DefaultCodec,
    video: DefaultVideo,
    audio: DefaultAudio,
};

if (!localStorage.Settings) {
    localStorage.setItem("Settings", JSON.stringify(DefaultSettings));
}

const Settings = ref(JSON.parse(localStorage.getItem("Settings")));

export default Settings;

export function updateSettings() {
    localStorage.setItem("Settings", JSON.stringify(Settings.value));
}
