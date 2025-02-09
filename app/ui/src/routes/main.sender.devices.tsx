import "../styles/main.sender.devices.css";
import { Device } from "../devices";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faWindows, faApple, faAndroid } from "@fortawesome/free-brands-svg-icons";
import { localesAtom } from "../locales";
import { useAtom } from "jotai";

export default function ({ devices }: { devices: Device[] }) {
    const [locales] = useAtom(localesAtom);

    return (
        <>
            <div id='SenderDevices'>
                <div className='header'>
                    <p>{locales.Devices}</p>
                    <span>{locales.DevicesHelp}</span>
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
                    <span id='tips'>{locales.DevicesSearchHelp}</span>
                </div>
            </div>
        </>
    );
}
