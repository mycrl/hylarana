import "../styles/main.receiver.devices.css";
import { Device } from "../hylarana";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";
import { useAtomValue } from "jotai";
import { localesAtom } from "../state";

export default function ({ devices }: { devices: Device[] }) {
    const locales = useAtomValue(localesAtom);

    return (
        <>
            <div id='ReceiverDevices'>
                <div className='header'>
                    <p>{locales.Devices}</p>
                    <span>{locales.DevicesReceiverHelp}</span>
                </div>
                <div className='items'>
                    {devices.map((it) => (
                        <div className='device'>
                            <div className='logo'>
                                {it.kind == "Windows" && (
                                    <FontAwesomeIcon className='icon' icon={faWindows} />
                                )}
                                {it.kind == "Android" && (
                                    <FontAwesomeIcon className='icon' icon={faAndroid} />
                                )}
                                {it.kind == "Apple" && (
                                    <FontAwesomeIcon className='icon' icon={faApple} />
                                )}
                            </div>
                            <div className='info'>
                                <p>{it.name}</p>
                                <span>{it.addrs[0]}</span>
                            </div>
                            <div className='description'>
                                <div className='item'>
                                    <p>{locales.Video} -</p>
                                    <span>
                                        {locales.Codec}: H264 / {locales.VideoSize}: 1280x720 /
                                        {locales.VideoFrameRate}: 30 / {locales.BitRate}: 10000000 /
                                        {locales.VideoFormat}: NV12
                                    </span>
                                </div>
                                <div className='item'>
                                    <p>{locales.Audio} -</p>
                                    <span>
                                        {locales.Codec}: OPUS / {locales.AudioChannel}:
                                        {locales.AudioStereo} / {locales.AudioSampleRate}: 48000 /
                                        {locales.AudioSampleBit}: 16 / {locales.BitRate}: 64000
                                    </span>
                                </div>
                            </div>
                            <div className='accept click'>
                                <span>Accapt</span>
                            </div>
                        </div>
                    ))}
                    <span id='tips'>{locales.DevicesSearchHelp}</span>
                </div>
            </div>
        </>
    );
}
