import "../styles/main.sender.devices.css";
import { DeviceInfo } from "../types";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";
import createLocalesStore from "../locales";

export default function ({ devices }: { devices: DeviceInfo[] }) {
    const Locales = createLocalesStore();

    return (
        <>
            <div id='SenderDevices'>
                <div className='header'>
                    <p>{Locales.Devices}</p>
                    <span>{Locales.DevicesHelp}</span>
                </div>
                <div className='items'>
                    {devices.map((it) => (
                        <div className='device click'>
                            <div className='logo'>
                                {it.kind == "Windows" && (
                                    <FontAwesomeIcon className='icon' icon={faWindows} />
                                )}
                                {it.kind == "Android" && (
                                    <FontAwesomeIcon className='icon' icon={faAndroid} />
                                )}
                                {it.kind == "Apple" && (
                                    <FontAwesomeIcon className='icon' icon={faApple} />
                                )}
                            </div>
                            <div className='info'>
                                <p>{it.name}</p>
                                <span>{it.addrs[0]}</span>
                            </div>
                        </div>
                    ))}
                    <span id='tips'>{Locales.DevicesSearchHelp}</span>
                </div>
            </div>
        </>
    );
}
