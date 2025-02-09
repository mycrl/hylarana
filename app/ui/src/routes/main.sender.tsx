import "../styles/main.sender.css";
import { useState } from "react";
import Switch from "../components/switch";
import Devices from "./main.sender.devices";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faVolumeLow, faDisplay, faNetworkWired } from "@fortawesome/free-solid-svg-icons";
import { createLocalesStore, Language } from "../locales";
import { Settings, setSettings } from "../settings";
import { createDevicesStore, createDisplaysStore, createAudiosStore } from "../devices";

function Displays() {
    const sources = createDisplaysStore();

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
    const sources = createAudiosStore();

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

function Transport({ Locales }: { Locales: Language }) {
    return (
        <>
            <div className='item'>
                <div className='icon'>
                    <FontAwesomeIcon icon={faNetworkWired} />
                </div>
                <select className='click'>
                    <option>{Locales.Direct}</option>
                    <option>{Locales.Relay}</option>
                    <option>{Locales.Multicast}</option>
                </select>
            </div>
        </>
    );
}

export default function () {
    const Locales = createLocalesStore();
    const devices = createDevicesStore();

    const [broadcast, setBroadcast] = useState(Settings.SystemSenderBroadcast);

    return (
        <>
            <div id='Sender'>
                <div id='switch'>
                    <div className='body'>
                        <span>{Locales.Broadcast}</span>
                        <Switch
                            defaultValue={Settings.SystemSenderBroadcast}
                            onChange={(it) => {
                                Settings.SystemSenderBroadcast = it;
                                setSettings(Settings);
                                setBroadcast(it);
                            }}
                        />
                    </div>
                    <p>{Locales.BroadcastHelp}</p>
                </div>
                <div id='content'>
                    {!broadcast ? <Devices devices={devices} /> : <div className='padding'></div>}

                    <div id='control'>
                        <div className='box'>
                            <div className='items'>
                                <Audios />
                                <Displays />
                                <Transport Locales={Locales} />
                            </div>
                            <button className='click'>{Locales.SenderStart}</button>
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
}
