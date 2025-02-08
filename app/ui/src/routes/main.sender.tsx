import "../styles/main.sender.css";
import { DeviceInfo } from "../types";
import Switch from "../components/switch";
import Devices from "./main.sender.devices";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faVolumeLow, faDisplay, faNetworkWired } from "@fortawesome/free-solid-svg-icons";
import createLocalesStore from "../locales";
import createSettingsStore from "../settings";

export default function ({ devices }: { devices: DeviceInfo[] }) {
    const Settings = createSettingsStore();
    const Locales = createLocalesStore();

    return (
        <>
            <div id='Sender'>
                <div id='switch'>
                    <div className='body'>
                        <span>{Locales.Broadcast}</span>
                        <Switch defaultValue={Settings.SystemSenderBroadcast} />
                    </div>
                    <p>{Locales.BroadcastHelp}</p>
                </div>
                <div id='content'>
                    {!Settings.SystemSenderBroadcast ? (
                        <Devices devices={devices} />
                    ) : (
                        <div className='padding'></div>
                    )}

                    <div id='control'>
                        <div className='box'>
                            <div className='items'>
                                <div className='item'>
                                    <div className='icon'>
                                        <FontAwesomeIcon icon={faVolumeLow} />
                                    </div>
                                    <select className='click'>
                                        <option>Redmi电脑音响</option>
                                    </select>
                                </div>
                                <div className='item'>
                                    <div className='icon'>
                                        <FontAwesomeIcon icon={faDisplay} />
                                    </div>
                                    <select className='click'>
                                        <option>P27QBB-RA</option>
                                    </select>
                                </div>
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
                            </div>
                            <button className='click'>Start</button>
                        </div>
                    </div>
                </div>
            </div>
        </>
    );
}
