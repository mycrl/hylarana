import "../styles/main.receiver.devices.css";
import { DeviceInfo } from "../types";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";
import createLocalesStore from "../locales";

export default function ({ devices }: { devices: DeviceInfo[] }) {
    const Locales = createLocalesStore();

    return (
        <>
            <div id='ReceiverDevices'>
                <div className='header'>
                    <p>{Locales.Devices}</p>
                    <span>All devices that are casting the screen.</span>
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
                                    <p>{Locales.Video} -</p>
                                    <span>
                                        {Locales.Codec}: H264 / {Locales.VideoSize}: 1280x720 /
                                        {Locales.VideoFrameRate}: 30 / {Locales.BitRate}: 10000000 /
                                        {Locales.VideoFormat}: NV12
                                    </span>
                                </div>
                                <div className='item'>
                                    <p>{Locales.Audio} -</p>
                                    <span>
                                        {Locales.Codec}: OPUS / {Locales.AudioChannel}:
                                        {Locales.AudioStereo} / {Locales.AudioSampleRate}: 48000 /
                                        {Locales.AudioSampleBit}: 16 / {Locales.BitRate}: 64000
                                    </span>
                                </div>
                            </div>
                            <div className='accept click'>
                                <span>Accapt</span>
                            </div>
                        </div>
                    ))}
                    <span id='tips'>{Locales.DevicesSearchHelp}</span>
                </div>
            </div>
        </>
    );
}
