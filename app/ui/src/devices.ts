import { useSyncExternalStore } from "react";
import { MessageRouter, Methods } from "./message";
import { ONCE } from "./utils";
import events from "./events";

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
    description: string | null;
}

export enum SourceType {
    Camera = "Camera",
    Screen = "Screen",
    Audio = "Audio",
}

export interface Source {
    id: string;
    index: number;
    is_default: boolean;
    kind: SourceType;
    name: string;
}

export const Devices = ONCE("devices", () => {
    let devices: Device[] = [];

    MessageRouter.call(Methods.GetDevices).then((list) => {
        {
            devices = list;
        }

        events.emit("devices.change");
    });

    MessageRouter.on(Methods.DevicesChangeNotify, async () => {
        {
            devices = await MessageRouter.call(Methods.GetDevices);
        }

        events.emit("devices.change");
    });

    return devices;
});

export function createDevicesStore() {
    return useSyncExternalStore(
        (callback) => {
            const sequence = events.on("devices.change", () => callback());
            return () => events.remove(sequence);
        },
        () => Devices
    );
}

export const Displays = ONCE("displays", () => {
    let sources: Source[] = [];

    MessageRouter.call(Methods.GetCaptureSources, SourceType.Screen).then((list) => {
        {
            sources = list;
        }

        events.emit("displays.change");
    });

    return sources;
});

export function createDisplaysStore() {
    return useSyncExternalStore(
        (callback) => {
            const sequence = events.on("displays.change", () => callback());
            return () => events.remove(sequence);
        },
        () => Displays
    );
}

export const Audios = ONCE("audios", () => {
    let sources: Source[] = [];

    MessageRouter.call(Methods.GetCaptureSources, SourceType.Audio).then((list) => {
        {
            sources = list;
        }

        events.emit("audios.change");
    });

    return sources;
});

export function createAudiosStore() {
    return useSyncExternalStore(
        (callback) => {
            const sequence = events.on("audios.change", () => callback());
            return () => events.remove(sequence);
        },
        () => Audios
    );
}
