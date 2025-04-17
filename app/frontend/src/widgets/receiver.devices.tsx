import "../styles/receiver.devices.css";

import { useAtomValue } from "jotai";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faLink } from "@fortawesome/free-solid-svg-icons";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";

import { createReceiver } from "../core";
import { devicesAtom, localesAtom, settingsAtom, statusAtom } from "../state";
import { Device, Status } from "../bridge";

export default function ({ offscreen }: { offscreen: boolean }) {
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
            <div className='Devices'>
                <div className='header'>
                    <p>{locales.Devices}</p>
                    <span>{locales.DevicesReceiverHelp}</span>
                </div>
                {!offscreen && (
                    <div className='items'>
                        {devices
                            .filter((it) => it.description != null)
                            .map((it) => (
                                <div className='item' key={it.description?.id}>
                                    <div className='header'>
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
                                    </div>
                                    <div className='description'>
                                        {it.description?.video && (
                                            <div className='sub'>
                                                <p>{locales.Video} - </p>
                                                <span>
                                                    {locales.Codec}: H264 / {locales.VideoSize}:{" "}
                                                    {it.description?.video?.size.width}x
                                                    {it.description?.video?.size.height} /
                                                    {locales.VideoFrameRate}:{" "}
                                                    {it.description?.video?.fps} / {locales.BitRate}
                                                    :{it.description?.video?.bit_rate} /
                                                    {locales.VideoFormat}:{" "}
                                                    {it.description?.video?.format}
                                                </span>
                                            </div>
                                        )}
                                        {it.description?.audio && (
                                            <div className='sub'>
                                                <p>{locales.Audio} - </p>
                                                <span>
                                                    {locales.Codec}: OPUS / {locales.AudioChannel}:
                                                    {locales.AudioStereo} /{" "}
                                                    {locales.AudioSampleRate}:{" "}
                                                    {it.description?.audio?.sample_rate}/
                                                    {locales.AudioSampleBit}: 16 / {locales.BitRate}
                                                    : {it.description?.audio?.bit_rate}
                                                </span>
                                            </div>
                                        )}
                                    </div>
                                    <button className='accept click' onClick={() => accept(it)}>
                                        <FontAwesomeIcon className='icon' icon={faLink} />
                                        <span>{locales.Accapt}</span>
                                    </button>
                                </div>
                            ))}
                        <div className='loading searching'>{locales.Searching}</div>
                    </div>
                )}
            </div>
        </>
    );
}
