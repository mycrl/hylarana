import "../styles/main.sender.css";
import { useState } from "react";
import Switch from "../components/switch";
import Devices from "./main.sender.devices";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faVolumeLow, faDisplay, faNetworkWired } from "@fortawesome/free-solid-svg-icons";
import { localesAtom, Language } from "../locales";
import { settingsAtom } from "../settings";
import { DisplaysAtom, AudiosAtom, DevicesAtom } from "../devices";
import { useAtom } from "jotai";

function Displays() {
    const [sources] = useAtom(DisplaysAtom);

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
    const [sources] = useAtom(AudiosAtom);

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

function Transport({ locales }: { locales: Language }) {
    const [settings] = useAtom(settingsAtom);

    return (
        <>
            <div className='item'>
                <div className='icon'>
                    <FontAwesomeIcon icon={faNetworkWired} />
                </div>
                <select className='click'>
                    <option>{locales.Direct}</option>
                    <option>{locales.Relay}</option>
                    {settings.NetworkServer && <option>{locales.Multicast}</option>}
                </select>
            </div>
        </>
    );
}

export default function () {
    const [locales] = useAtom(localesAtom);
    const [settings] = useAtom(settingsAtom);
    const [devices] = useAtom(DevicesAtom);

    const [broadcast, setBroadcast] = useState(settings.SystemSenderBroadcast);

    return (
        <>
            <div id='Sender'>
                <div id='switch'>
                    <div className='body'>
                        <span>{locales.Broadcast}</span>
                        <Switch
                            defaultValue={settings.SystemSenderBroadcast}
                            onChange={(it) => {
                                settings.SystemSenderBroadcast = it;
                                setBroadcast(it);
                            }}
                        />
                    </div>
                    <p>{locales.BroadcastHelp}</p>
                </div>
                <div id='content'>
                    {!broadcast ? <Devices devices={devices} /> : <div className='padding'></div>}

                    <div id='control'>
                        <div className='box'>
                            <div className='items'>
                                <Audios />
                                <Displays />
                                <Transport locales={locales} />
                            </div>
                            <button className='click'>{locales.SenderStart}</button>
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
}
