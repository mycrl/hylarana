import "../styles/main.settings.css";
import { useState } from "react";
import {
    settingsAtom,
    VideoDecoders,
    VideoEncoders,
    DefaultSettings,
    SettingsType,
} from "../settings";
import { localesAtom, LanguageOptions, Languages, setLanguage } from "../locales";
import { MessageRouter, Methods } from "../message";
import { useAtom, useSetAtom } from "jotai";

function Input<T extends string | number | null>({
    ref,
    disabled,
}: {
    disabled: boolean;
    ref: RefHandle<T>;
}) {
    const [value, setValue] = useState(ref.get());
    const isNumber = typeof value == "number";

    return (
        <>
            <input
                type={isNumber ? "number" : "text"}
                value={value || ""}
                disabled={disabled}
                onChange={({ target }) => {
                    const value =
                        target.value.length == 0
                            ? null
                            : isNumber
                            ? Number(target.value)
                            : target.value;

                    setValue(value as T);
                    ref.set(value as T);
                }}
            />
        </>
    );
}

function Select<T extends string | number>({
    ref,
    options,
    disabled,
    onChange,
}: {
    disabled: boolean;
    ref: RefHandle<T>;
    options: { [k: string]: T };
    onChange?: (v: T) => void;
}) {
    const [value, setValue] = useState(ref.get());
    const isNumber = typeof value == "number";

    return (
        <>
            <select
                value={value}
                disabled={disabled}
                onChange={({ target }) => {
                    const value = isNumber ? Number(target.value) : target.value;

                    setValue(target.value as any);
                    ref.set(value as T);

                    if (onChange) {
                        onChange(value as T);
                    }
                }}
            >
                {Object.entries(options).map(([k, v]) => (
                    <option key={k} value={k}>
                        {v}
                    </option>
                ))}
            </select>
        </>
    );
}

function System({ settings, disabled }: { settings: Ref<SettingsType>; disabled: boolean }) {
    const [locales] = useAtom(localesAtom);

    return (
        <>
            <div className='module'>
                <h1>{locales.System}</h1>
                <div className='item'>
                    <p>{locales.DeviceName}:</p>
                    <Input ref={settings.SystemDeviceName} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.Language}:</p>
                    <Select
                        ref={settings.SystemLanguage as any}
                        options={LanguageOptions}
                        disabled={disabled}
                        onChange={(lang) => {
                            setLanguage(lang as keyof typeof Languages);
                        }}
                    />
                </div>
            </div>
        </>
    );
}

function Network({ settings, disabled }: { settings: Ref<SettingsType>; disabled: boolean }) {
    const [locales] = useAtom(localesAtom);

    return (
        <>
            <div className='module'>
                <h1>{locales.Network}</h1>
                <div className='item'>
                    <p>{locales.NetworkInterface}:</p>
                    <sub>{locales.NetworkInterfaceHelp}</sub>
                    <Input ref={settings.NetworkInterface} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkMulticast}:</p>
                    <sub>{locales.NetworkMulticastHelp}</sub>
                    <Input ref={settings.NetworkMulticast} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkServer}:</p>
                    <sub>{locales.NetworkServerHelp}</sub>
                    <Input ref={settings.NetworkServer} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkMtu}:</p>
                    <sub>{locales.NetworkMtuHelp}</sub>
                    <Input ref={settings.NetworkMtu} disabled={disabled} />
                </div>
            </div>
        </>
    );
}

function Codec({ settings, disabled }: { settings: Ref<SettingsType>; disabled: boolean }) {
    const [locales] = useAtom(localesAtom);

    return (
        <>
            <div className='module'>
                <h1>{locales.Codec}</h1>
                <div className='item'>
                    <p>{locales.CodecDecoder}:</p>
                    <sub>{locales.CodecDecoderHelp}</sub>
                    <Select
                        ref={settings.CodecDecoder as any}
                        options={VideoDecoders}
                        disabled={disabled}
                    />
                </div>
                <div className='item'>
                    <p>{locales.CodecEncoder}:</p>
                    <sub>{locales.CodecEncoderHelp}</sub>
                    <Select
                        ref={settings.CodecEncoder as any}
                        options={VideoEncoders}
                        disabled={disabled}
                    />
                </div>
            </div>
        </>
    );
}

function Video({ settings, disabled }: { settings: Ref<SettingsType>; disabled: boolean }) {
    const [locales] = useAtom(localesAtom);

    return (
        <>
            <div className='module'>
                <h1>{locales.Video}</h1>
                <div className='item'>
                    <p>{locales.VideoSize}:</p>
                    <sub>{locales.VideoSizeHelp}</sub>
                    <div>
                        <Input ref={settings.VideoSizeWidth} disabled={disabled} />
                        -
                        <Input ref={settings.VideoSizeHeight} disabled={disabled} />
                    </div>
                </div>
                <div className='item'>
                    <p>{locales.VideoFrameRate}:</p>
                    <sub>{locales.VideoFrameRateHelp}</sub>
                    <Input ref={settings.VideoFrameRate} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.BitRate}:</p>
                    <sub>{locales.VideoBitRateHelp}</sub>
                    <Input ref={settings.VideoBitRate} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.VideoKeyFrameInterval}:</p>
                    <sub>{locales.VideoKeyFrameIntervalHelp}</sub>
                    <Input ref={settings.VideoKeyFrameInterval} disabled={disabled} />
                </div>
            </div>
        </>
    );
}

function Audio({ settings, disabled }: { settings: Ref<SettingsType>; disabled: boolean }) {
    const [locales] = useAtom(localesAtom);

    return (
        <>
            <div className='module'>
                <h1>{locales.Audio}</h1>
                <div className='item'>
                    <p>{locales.AudioSampleRate}:</p>
                    <sub>{locales.AudioSampleRateHelp}</sub>
                    <Input ref={settings.AudioSampleRate} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.BitRate}:</p>
                    <sub>{locales.AudioBitRateHelp}</sub>
                    <Input ref={settings.AudioBitRate} disabled={disabled} />
                </div>
            </div>
        </>
    );
}

type RefHandle<T> = {
    value: T;
    get: () => T;
    set: (value: T) => void;
};

type Ref<T> = {
    [P in keyof T]: RefHandle<T[P]>;
};

function createSettingsRef(): Ref<SettingsType> {
    const [settings] = useAtom(settingsAtom);

    let ref = {} as Ref<SettingsType>;
    for (const k in DefaultSettings) {
        const property = k as keyof SettingsType;

        Object.assign(ref, {
            [property]: {
                value: settings[property],
                get() {
                    return this.value;
                },
                set(value: SettingsType[typeof property]) {
                    this.value = value;
                },
            },
        });
    }

    return ref;
}

function freezeSettingsRef(ref: Ref<SettingsType>): SettingsType {
    let values = {} as any;

    for (const k in ref) {
        values[k] = ref[k as keyof SettingsType].value;
    }

    return values;
}

export default function () {
    const settings = createSettingsRef();
    const [locales] = useAtom(localesAtom);
    const setSettings = useSetAtom(settingsAtom);
    const [disabled, setDisabled] = useState(false);

    function submit() {
        MessageRouter.call(Methods.SetName, settings.SystemDeviceName.value).then(() => {
            setSettings(() => freezeSettingsRef(settings));
            setDisabled(true);
        });
    }

    return (
        <>
            <div id='settings'>
                <div id='content'>
                    <System settings={settings} disabled={disabled} />
                    <Network settings={settings} disabled={disabled} />
                    <Codec settings={settings} disabled={disabled} />
                    <Video settings={settings} disabled={disabled} />
                    <Audio settings={settings} disabled={disabled} />
                </div>
                {!disabled && (
                    <button id='apply' className='click' onClick={submit}>
                        {locales.Apply}
                    </button>
                )}
            </div>
        </>
    );
}
