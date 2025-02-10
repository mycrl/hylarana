import "../styles/main.receiver.css";
import Devices from "./main.receiver.devices";
import Switch from "./switch";
import { localesAtom } from "../locales";
import { useState } from "react";
import { useAtomValue } from "jotai";

export default function () {
    const locales = useAtomValue(localesAtom);
    const [devices, _] = useState([]);

    return (
        <>
            <div id='Receiver'>
                <div id='switch'>
                    <div className='body'>
                        <span>{locales.AutoAllow}</span>
                        <Switch defaultValue={false} />
                    </div>
                    <p>{locales.AutoAllowHelp}</p>
                </div>
                <div className='devices'>
                    <Devices devices={devices} />
                </div>
            </div>
        </>
    );
}
