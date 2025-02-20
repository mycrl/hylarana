import { Backend, Settings as SettingsType, VideoDecoder, VideoEncoder } from "./hylarana";
import { accessSync, readFileSync, writeFileSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { faker } from "@faker-js/faker";
import { join } from "node:path";
import { app } from "electron";

/**
 * Stored by default within the settings.json file in the application data
 * directory.
 */
const PATH = join(app.getPath("userData"), "./settings.json");

console.log("init settings file, path = ", PATH);

/**
 * Checks if the configuration file already exists, and creates a default
 * configuration file if it does not.
 */
{
    try {
        accessSync(PATH);
    } catch {
        writeFileSync(
            PATH,
            JSON.stringify(
                {
                    system: {
                        name: faker.person.fullName(),
                        language: "english",
                        backend: Backend.WebGPU,
                    },
                    network: {
                        interface: "0.0.0.0",
                        multicast: "239.0.0.1",
                        server: null,
                        port: 8080,
                        mtu: 1500,
                    },
                    codec: {
                        encoder: VideoEncoder.X264,
                        decoder: VideoDecoder.H264,
                    },
                    video: {
                        width: 1280,
                        height: 720,
                        frame_rate: 24,
                        bit_rate: 10000000,
                        key_frame_interval: 24,
                    },
                    audio: {
                        sample_rate: 48000,
                        bit_rate: 64000,
                    },
                },
                null,
                4
            )
        );
    }
}

export let SettingsValue = JSON.parse(readFileSync(PATH, "utf8")) as SettingsType;

/**
 * Because of testing needs, it is sometimes necessary to randomise a device
 * name each time it is started.
 */
if (process.env.RANDOM_NAME) {
    SettingsValue.system.name = faker.person.fullName();
}

export namespace Settings {
    export async function update(value: SettingsType) {
        await writeFile(PATH, JSON.stringify(value, null, 4));
        SettingsValue = value;
    }
}
