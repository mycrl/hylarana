import "../styles/main.receiver.devices.css";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";
import { devicesAtom, localesAtom, settingsAtom, statusAtom } from "../state";
import { Device, Status } from "@/hylarana";
import { createReceiver } from "../core";
import { useAtomValue } from "jotai";

export default function () {
    const locales = useAtomValue(localesAtom);
    const devices = useAtomValue(devicesAtom);
    const settings = useAtomValue(settingsAtom);
    const status = useAtomValue(statusAtom);

    async function accept(device: Device) {
        if (status == Status.Idle) {
            await createReceiver(device, settings);
        }
    }

    return (
        <>
            <div id='ReceiverDevices'>
                <div className='header'>
                    <p>{locales.Devices}</p>
                    <span>{locales.DevicesReceiverHelp}</span>
                </div>
                <div className='items'>
                    {devices
                        .filter((it) => it.description != null)
                        .map((it) => (
                            <div className='device' key={it.name}>
                                <div className='logo'>
                                    <FontAwesomeIcon
                                        className='icon'
                                        icon={
                                            it.kind == "Windows"
                                                ? faWindows
                                                : it.kind == "Android"
                                                ? faAndroid
                                                : faApple
                                        }
                                    />
                                </div>
                                <div className='info'>
                                    <p>{it.name}</p>
                                    <span>{it.ip}</span>
                                </div>
                                <div className='description'>
                                    <div className='item'>
                                        <p>{locales.Video} - </p>
                                        <span>
                                            {locales.Codec}: H264 / {locales.VideoSize}: 1280x720 /
                                            {locales.VideoFrameRate}: 30 / {locales.BitRate}:
                                            10000000 /{locales.VideoFormat}: NV12
                                        </span>
                                    </div>
                                    <div className='item'>
                                        <p>{locales.Audio} - </p>
                                        <span>
                                            {locales.Codec}: OPUS / {locales.AudioChannel}:
                                            {locales.AudioStereo} / {locales.AudioSampleRate}: 48000
                                            /{locales.AudioSampleBit}: 16 / {locales.BitRate}: 64000
                                        </span>
                                    </div>
                                </div>
                                <div className='accept click' onClick={() => accept(it)}>
                                    <span>{locales.Accapt}</span>
                                </div>
                            </div>
                        ))}
                    <span id='tips'>{locales.DevicesSearchHelp}</span>
                </div>
            </div>
        </>
    );
}
