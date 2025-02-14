import "../styles/main.sender.css";
import Devices from "./main.sender.devices";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faVolumeLow, faDisplay, faNetworkWired } from "@fortawesome/free-solid-svg-icons";
import { useAtomValue } from "jotai";
import { useRef, useState } from "react";
import { closeSender, createSender, Source, Status, TransportStrategy } from "../core";
import { audiosAtom, displaysAtom, localesAtom, settingsAtom, statusAtom } from "../state";
import SenderImage from "../assets/sender.svg";
import ReceiverImage from "../assets/receiver.svg";

function Displays({
    displays,
    disabled,
    onChange,
}: {
    disabled: boolean;
    displays: Source[];
    onChange: (index: number) => void;
}) {
    const [index, setIndex] = useState(0);

    return (
        <>
            <div className='item'>
                <div className='icon'>
                    <FontAwesomeIcon icon={faDisplay} />
                </div>
                <select
                    className='click'
                    value={index}
                    disabled={disabled}
                    onChange={({ target }) => {
                        const value = Number(target.value);
                        setIndex(value);
                        onChange(value);
                    }}
                >
                    {displays.map((it, index) => (
                        <option key={it.id} value={index}>
                            {it.name}
                        </option>
                    ))}
                </select>
            </div>
        </>
    );
}

function Audios({
    audios,
    disabled,
    onChange,
}: {
    audios: Source[];
    disabled: boolean;
    onChange: (index: number) => void;
}) {
    const [index, setIndex] = useState(0);

    return (
        <>
            <div className='item'>
                <div className='icon'>
                    <FontAwesomeIcon icon={faVolumeLow} />
                </div>
                <select
                    className='click'
                    value={index}
                    disabled={disabled}
                    onChange={({ target }) => {
                        const value = Number(target.value);
                        setIndex(value);
                        onChange(value);
                    }}
                >
                    {audios.map((it, index) => (
                        <option key={it.id} value={index}>
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
    disabled,
    onChange,
}: {
    disabled: boolean;
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
                    disabled={disabled}
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
    const status = useAtomValue(statusAtom);
    const [transport, setTransport] = useState(TransportStrategy.Direct);

    const settings = useAtomValue(settingsAtom);
    const displays = useAtomValue(displaysAtom);
    const audios = useAtomValue(audiosAtom);

    const names = useRef<string[]>([]);
    const display = useRef<number>(0);
    const audio = useRef<number>(0);

    async function start() {
        if (status == Status.Idle) {
            await createSender(names.current, {
                transport: {
                    mtu: settings.NetworkMtu,
                    strategy: {
                        [transport]:
                            ({
                                [TransportStrategy.Relay]: settings.NetworkServer,
                                [TransportStrategy.Direct]: settings.NetworkInterface,
                                [TransportStrategy.Multicast]: settings.NetworkMulticast,
                            }[transport] as string) +
                            ":" +
                            settings.NetworkPort,
                    } as any,
                },
                media: {
                    video: {
                        source: displays[display.current],
                        options: {
                            codec: settings.CodecEncoder,
                            frame_rate: settings.VideoFrameRate,
                            width: settings.VideoSizeWidth,
                            height: settings.VideoSizeHeight,
                            bit_rate: settings.VideoBitRate,
                            key_frame_interval: settings.VideoKeyFrameInterval,
                        },
                    },
                    audio: {
                        source: audios[audio.current],
                        options: {
                            sample_rate: settings.AudioSampleRate,
                            bit_rate: settings.AudioBitRate,
                        },
                    },
                },
            });
        }
    }

    async function stop() {
        if (status == Status.Sending) {
            await closeSender();
        }
    }

    return (
        <>
            <div id='Sender'>
                <div id='content'>
                    {status == Status.Idle && (
                        <Devices
                            onChange={(it) => {
                                names.current = it;
                            }}
                        />
                    )}
                    {status != Status.Idle && (
                        <div className='working'>
                            <img
                                src={status == Status.Receiving ? ReceiverImage : SenderImage}
                                style={{
                                    marginTop: status == Status.Receiving ? "50px" : "100px",
                                }}
                            />
                            <p>
                                {status == Status.Receiving
                                    ? locales.Receivering
                                    : locales.Sendering}
                            </p>
                        </div>
                    )}
                    <div id='control'>
                        <div className='box'>
                            <div className='items'>
                                <Audios
                                    disabled={status != Status.Idle}
                                    audios={audios}
                                    onChange={(it) => {
                                        audio.current = it;
                                    }}
                                />
                                <Displays
                                    disabled={status != Status.Idle}
                                    displays={displays}
                                    onChange={(it) => {
                                        display.current = it;
                                    }}
                                />
                                <Transport
                                    disabled={status != Status.Idle}
                                    value={transport}
                                    onChange={setTransport}
                                />
                            </div>
                            <button
                                className='click'
                                onClick={() => {
                                    status == Status.Idle ? start() : stop();
                                }}
                                disabled={status == Status.Receiving}
                                style={{
                                    backgroundColor:
                                        status == Status.Sending
                                            ? "#f00222"
                                            : status == Status.Receiving
                                            ? "#ddd"
                                            : undefined,
                                }}
                            >
                                {status == Status.Sending
                                    ? locales.SenderStop
                                    : locales.SenderStart}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
}
