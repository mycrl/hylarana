import { Backend, Settings as TSettings, VideoDecoder, VideoEncoder } from "../common";
import { accessSync, readFileSync, writeFileSync } from "node:fs";
import { faker } from "@faker-js/faker";
import { join } from "node:path";
import { app } from "electron";

const DefaultSettings: TSettings = {
    system_name: faker.person.fullName(),
    system_language: "english",
    system_renderer_backend: Backend.WebGPU,
    network_interface: "0.0.0.0",
    networkk_multicast: "239.0.0.1",
    network_server: null,
    network_port: 8080,
    network_mtu: 1500,
    codec_encoder: VideoEncoder.X264,
    codec_decoder: VideoDecoder.H264,
    video_size_width: 1280,
    video_size_height: 720,
    video_frame_rate: 24,
    video_bit_rate: 10000000,
    video_key_frame_interval: 24,
    audio_sample_rate: 48000,
    audio_bit_rate: 64000,
};

export const Settings: {
    path: string;
    value?: TSettings;
    get: () => TSettings;
    set: (value: TSettings) => void;
} = {
    path: join(app.getPath("userData"), "./settings.json"),
    value: undefined,
    get() {
        if (!this.value) {
            try {
                accessSync(this.path);
            } catch {
                writeFileSync(this.path, JSON.stringify(DefaultSettings));
            }

            this.value = JSON.parse(readFileSync(this.path, "utf8")) as TSettings;
        }

        return this.value;
    },
    set(value) {
        this.value = value;

        writeFileSync(this.path, JSON.stringify(value));
    },
};
