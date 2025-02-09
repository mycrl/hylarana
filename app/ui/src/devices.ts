import { atom, useSetAtom } from "jotai";
import { MessageRouter, Methods } from "./message";

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

export const DevicesAtom = atom<Device[]>([]);

MessageRouter.call(Methods.GetDevices).then((list) => {
    useSetAtom(DevicesAtom)(() => list);
});

MessageRouter.on(Methods.DevicesChangeNotify, async () => {
    MessageRouter.call(Methods.GetDevices).then((list) => {
        useSetAtom(DevicesAtom)(() => list);
    });
});

export const DisplaysAtom = atom<Source[]>([]);

MessageRouter.call(Methods.GetCaptureSources, SourceType.Screen).then((list) => {
    useSetAtom(DisplaysAtom)(() => list);
});

export const AudiosAtom = atom<Source[]>([]);

{
    MessageRouter.call(Methods.GetCaptureSources, SourceType.Audio).then((list) => {
        useSetAtom(AudiosAtom)(() => list);
    });
}
