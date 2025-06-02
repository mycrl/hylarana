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
                    <p>{locales.NetworkBind}:</p>
                    <span>{locales.NetworkBindHelp}</span>
                    <Input ref={settings.network.bind} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkMtu}:</p>
                    <span>{locales.NetworkMtuHelp}</span>
                    <Input ref={settings.network.mtu} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkMaxBandwidth}:</p>
                    <span>{locales.NetworkMaxBandwidthHelp}</span>
                    <Input ref={settings.network.max_bandwidth} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkLatency}:</p>
                    <span>{locales.NetworkLatencyHelp}</span>
                    <Input ref={settings.network.latency} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkTimeout}:</p>
                    <span>{locales.NetworkTimeoutHelp}</span>
                    <Input ref={settings.network.timeout} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkFc}:</p>
                    <span>{locales.NetworkFcHelp}</span>
                    <Input ref={settings.network.fc} disabled={disabled} />
                </div>
                <div className='item'>
                    <p>{locales.NetworkFec}:</p>
                    <span>{locales.NetworkFecHelp}</span>
                    <Input ref={settings.network.fec} disabled={disabled} />
                </div>
            </div>
        </>
    );
}
