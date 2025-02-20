import "../styles/main.info.css";
import { useAtomValue } from "jotai";
import { localesAtom, settingsAtom, timerAtom } from "../state";

export default function () {
    const settings = useAtomValue(settingsAtom);
    const locales = useAtomValue(localesAtom);
    const timer = useAtomValue(timerAtom);

    return (
        <>
            <div id='Info'>
                <span>{locales.Network}:</span>
                <p>
                    {locales.Direct}-{settings.network.interface}
                </p>
                <sub>/</sub>
                <span>{locales.Video}:</span>
                <p>
                    {settings.codec.encoder}/{settings.video.width}x{settings.video.height}/
                    {settings.video.frame_rate}/{settings.video.bit_rate}
                </p>
                <sub>/</sub>
                <span>{locales.Audio}:</span>
                <p>
                    {locales.AudioStereo}/{settings.audio.sample_rate}/{settings.audio.bit_rate}
                </p>

                <div className='timer'>
                    {timer}
                </div>
            </div>
        </>
    );
}
