import "../styles/main.settings.css";
import { useState } from "react";
import { useAtom, useAtomValue } from "jotai";
import { VideoDecoders, VideoEncoders } from "../core";
import { localesAtom, settingsAtom, statusAtom } from "../state";
import { Backend, Settings, Status } from "@/hylarana";
import { LanguageOptions } from "../locales";

type RefHandle<T> = {
    value: T;
    get: () => T;
    set: (value: T) => void;
};

type Ref<T> = {
    [P in keyof T]: RefHandle<T[P]>;
};

type SettingsRef = {
    [P in keyof Settings]: Ref<Settings[P]>;
};

function createSettingsRef(settings: any): SettingsRef {
    let ref = {} as any;
    for (const key of Object.keys(settings)) {
        if (!ref[key]) {
            ref[key] = {};
        }

        for (const k of Object.keys(settings[key])) {
            Object.assign(ref[key], {
                [k]: {
                    value: settings[key][k],
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

function freezeSettingsRef(ref: any): Settings {
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

export default function () {
    const locales = useAtomValue(localesAtom);
    const [settings_, setSettings] = useAtom(settingsAtom);
    const [disabled, setDisabled] = useState(useAtomValue(statusAtom) != Status.Idle);

    let settings = createSettingsRef(settings_);

    return (
        <>
            <div id='settings'>
                <div id='content'>
                    <div className='module'>
                        <h1>{locales.System}</h1>
                        <div className='item'>
                            <p>{locales.DeviceName}:</p>
                            <Input ref={settings.system.name} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.Language}:</p>
                            <Select
                                ref={settings.system.language as any}
                                options={LanguageOptions}
                                disabled={disabled}
                            />
                        </div>
                        <div className='item'>
                            <p>{locales.RendererBackend}:</p>
                            <sub>{locales.RendererBackendHelp}</sub>
                            <Select
                                ref={settings.system.backend as any}
                                options={Backend}
                                disabled={disabled}
                            />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{locales.Network}</h1>
                        <div className='item'>
                            <p>{locales.NetworkInterface}:</p>
                            <sub>{locales.NetworkInterfaceHelp}</sub>
                            <Input ref={settings.network.interface} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.NetworkMulticast}:</p>
                            <sub>{locales.NetworkMulticastHelp}</sub>
                            <Input ref={settings.network.multicast} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.NetworkServer}:</p>
                            <sub>{locales.NetworkServerHelp}</sub>
                            <Input ref={settings.network.server} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.NetworkPort}:</p>
                            <Input ref={settings.network.port} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.NetworkMtu}:</p>
                            <sub>{locales.NetworkMtuHelp}</sub>
                            <Input ref={settings.network.mtu} disabled={disabled} />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{locales.Codec}</h1>
                        <div className='item'>
                            <p>{locales.CodecDecoder}:</p>
                            <sub>{locales.CodecDecoderHelp}</sub>
                            <Select
                                ref={settings.codec.decoder as any}
                                options={VideoDecoders}
                                disabled={disabled}
                            />
                        </div>
                        <div className='item'>
                            <p>{locales.CodecEncoder}:</p>
                            <sub>{locales.CodecEncoderHelp}</sub>
                            <Select
                                ref={settings.codec.encoder as any}
                                options={VideoEncoders}
                                disabled={disabled}
                            />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{locales.Video}</h1>
                        <div className='item'>
                            <p>{locales.VideoSize}:</p>
                            <sub>{locales.VideoSizeHelp}</sub>
                            <div>
                                <Input ref={settings.video.width} disabled={disabled} />
                                -
                                <Input ref={settings.video.height} disabled={disabled} />
                            </div>
                        </div>
                        <div className='item'>
                            <p>{locales.VideoFrameRate}:</p>
                            <sub>{locales.VideoFrameRateHelp}</sub>
                            <Input ref={settings.video.frame_rate} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.BitRate}:</p>
                            <sub>{locales.VideoBitRateHelp}</sub>
                            <Input ref={settings.video.bit_rate} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.VideoKeyFrameInterval}:</p>
                            <sub>{locales.VideoKeyFrameIntervalHelp}</sub>
                            <Input ref={settings.video.key_frame_interval} disabled={disabled} />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{locales.Audio}</h1>
                        <div className='item'>
                            <p>{locales.AudioSampleRate}:</p>
                            <sub>{locales.AudioSampleRateHelp}</sub>
                            <Input ref={settings.audio.sample_rate} disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{locales.BitRate}:</p>
                            <sub>{locales.AudioBitRateHelp}</sub>
                            <Input ref={settings.audio.bit_rate} disabled={disabled} />
                        </div>
                    </div>
                </div>
                {!disabled && (
                    <button
                        id='apply'
                        className='click'
                        onClick={() => {
                            setSettings(freezeSettingsRef(settings));
                            setDisabled(true);
                        }}
                    >
                        {locales.Apply}
                    </button>
                )}
            </div>
        </>
    );
}
