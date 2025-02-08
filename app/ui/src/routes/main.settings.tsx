import "../styles/main.settings.css";
import { Settings, VideoDecoders, VideoEncoders, setSettings } from "../settings";
import { useState } from "react";
import { MessageRouter } from "../message";
import createLocalesStore, { LanguageOptions, languageChange } from "../locales";

function Input<T extends string | number | null>({
    ref,
    property,
    disabled,
}: {
    property: string;
    disabled: boolean;
    ref: { [k: string]: T };
}) {
    const [value, setValue] = useState(ref[property]);
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
                    ref[property] = value as T;
                }}
            />
        </>
    );
}

function Select<T extends string | number>({
    ref,
    property,
    options,
    disabled,
}: {
    property: string;
    disabled: boolean;
    ref: { [k: string]: T };
    options: { [k: string]: T };
}) {
    const [value, setValue] = useState(ref[property]);
    const isNumber = typeof value == "number";

    return (
        <>
            <select
                value={value}
                disabled={disabled}
                onChange={({ target }) => {
                    const value = isNumber ? Number(target.value) : target.value;

                    setValue(target.value as any);
                    ref[property] = value as T;
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
    const [disabled, setDisabled] = useState(false);
    const Locales = createLocalesStore();

    let settings = Object.assign({}, Settings) as any;

    function submit() {
        setSettings(settings);
        setDisabled(true);
        languageChange();

        // MessageRouter.call("SetName", settings.systemDeviceName).then(() => {
        //     setDisabled(true);
        // });
    }

    return (
        <>
            <div id='Settings'>
                <div id='content'>
                    <div className='module'>
                        <h1>{Locales.System}</h1>
                        <div className='item'>
                            <p>{Locales.DeviceName}:</p>
                            <Input ref={settings} property='SystemDeviceName' disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{Locales.Language}:</p>
                            <Select
                                ref={settings}
                                property='SystemLanguage'
                                options={LanguageOptions}
                                disabled={disabled}
                            />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{Locales.Network}</h1>
                        <div className='item'>
                            <p>{Locales.NetworkInterface}:</p>
                            <sub>{Locales.NetworkInterfaceHelp}</sub>
                            <Input ref={settings} property='NetworkInterface' disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{Locales.NetworkMulticast}:</p>
                            <sub>{Locales.NetworkMulticastHelp}</sub>
                            <Input ref={settings} property='NetworkMulticast' disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{Locales.NetworkServer}:</p>
                            <sub>{Locales.NetworkServerHelp}</sub>
                            <Input ref={settings} property='NetworkServer' disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{Locales.NetworkMtu}:</p>
                            <sub>{Locales.NetworkMtuHelp}</sub>
                            <Input ref={settings} property='NetworkMtu' disabled={disabled} />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{Locales.Codec}</h1>
                        <div className='item'>
                            <p>{Locales.CodecDecoder}:</p>
                            <sub>{Locales.CodecDecoderHelp}</sub>
                            <Select
                                ref={settings}
                                property='CodecDecoder'
                                options={VideoDecoders}
                                disabled={disabled}
                            />
                        </div>
                        <div className='item'>
                            <p>{Locales.CodecEncoder}:</p>
                            <sub>{Locales.CodecEncoderHelp}</sub>
                            <Select
                                ref={settings}
                                property='CodecEncoder'
                                options={VideoEncoders}
                                disabled={disabled}
                            />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{Locales.Video}</h1>
                        <div className='item'>
                            <p>{Locales.VideoSize}:</p>
                            <sub>{Locales.VideoSizeHelp}</sub>
                            <div>
                                <Input
                                    ref={settings}
                                    property='VideoSizeWidth'
                                    disabled={disabled}
                                />
                                -
                                <Input
                                    ref={settings}
                                    property='VideoSizeHeight'
                                    disabled={disabled}
                                />
                            </div>
                        </div>
                        <div className='item'>
                            <p>{Locales.VideoFrameRate}:</p>
                            <sub>{Locales.VideoFrameRateHelp}</sub>
                            <Input ref={settings} property='VideoFrameRate' disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{Locales.BitRate}:</p>
                            <sub>{Locales.VideoBitRateHelp}</sub>
                            <Input ref={settings} property='VideoBitRate' disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{Locales.VideoKeyFrameInterval}:</p>
                            <sub>{Locales.VideoKeyFrameIntervalHelp}</sub>
                            <Input
                                ref={settings}
                                property='VideoKeyFrameInterval'
                                disabled={disabled}
                            />
                        </div>
                    </div>
                    <div className='module'>
                        <h1>{Locales.Audio}</h1>
                        <div className='item'>
                            <p>{Locales.AudioSampleRate}:</p>
                            <sub>{Locales.AudioSampleRateHelp}</sub>
                            <Input ref={settings} property='AudioSampleRate' disabled={disabled} />
                        </div>
                        <div className='item'>
                            <p>{Locales.BitRate}:</p>
                            <sub>{Locales.AudioBitRateHelp}</sub>
                            <Input ref={settings} property='AudioBitRate' disabled={disabled} />
                        </div>
                    </div>
                </div>
                {!disabled && (
                    <button id='apply' className='click' onClick={submit}>
                        {Locales.Apply}
                    </button>
                )}
            </div>
        </>
    );
}
