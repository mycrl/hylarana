import { Language } from "../state";
import type { SettingsRef } from "./settings";
import { VideoDecoders, VideoEncoders } from "../core";

import Select from "../components/select";

export default function ({
    settings,
    locales,
    disabled,
}: {
    settings: SettingsRef;
    locales: Language;
    disabled: boolean;
}) {
    return (
        <>
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
        </>
    );
}
