import "../styles/info.css";

import { useAtomValue } from "jotai";

import { localesAtom, settingsAtom, statusAtom, timerAtom } from "../state";
import { Status } from "../bridge";

export default function () {
    const settings = useAtomValue(settingsAtom);
    const locales = useAtomValue(localesAtom);
    const status = useAtomValue(statusAtom);
    const timer = useAtomValue(timerAtom);

    return (
        <>
            <div className='Info'>
                <span>{locales.Network}:</span>
                <p>{settings.network.bind}</p>
                <sub>-</sub>
                <span>{locales.Video}:</span>
                <p>
                    {settings.codec.encoder} · {settings.video.width}x{settings.video.height} ·{" "}
                    {settings.video.frame_rate} fps · {settings.video.bit_rate / 1000} kbit/s
                </p>
                <sub>-</sub>
                <span>{locales.Audio}:</span>
                <p>
                    {locales.AudioStereo} · {settings.audio.sample_rate / 1000} khz ·{" "}
                    {settings.audio.bit_rate / 1000}
                    kbit/s
                </p>

                <div
                    className='timer'
                    style={{
                        color: status != Status.Idle ? "var(--accept-color)" : undefined,
                    }}
                >
                    {timer}
                </div>
            </div>
        </>
    );
}
