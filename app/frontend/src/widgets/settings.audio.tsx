import { Language } from "../state";
import { SettingsRef } from "./settings";

import Input from "../components/input";

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
        </>
    );
}
