import { Language } from "../state";
import type { SettingsRef } from "./settings";

import Input from "../components/input";

export default function ({
    settings,
    locales,
    disabled,
}: {
    settings: SettingsRef;
    locales: Language;
    disabled: boolean;
}) {
    return (
        <>
            <div className='module'>
                <h1>{locales.Network}</h1>
                <div className='item'>
                    <p>{locales.NetworkInterface}:</p>
                    <sub>{locales.NetworkInterfaceHelp}</sub>
                    <Input ref={settings.network.interface} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkMulticast}:</p>
                    <sub>{locales.NetworkMulticastHelp}</sub>
                    <Input ref={settings.network.multicast} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkServer}:</p>
                    <sub>{locales.NetworkServerHelp}</sub>
                    <Input ref={settings.network.server} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkPort}:</p>
                    <Input ref={settings.network.port} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkMtu}:</p>
                    <sub>{locales.NetworkMtuHelp}</sub>
                    <Input ref={settings.network.mtu} disabled={disabled} />
                </div>
            </div>
        </>
    );
}
