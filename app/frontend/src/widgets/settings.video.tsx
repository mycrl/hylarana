import { Language } from "../state";
import type { SettingsRef } from "./settings";

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
        </>
    );
}
