import "../styles/main.receiver.css";
import Devices from "./main.receiver.devices";
import Switch from "../components/switch";
import { createLocalesStore } from "../locales";
import { useState } from "react";

export default function () {
    const Locales = createLocalesStore();
    const [devices, _] = useState([]);

    return (
        <>
            <div id='Receiver'>
                <div id='switch'>
                    <div className='body'>
                        <span>{Locales.AutoAllow}</span>
                        <Switch defaultValue={false} />
                    </div>
                    <p>{Locales.AutoAllowHelp}</p>
                </div>
                <div className='devices'>
                    <Devices devices={devices} />
                </div>
            </div>
        </>
    );
}
