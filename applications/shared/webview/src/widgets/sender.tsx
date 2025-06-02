import "../styles/sender.css";

import { useAtomValue } from "jotai";
import { useRef, useState } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faVolumeHigh, faDisplay } from "@fortawesome/free-solid-svg-icons";

import Devices from "./sender.devices";
import SenderImage from "../assets/sender.svg";
import ReceiverImage from "../assets/receiver.svg";
import Button from "./sender.button";

import { closeSender, createSender } from "../core";
import { audiosAtom, displaysAtom, localesAtom, settingsAtom, statusAtom } from "../state";
import { Source, Status } from "../bridge";

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
                    <FontAwesomeIcon icon={faVolumeHigh} />
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

export default function ({
    offscreen,
    style,
}: {
    offscreen: boolean;
    style?: React.CSSProperties;
}) {
    const locales = useAtomValue(localesAtom);
    const status = useAtomValue(statusAtom);

    const settings = useAtomValue(settingsAtom);
    const displays = useAtomValue(displaysAtom);
    const audios = useAtomValue(audiosAtom);

    const address = useRef<string[]>([]);
    const display = useRef<number>(0);
    const audio = useRef<number>(0);

    return (
        <>
            <div className='Sender' style={style}>
                <div className='Content'>
                    {status == Status.Idle && (
                        <Devices
                            offscreen={offscreen}
                            onChange={(it) => {
                                address.current = it;
                            }}
                        />
                    )}
                    {status != Status.Idle && (
                        <div
                            className='Working'
                            style={{
                                marginTop: status == Status.Receiving ? "100px" : undefined,
                            }}
                        >
                            <img src={status == Status.Receiving ? ReceiverImage : SenderImage} />
                            <p>
                                {status == Status.Receiving
                                    ? locales.Receivering
                                    : locales.Sendering}
                            </p>
                        </div>
                    )}
                    {status != Status.Receiving && (
                        <div className='Control'>
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
                                </div>
                                <Button
                                    status={status}
                                    locales={locales}
                                    onStart={async () => {
                                        if (status == Status.Idle) {
                                            await createSender(
                                                address.current,
                                                displays[display.current],
                                                audios[audio.current],
                                                settings
                                            );
                                        }
                                    }}
                                    onStop={async () => {
                                        if (status == Status.Sending) {
                                            await closeSender();
                                        }
                                    }}
                                />
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </>
    );
}
