import {
    Device,
    getCaptureSources,
    getDevices,
    getStatus,
    onDevicesChange,
    onReceiverClose,
    onReceiverCreated,
    onSenderClose,
    onSenderCreated,
    SourceType,
    Status,
} from "./core";
import { atomWithStorage, createJSONStorage } from "jotai/utils";
import { atom, getDefaultStore } from "jotai";
import { DefaultSettings } from "./settings";
import { Languages } from "./locales";

const store = getDefaultStore();

function createLocalStorageStore<T>(defaultValue: T): [
    T,
    {
        getItem(key: string): T;
        setItem(key: string, value: T): void;
        removeItem(key: string): void;
    }
] {
    return [
        defaultValue,
        {
            getItem(key: string) {
                const value = localStorage[key];
                if (typeof defaultValue == "boolean") {
                    if (value == "true") {
                        return true;
                    } else {
                        return false;
                    }
                } else if (typeof defaultValue == "number") {
                    return Number(value);
                } else {
                    return value;
                }
            },

            setItem(key: string, value: T) {
                localStorage[key] = value;
            },

            removeItem(key: string) {
                localStorage.removeItem(key);
            },
        },
    ];
}

export const settingsAtom = atomWithStorage(
    "settings",
    DefaultSettings,
    createJSONStorage(() => localStorage),
    {
        getOnInit: true,
    }
);

export const deviceNameAtom = atom("");

{
    window.MessageTransport.getName().then((name) => {
        store.set(deviceNameAtom, name);

        store.sub(settingsAtom, () => {
            const value = store.get(deviceNameAtom);
            if (value != name) {
                window.MessageTransport.setName(value).then(() => {
                    name = value;
                });
            }
        });
    });
}

export const languageAtom = atomWithStorage<keyof typeof Languages>(
    "language",
    ...createLocalStorageStore("English" as keyof typeof Languages),
    {
        getOnInit: true,
    }
);

export const localesAtom = atom(Languages[store.get(languageAtom)]);

{
    store.sub(languageAtom, () => {
        const value = store.get(languageAtom);
        store.set(localesAtom, Languages[value]);
    });
}

export const devicesAtom = atom<Device[]>([]);

{
    getDevices().then((data) => {
        store.set(devicesAtom, data);
    });

    onDevicesChange(() => {
        getDevices().then((data) => {
            store.set(devicesAtom, data);
        });
    });
}

export const displaysAtom = atom(() => getCaptureSources(SourceType.Screen));

export const audiosAtom = atom(() => getCaptureSources(SourceType.Audio));

export const statusAtom = atom(Status.Idle);

{
    getStatus().then((value) => {
        store.set(statusAtom, value);
    });

    onReceiverClose(() => {
        store.set(statusAtom, Status.Idle);
    });

    onSenderClose(() => {
        store.set(statusAtom, Status.Idle);
    });

    onSenderCreated(() => {
        store.set(statusAtom, Status.Sending);
    });

    onReceiverCreated(() => {
        store.set(statusAtom, Status.Receiving);
    });
}
