import {
    createReceiver,
    getCaptureSources,
    getDevices,
    getSettings,
    getStatus,
    onDevicesChange,
    onStatusChange,
    setSettings,
} from "./core";
import { atom, getDefaultStore } from "jotai";
import { atomWithStorage } from "jotai/utils";

import english from "./assets/locales/english.json";
import chinase from "./assets/locales/chinase.json";
import { Device, Settings, SourceType, Status } from "./bridge";

const store = getDefaultStore();

const localSettingsAtom = atom<Settings | null>(null);

export const settingsAtom = atom(
    async (get) => {
        let value = get(localSettingsAtom);
        if (value == null) {
            value = await getSettings();
            store.set(localSettingsAtom, value);
        }

        return value;
    },
    async (_, set, value: Settings) => {
        await setSettings(value);
        set(localSettingsAtom, value);
    }
);

export const localesAtom = atom(async (get) => {
    return getLocales((await get(settingsAtom)).system.language);
});

export const devicesAtom = atom<Device[]>([]);

getDevices().then((devices) => {
    store.set(devicesAtom, devices);
});

onDevicesChange(() => {
    getDevices().then((devices) => {
        store.set(devicesAtom, devices);

        /**
         * If Auto Allow is turned on, the first device being sent is automatically
         * selected from the device list each time the device list changes.
         */
        if (store.get(autoAllowAtom) && store.get(statusAtom) == Status.Idle) {
            for (const item of devices) {
                if (item.metadata) {
                    const settings = store.get(localSettingsAtom);
                    if (settings) {
                        createReceiver(item, settings);
                    }

                    break;
                }
            }
        }
    });
});

export const displaysAtom = atom(() => {
    return getCaptureSources(SourceType.Screen);
});

export const audiosAtom = atom(() => {
    return getCaptureSources(SourceType.Audio);
});

export const timerAtom = atom("00:00:00");

export const statusAtom = atom(Status.Idle);

let timer: any | null = null;

onStatusChange(async () => {
    const status = await getStatus();
    store.set(statusAtom, status);

    if (status == Status.Sending || status == Status.Receiving) {
        let seconds = 0;

        timer = setInterval(() => {
            seconds += 1;

            store.set(
                timerAtom,
                [Math.floor(seconds / 3600), Math.floor((seconds % 3600) / 60), seconds % 60]
                    .map((v) => String(v).padStart(2, "0"))
                    .join(":")
            );
        }, 1000);
    } else {
        store.set(timerAtom, "00:00:00");
        clearInterval(timer);
    }
});

export const autoAllowAtom = atomWithStorage(
    "auto-allow",
    false,
    {
        getItem(key, initialValue) {
            let value = localStorage.getItem(key);
            if (value == null) {
                const item = initialValue ? "1" : "0";
                localStorage.setItem(key, item);
                value = item;
            }

            return value == "1";
        },
        setItem(key, value) {
            localStorage.setItem(key, value ? "1" : "0");
        },
        removeItem(key) {
            localStorage.removeItem(key);
        },
    },
    {
        getOnInit: true,
    }
);

export const LanguageOptions = {
    chinase: "简体中文",
    english: "English",
};

export type Language = typeof english;

export function getLocales(lang: string): Language {
    return { english, chinase }[lang as "chinase" | "english"];
}
