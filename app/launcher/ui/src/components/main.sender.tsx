import "../styles/main.sender.css";
import Switch from "./switch";
import Devices from "./main.sender.devices";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faVolumeLow, faDisplay, faNetworkWired } from "@fortawesome/free-solid-svg-icons";
import { localesAtom } from "../locales";
import { settingsAtom, broadcastAtom } from "../settings";
import { DisplaysAtom, AudiosAtom, Device, createSender, TransportStrategy } from "../hylarana";
import { useAtom, useAtomValue } from "jotai";
import { useRef, useState } from "react";

function Displays() {
    const sources = useAtomValue(DisplaysAtom);

    return (
        <>
            <div className='item'>
                <div className='icon'>
                    <FontAwesomeIcon icon={faDisplay} />
                </div>
                <select className='click'>
                    {sources.map((it) => (
                        <option key={it.id} value={it.index}>
                            {it.name}
                        </option>
                    ))}
                </select>
            </div>
        </>
    );
}

function Audios() {
    const sources = useAtomValue(AudiosAtom);

    return (
        <>
            <div className='item'>
                <div className='icon'>
                    <FontAwesomeIcon icon={faVolumeLow} />
                </div>
                <select className='click'>
                    {sources.map((it) => (
                        <option key={it.id} value={it.index}>
                            {it.name}
                        </option>
                    ))}
                </select>
            </div>
        </>
    );
}

function Transport({
    value,
    onChange,
}: {
    value: TransportStrategy;
    onChange: (value: TransportStrategy) => void;
}) {
    const locales = useAtomValue(localesAtom);
    const settings = useAtomValue(settingsAtom);

    return (
        <>
            <div className='item'>
                <div className='icon'>
                    <FontAwesomeIcon icon={faNetworkWired} />
                </div>
                <select
                    className='click'
                    value={value}
                    onChange={({ target }) => onChange(target.value as TransportStrategy)}
                >
                    <option value='Direct'>{locales.Direct}</option>
                    <option value='Relay'>{locales.Relay}</option>
                    {settings.NetworkServer && (
                        <option value='Multicast'>{locales.Multicast}</option>
                    )}
                </select>
            </div>
        </>
    );
}

export default function () {
    const locales = useAtomValue(localesAtom);
    const [broadcast, setBroadcast] = useAtom(broadcastAtom);
    const [transport, setTransport] = useState(TransportStrategy.Direct);

    const devices = useRef<Device[]>([]);
    const settings = useAtomValue(settingsAtom);
    function start() {
        createSender(
            devices.current.map((it) => it.addrs[0]),
            {
                transport: {
                    mtu: settings.NetworkMtu,
                    strategy: {
                        strategy: transport,
                        address: {
                            [TransportStrategy.Relay]: settings.NetworkServer,
                            [TransportStrategy.Direct]: settings.NetworkInterface,
                            [TransportStrategy.Multicast]: settings.NetworkMulticast,
                        }[transport] as string,
                    },
                },
                video: {
                    size: {
                        width: settings.VideoSizeWidth,
                        height: settings.VideoSizeHeight,
                    },
                    fps: settings.VideoFrameRate,
                    bitRate: settings.VideoBitRate,
                },
                audio: {
                    sampleRate: settings.AudioSampleRate,
                    bitRate: settings.AudioBitRate,
                },
            }
        );
    }

    return (
        <>
            <div id='Sender'>
                <div id='switch'>
                    <div className='body'>
                        <span>{locales.Broadcast}</span>
                        <Switch defaultValue={broadcast} onChange={setBroadcast} />
                    </div>
                    <p>{locales.BroadcastHelp}</p>
                </div>
                <div id='content'>
                    {!broadcast ? (
                        <Devices
                            onChange={(it) => {
                                devices.current = it;
                            }}
                        />
                    ) : (
                        <div className='padding'></div>
                    )}

                    <div id='control'>
                        <div className='box'>
                            <div className='items'>
                                <Audios />
                                <Displays />
                                <Transport value={transport} onChange={setTransport} />
                            </div>
                            <button className='click' onClick={start}>
                                {locales.SenderStart}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
}
