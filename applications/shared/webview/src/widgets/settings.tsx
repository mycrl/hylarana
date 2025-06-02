import "../styles/settings.css";

import { useEffect, useState } from "react";
import { useAtom, useAtomValue } from "jotai";

import Codec from "./settings.codec";
import Video from "./settings.video";
import Audio from "./settings.audio";
import System from "./settings.system";
import Network from "./settings.network";

import { localesAtom, settingsAtom, statusAtom } from "../state";
import { Settings, Status } from "../bridge";
import { cfg, TargetOs } from "../utils";

export type RefHandle<T> = {
    value: T;
    get: () => T;
    set: (value: T) => void;
};

export type Ref<T> = {
    [P in keyof T]: RefHandle<T[P]>;
};

export type SettingsRef = {
    [P in keyof Settings]: Ref<Settings[P]>;
};

class SettingsObject {
    static create(object: any): SettingsRef {
        let ref = {} as any;
        for (const key of Object.keys(object)) {
            if (!ref[key]) {
                ref[key] = {};
            }

            for (const k of Object.keys(object[key])) {
                Object.assign(ref[key], {
                    [k]: {
                        value: object[key][k],
                        get() {
                            return this.value;
                        },
                        set(value: any) {
                            this.value = value;
                        },
                    },
                });
            }
        }

        return ref;
    }

    static freeze(ref: any): Settings {
        let values = {} as any;
        for (const key of Object.keys(ref)) {
            if (!values[key]) {
                values[key] = {};
            }

            for (const k of Object.keys(ref[key])) {
                values[key][k] = ref[key][k].value;
            }
        }

        return values;
    }
}

export default function ({
    offscreen,
    style,
}: {
    offscreen: boolean;
    style?: React.CSSProperties;
}) {
    const status = useAtomValue(statusAtom);
    const locales = useAtomValue(localesAtom);
    const [settings_, setSettings] = useAtom(settingsAtom);
    const [disabled, setDisabled] = useState(status != Status.Idle);

    useEffect(() => {
        setDisabled(status != Status.Idle);
    }, [offscreen, status]);

    const settings = SettingsObject.create(settings_);
    return (
        <>
            <div className='Settings' style={style}>
                {!offscreen && (
                    <div className='Content'>
                        <Video locales={locales} settings={settings} disabled={disabled} />
                        {cfg(TargetOs.Windows) && (
                            <Codec locales={locales} settings={settings} disabled={disabled} />
                        )}
                        {!cfg(TargetOs.Android) && (
                            <Audio locales={locales} settings={settings} disabled={disabled} />
                        )}
                        <System locales={locales} settings={settings} disabled={disabled} />
                        <Network locales={locales} settings={settings} disabled={disabled} />
                    </div>
                )}
                {!disabled && (
                    <button
                        id='Apply'
                        className='click'
                        onClick={() => {
                            setSettings(SettingsObject.freeze(settings)).then(() => {
                                setDisabled(true);
                            });
                        }}
                    >
                        {locales.Apply}
                    </button>
                )}
            </div>
        </>
    );
}
