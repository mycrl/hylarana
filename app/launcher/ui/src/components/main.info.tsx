import "../styles/main.info.css";
import { localesAtom } from "../locales";
import { settingsAtom } from "../settings";
import { useAtomValue } from "jotai";
import { VideoEncoders } from "../hylarana";

export default function () {
    const settings = useAtomValue(settingsAtom);
    const locales = useAtomValue(localesAtom);

    return (
        <>
            <div id='Info'>
                <span>{locales.Network}:</span>
                <p>
                    {locales.Direct}-{settings.NetworkInterface}
                </p>
                <sub>/</sub>
                <span>{locales.Video}:</span>
                <p>
                    {VideoEncoders[settings.CodecEncoder]}/{settings.VideoSizeWidth}x
                    {settings.VideoSizeHeight}/{settings.VideoFrameRate}/{settings.VideoBitRate}
                </p>
                <sub>/</sub>
                <span>{locales.Audio}:</span>
                <p>
                    {locales.AudioStereo}/{settings.AudioSampleRate}/{settings.AudioBitRate}
                </p>

                <div className='timer'>00:00:00</div>
            </div>
        </>
    );
}
