import "../styles/main.receiver.css";
import Devices from "./main.receiver.devices";
import Switch from "../components/switch";
import { localesAtom } from "../locales";
import { useState } from "react";
import { useAtom } from "jotai";

export default function () {
    const [locales] = useAtom(localesAtom);
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
