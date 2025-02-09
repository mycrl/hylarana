import "../styles/main.info.css";
import { createLocalesStore } from "../locales";
import { createSettingsStore, VideoEncoders } from "../settings";

export default function () {
    const Settings = createSettingsStore();
    const Locales = createLocalesStore();

    return (
        <>
            <div id='Info'>
                <span>{Locales.Network}:</span>
                <p>
                    {Locales.Direct}-{Settings.NetworkInterface}
                </p>
                <sub>/</sub>
                <span>{Locales.Video}:</span>
                <p>
                    {VideoEncoders[Settings.CodecEncoder]}/{Settings.VideoSizeWidth}x
                    {Settings.VideoSizeHeight}/{Settings.VideoFrameRate}/{Settings.VideoBitRate}
                </p>
                <sub>/</sub>
                <span>{Locales.Audio}:</span>
                <p>
                    {Locales.AudioStereo}/{Settings.AudioSampleRate}/{Settings.AudioBitRate}
                </p>

                <div className='timer'>00:00:00</div>
            </div>
        </>
    );
}
