import "../styles/main.receiver.css";
import { DeviceInfo } from "../types";
import Devices from "./main.receiver.devices";
import Switch from "../components/switch";
import createLocalesStore from "../locales";

export default function ({ devices }: { devices: DeviceInfo[] }) {
    const Locales = createLocalesStore();

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
