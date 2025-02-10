import "../styles/main.sender.devices.css";
import { DevicesAtom, Device } from "../hylarana";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";
import { localesAtom } from "../locales";
import { useAtomValue } from "jotai";
import { RefObject, useRef, useState } from "react";

function DeviceNode({
    device,
    index,
    indexs,
    onChange,
}: {
    device: Device;
    index: number;
    onChange: () => void;
    indexs: RefObject<Set<number>>;
}) {
    const [selected, setSelected] = useState(false);

    return (
        <>
            <div
                className='device click'
                id={selected ? "selected" : ""}
                onClick={() => {
                    if (selected) {
                        indexs.current.delete(index);
                    } else {
                        indexs.current.add(index);
                    }

                    setSelected(!selected);
                    onChange();
                }}
            >
                <div className='logo'>
                    {device.kind == "Windows" && (
                        <FontAwesomeIcon
                            className='icon'
                            icon={
                                device.kind == "Windows"
                                    ? faWindows
                                    : device.kind == "Android"
                                    ? faAndroid
                                    : faApple
                            }
                        />
                    )}
                </div>
                <div className='info'>
                    <p>{device.name}</p>
                    <span>{device.addrs[0]}</span>
                </div>
            </div>
        </>
    );
}

export default function ({ onChange }: { onChange?: (values: Device[]) => void }) {
    const locales = useAtomValue(localesAtom);
    const devices = useAtomValue(DevicesAtom);
    const indexs = useRef(new Set<number>([]));

    return (
        <>
            <div id='SenderDevices'>
                <div className='header'>
                    <p>{locales.Devices}</p>
                    <span>{locales.DevicesHelp}</span>
                </div>
                <div className='items'>
                    {devices
                        .filter((it) => !it.description)
                        .map((it, i) => (
                            <DeviceNode
                                device={it}
                                indexs={indexs}
                                index={i}
                                key={i}
                                onChange={() => {
                                    if (onChange) {
                                        onChange([...indexs.current].map((i) => devices[i]));
                                    }
                                }}
                            />
                        ))}
                    <span id='tips'>{locales.DevicesSearchHelp}</span>
                </div>
            </div>
        </>
    );
}
