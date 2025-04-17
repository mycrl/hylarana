import "../styles/sender.devices.css";

import { useAtomValue } from "jotai";
import { useEffect, useState } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faCircleInfo } from "@fortawesome/free-solid-svg-icons";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";

import { devicesAtom, localesAtom } from "../state";
import { Device } from "../bridge";

export default function ({
    offscreen,
    onChange,
}: {
    offscreen: boolean;
    onChange?: (values: string[]) => void;
}) {
    const locales = useAtomValue(localesAtom);
    const devices = useAtomValue(devicesAtom);

    const [receivers, setReceivers] = useState<Device[]>([]);
    const [selecteds, setSelecteds] = useState<string[]>([]);

    useEffect(() => {
        setReceivers(devices.filter((it) => !it.description));
    }, [devices]);

    return (
        <>
            <div className='Devices'>
                <div className='header'>
                    <p>{locales.Devices}</p>
                    <span>{locales.DevicesHelp}</span>
                </div>
                <div className='count'>
                    <p>
                        {locales.Selected}
                        <span>{selecteds.length}</span>
                        {locales.DevicesCount} - {locales.SelectAll}
                        <input
                            className='click'
                            type='checkbox'
                            checked={receivers.length > 0 && selecteds.length == receivers.length}
                            onChange={({ target }) => {
                                let values: string[] = [];

                                if (target.checked) {
                                    values = receivers.map((v) => v.ip);
                                }

                                setSelecteds(values);
                                if (onChange) {
                                    onChange(values);
                                }
                            }}
                        />
                    </p>
                    <sub>
                        <FontAwesomeIcon icon={faCircleInfo} />
                        {" " + locales.BroadcastHelp}
                    </sub>
                </div>
                {!offscreen && (
                    <div className='items'>
                        {receivers.map((it) => (
                            <div
                                key={it.ip}
                                className='item click'
                                id={selecteds.includes(it.ip) ? "selected" : undefined}
                                onClick={() => {
                                    let values = [];

                                    if (selecteds.includes(it.ip)) {
                                        values = selecteds.filter((v) => v != it.ip);
                                    } else {
                                        values = [...selecteds, it.ip];
                                    }

                                    setSelecteds(values);
                                    if (onChange) {
                                        onChange(values);
                                    }
                                }}
                            >
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
                        ))}
                        <div className='loading searching'>{locales.Searching}</div>
                    </div>
                )}
            </div>
        </>
    );
}
